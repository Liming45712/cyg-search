#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod api;
mod db;
mod model;
mod parser;

use model::{Area, Role};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

struct AppState {
    path: PathBuf,
    client: reqwest::Client,
    running: AtomicBool,
    cancel: AtomicBool,
}
struct Running<'a>(&'a AtomicBool);
impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[derive(Clone, Serialize)]
struct Progress {
    stage: String,
    done: usize,
    total: usize,
    message: String,
}
fn progress(
    app: &tauri::AppHandle,
    stage: &str,
    done: usize,
    total: usize,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "crawl-progress",
        Progress {
            stage: stage.into(),
            done,
            total,
            message: message.into(),
        },
    );
}

#[derive(Serialize)]
struct Snapshot {
    roles: Vec<Role>,
    areas: Vec<Area>,
    data_path: String,
}
#[tauri::command]
fn snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let conn = db::connect(&state.path)?;
    Ok(Snapshot {
        roles: db::load_roles(&conn)?,
        areas: db::load_areas(&conn)?,
        data_path: state.path.display().to_string(),
    })
}

#[tauri::command]
async fn refresh_areas(state: State<'_, AppState>) -> Result<Vec<Area>, String> {
    let areas = api::areas(&state.client).await?;
    db::save_areas(&db::connect(&state.path)?, &areas)?;
    Ok(areas)
}

#[tauri::command]
fn cancel_crawl(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[derive(Serialize)]
struct Summary {
    cancelled: bool,
    fetched: usize,
    failed: usize,
    removed: usize,
}

#[tauri::command]
async fn crawl(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    server_id: String,
    force: bool,
) -> Result<Summary, String> {
    if !model::valid_id(&server_id) {
        return Err("请选择有效区服".into());
    }
    if state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("已有拉取任务在运行".into());
    }
    let _running = Running(&state.running);
    state.cancel.store(false, Ordering::SeqCst);
    let result = crawl_inner(&app, &state, &server_id, force).await;
    if let Err(ref error) = result {
        progress(&app, "error", 0, 0, error.clone());
    }
    result
}

async fn crawl_inner(
    app: &tauri::AppHandle,
    state: &AppState,
    sid: &str,
    force: bool,
) -> Result<Summary, String> {
    let cancelled = || state.cancel.load(Ordering::SeqCst);
    let stopped = |fetched, failed, removed| Summary {
        cancelled: true,
        fetched,
        failed,
        removed,
    };
    let mut keep = HashSet::new();
    if force {
        db::mark_refresh(&db::connect(&state.path)?, sid)?;
    }
    let mut cleanup_safe = true;
    for status in [2, 1] {
        let mut page_num = 1;
        let mut group_codes = HashSet::new();
        loop {
            if cancelled() {
                return Ok(stopped(0, 0, 0));
            }
            progress(
                app,
                "list",
                page_num,
                0,
                format!(
                    "正在获取{}列表，第 {page_num} 页",
                    if status == 2 { "在售" } else { "公示" }
                ),
            );
            let mut page = None;
            let mut last_error = String::new();
            for attempt in 0..3 {
                if cancelled() {
                    return Ok(stopped(0, 0, 0));
                }
                match api::list(&state.client, sid, status, page_num, 20).await {
                    Ok(value) => {
                        page = Some(value);
                        break;
                    }
                    Err(e) => {
                        last_error = e;
                        if attempt < 2 {
                            tokio::time::sleep(Duration::from_secs(attempt + 1)).await;
                        }
                    }
                }
            }
            let page = page.ok_or_else(|| {
                format!("列表第 {page_num} 页失败：{last_error}。未清理旧缓存，可重新拉取。")
            })?;
            if cancelled() {
                return Ok(stopped(0, 0, 0));
            }
            if page.roles.is_empty() && page_num < page.pages {
                return Err("列表中途为空，已停止，旧缓存未清理".into());
            }
            for role in &page.roles {
                if !group_codes.insert(role.code.clone()) {
                    cleanup_safe = false;
                }
                keep.insert(role.code.clone());
            }
            db::save_roles(&mut db::connect(&state.path)?, &page.roles)?;
            progress(
                app,
                "list",
                page_num,
                page.pages,
                format!("已缓存 {} 个角色；点击暂停会保留进度", keep.len()),
            );
            if page_num >= page.pages {
                if group_codes.len() != page.count {
                    cleanup_safe = false;
                }
                break;
            }
            page_num += 1;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    if cancelled() {
        return Ok(stopped(0, 0, 0));
    }
    let removed = if cleanup_safe {
        db::prune(&mut db::connect(&state.path)?, sid, &keep)?
    } else {
        0
    };
    let pending: Vec<Role> = db::load_roles(&db::connect(&state.path)?)?
        .into_iter()
        .filter(|r| {
            r.server_id == sid
                && keep.contains(&r.code)
                && (r.needs_refresh || r.detail.is_none() || r.detail_error.is_some())
        })
        .collect();
    let mut failed = 0;
    let mut fetched = 0;
    for (index, role) in pending.iter().enumerate() {
        if cancelled() {
            return Ok(stopped(fetched, failed, removed));
        }
        progress(
            app,
            "detail",
            index,
            pending.len(),
            format!("正在解析详情 · {}（失败 {failed}）", role.name),
        );
        match api::detail(&state.client, &role.code).await {
            Ok(detail) => {
                db::save_detail(&db::connect(&state.path)?, &role.code, &detail)?;
                fetched += 1;
            }
            Err(e) => {
                db::detail_error(&db::connect(&state.path)?, &role.code, &e)?;
                failed += 1;
            }
        }
        progress(
            app,
            "detail",
            index + 1,
            pending.len(),
            format!("已解析 {fetched} 个详情，失败 {failed} 个"),
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let message = format!(
        "更新完成：{} 个角色，新增详情 {fetched} 个，失败 {failed} 个，清理 {removed} 个{}",
        keep.len(),
        if !cleanup_safe {
            "；列表在更新中发生变化，已跳过清理"
        } else if keep.is_empty() {
            "；空列表保留旧缓存"
        } else {
            ""
        }
    );
    progress(app, "done", pending.len(), pending.len(), message);
    Ok(Summary {
        cancelled: false,
        fetched,
        failed,
        removed,
    })
}

#[tauri::command]
fn open_role(app: tauri::AppHandle, code: String) -> Result<(), String> {
    if !model::valid_id(&code) {
        return Err("商品编号格式错误".into());
    }
    app.opener()
        .open_url(
            format!("{}/details?goodsCode={code}", api::BASE),
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("cyg.db");
            db::connect(&path).map_err(std::io::Error::other)?;
            app.manage(AppState {
                path,
                client: api::client().map_err(std::io::Error::other)?,
                running: AtomicBool::new(false),
                cancel: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            refresh_areas,
            crawl,
            cancel_crawl,
            open_role
        ])
        .run(tauri::generate_context!())
        .expect("无法启动畅易阁角色筛选");
}
