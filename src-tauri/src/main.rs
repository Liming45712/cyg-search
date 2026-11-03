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
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

struct AppState {
    path: PathBuf,
    client: reqwest::Client,
    running: AtomicBool,
    cancel: AtomicBool,
    servers: Mutex<HashSet<String>>,
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
    let mut roles = db::load_roles(&conn)?;
    roles.extend(db::load_roles(&db::connect(
        &state.path.with_file_name("spirits.db"),
    )?)?);
    Ok(Snapshot {
        roles,
        areas: db::load_areas(&conn)?,
        data_path: state.path.display().to_string(),
    })
}

#[tauri::command]
async fn refresh_areas(state: State<'_, AppState>) -> Result<Vec<Area>, String> {
    state.servers.lock().map_err(|e| e.to_string())?.clear();
    let areas = api::areas(&state.client).await?;
    db::save_areas(&db::connect(&state.path)?, &areas)?;
    *state.servers.lock().map_err(|e| e.to_string())? = areas
        .iter()
        .flat_map(|area| &area.server)
        .map(|server| server.id.clone())
        .collect();
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
    last_error: Option<String>,
}

#[tauri::command]
async fn crawl(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    server_id: String,
    force: bool,
    goods_type: Option<u8>,
) -> Result<Summary, String> {
    let goods_type = goods_type.unwrap_or(1);
    if ![1, 5].contains(&goods_type) {
        return Err("不支持的商品类型".into());
    }
    validate_server(
        &*state.servers.lock().map_err(|e| e.to_string())?,
        &server_id,
    )?;
    if state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("已有拉取任务在运行".into());
    }
    let _running = Running(&state.running);
    state.cancel.store(false, Ordering::SeqCst);
    let result = crawl_inner(&app, &state, &server_id, force, goods_type).await;
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
    goods_type: u8,
) -> Result<Summary, String> {
    // Separate caches keep a star-list refresh from pruning existing role data.
    let path = if goods_type == 5 {
        state.path.with_file_name("spirits.db")
    } else {
        state.path.clone()
    };
    let kind = if goods_type == 5 { "星灵" } else { "角色" };
    let cancelled = || state.cancel.load(Ordering::SeqCst);
    let stopped = |fetched, failed, removed| Summary {
        cancelled: true,
        fetched,
        failed,
        removed,
        last_error: None,
    };
    let mut keep = HashSet::new();
    if force {
        db::mark_refresh(&db::connect(&path)?, sid)?;
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
                match api::list(&state.client, sid, status, page_num, 20, goods_type).await {
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
            db::save_roles(&mut db::connect(&path)?, &page.roles)?;
            progress(
                app,
                "list",
                page_num,
                page.pages,
                format!(
                    "已匹配 {} 个{kind}，仅变化或缺失的详情进入更新队列",
                    keep.len()
                ),
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
    let mut unconfirmed = 0;
    let removed = if cleanup_safe && !keep.is_empty() {
        db::prune(&mut db::connect(&path)?, sid, &keep)?
    } else {
        // Unstable pagination cannot prove absence; confirm only missing cached goods individually.
        let missing: Vec<Role> = db::load_roles(&db::connect(&path)?)?
            .into_iter()
            .filter(|role| role.server_id == sid && !keep.contains(&role.code))
            .collect();
        let mut removed = 0;
        for (index, role) in missing.iter().enumerate() {
            if cancelled() {
                return Ok(stopped(0, 0, removed));
            }
            progress(
                app,
                "list",
                index + 1,
                missing.len(),
                format!(
                    "核查列表中缺失的{kind}销售状态：{} / {}",
                    index + 1,
                    missing.len()
                ),
            );
            match api::unavailable(&state.client, role).await {
                Ok(true) => {
                    if cancelled() {
                        return Ok(stopped(0, 0, removed));
                    }
                    removed += db::remove_unavailable(&db::connect(&path)?, sid, &role.code)?;
                }
                Ok(false) | Err(_) => unconfirmed += 1,
            }
            if index + 1 < missing.len() {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        removed
    };
    let pending: Vec<Role> = db::load_roles(&db::connect(&path)?)?
        .into_iter()
        .filter(|r| {
            r.server_id == sid
                && keep.contains(&r.code)
                && (r.needs_refresh || r.detail.is_none() || r.detail_error.is_some())
        })
        .collect();
    let mut last_error = None;
    let mut failed = 0;
    let mut fetched = 0;
    let mut tasks = tokio::task::JoinSet::new();
    let mut next = 0;
    progress(
        app,
        "detail",
        0,
        pending.len(),
        format!(
            "列表匹配完成：需更新 {} 个详情，复用 {} 个缓存（最多 5 个并发）",
            pending.len(),
            keep.len().saturating_sub(pending.len())
        ),
    );
    loop {
        while tasks.len() < 5 && next < pending.len() && !cancelled() {
            let client = state.client.clone();
            let code = pending[next].code.clone();
            tasks.spawn(async move {
                let result = api::detail(&client, &code, goods_type).await;
                (code, result)
            });
            next += 1;
        }
        debug_assert!(tasks.len() <= 5);
        debug_assert_eq!(fetched + failed + tasks.len(), next);
        let Some(result) = tasks.join_next().await else {
            break;
        };
        let (code, result) = result.map_err(|e| format!("详情解析任务失败：{e}"))?;
        match result {
            Ok(detail) => {
                db::save_detail(&db::connect(&path)?, &code, &detail)?;
                fetched += 1;
            }
            Err(e) => {
                db::detail_error(&db::connect(&path)?, &code, &e)?;
                last_error = Some(format!("商品 {code}：{e}"));
                failed += 1;
                progress(
                    app,
                    "detail-error",
                    fetched + failed,
                    pending.len(),
                    format!(
                        "详情获取失败，旧缓存已保留。原因：{}",
                        last_error.as_deref().unwrap_or_default()
                    ),
                );
            }
        }
        progress(
            app,
            "detail",
            fetched + failed,
            pending.len(),
            format!("已解析 {fetched} 个详情，失败 {failed} 个"),
        );
    }
    if cancelled() {
        return Ok(stopped(fetched, failed, removed));
    }
    debug_assert_eq!(fetched + failed, pending.len());
    let cleanup_note = if !cleanup_safe || keep.is_empty() {
        format!("；已核查缺失商品，保留 {unconfirmed} 个未确认下架的缓存")
    } else {
        String::new()
    };
    let message = format!(
        "更新完成：{} 个{kind}，更新详情 {fetched} 个，失败 {failed} 个，复用 {} 个缓存，清理 {removed} 个{}",
        keep.len(),
        keep.len().saturating_sub(pending.len()),
        cleanup_note
    );
    progress(app, "done", pending.len(), pending.len(), message);
    Ok(Summary {
        cancelled: false,
        fetched,
        failed,
        removed,
        last_error,
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
                servers: Mutex::new(HashSet::new()),
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

fn validate_server(servers: &HashSet<String>, server: &str) -> Result<(), String> {
    if servers.is_empty() {
        return Err("尚未成功获取官网区服，请先刷新区服后再拉取数据".into());
    }
    if !model::valid_id(server) || !servers.contains(server) {
        return Err("所选区服不在官网区服列表中，请刷新区服并重新选择".into());
    }
    Ok(())
}

#[cfg(test)]
mod crawl_tests {
    use super::*;

    #[test]
    fn crawl_requires_a_successfully_loaded_official_server() {
        assert!(validate_server(&HashSet::new(), "104").is_err());
        let servers = HashSet::from(["104".to_string()]);
        assert!(validate_server(&servers, "104").is_ok());
        for server in ["", "615", "104 OR 1=1"] {
            assert!(validate_server(&servers, server).is_err());
        }
    }
}
