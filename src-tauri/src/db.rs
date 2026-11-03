use crate::model::{now, Area, Detail, Role};
use rusqlite::{params, Connection, OptionalExtension};
use std::{collections::HashSet, path::Path};

pub fn connect(path: &Path) -> Result<Connection, String> {
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS roles(code TEXT PRIMARY KEY, server_id TEXT NOT NULL, body TEXT NOT NULL, detail TEXT, detail_at INTEGER, detail_error TEXT, needs_refresh INTEGER NOT NULL DEFAULT 0);
        CREATE INDEX IF NOT EXISTS roles_server ON roles(server_id);
        CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);")
        .map_err(|e| e.to_string())?;
    Ok(db)
}

pub fn save_roles(db: &mut Connection, roles: &[Role]) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for role in roles {
        let previous: Option<String> = tx
            .query_row(
                "SELECT body FROM roles WHERE code=?1",
                [&role.code],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let previous = previous
            .map(|body| serde_json::from_str::<Role>(&body))
            .transpose()
            .map_err(|e| format!("缓存读取失败：{e}"))?;
        let changed = previous
            .as_ref()
            .is_some_and(|old| role.listing_changed(old));
        let body = serde_json::to_string(role).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO roles(code,server_id,body) VALUES(?1,?2,?3)
            ON CONFLICT(code) DO UPDATE SET server_id=excluded.server_id,body=excluded.body,
            needs_refresh=CASE WHEN ?4 THEN 1 ELSE roles.needs_refresh END",
            params![role.code, role.server_id, body, changed],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn load_roles(db: &Connection) -> Result<Vec<Role>, String> {
    let mut stmt = db
        .prepare("SELECT body,detail,detail_at,detail_error,needs_refresh FROM roles")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<u64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.map(|row| {
        let (body, detail, at, error, needs_refresh) = row.map_err(|e| e.to_string())?;
        let mut role: Role =
            serde_json::from_str(&body).map_err(|e| format!("缓存读取失败：{e}"))?;
        role.detail = detail
            .map(|d| serde_json::from_str(&d))
            .transpose()
            .map_err(|e| e.to_string())?;
        role.detail_at = at;
        role.detail_error = error;
        role.needs_refresh = needs_refresh;
        Ok(role)
    })
    .collect()
}

pub fn save_detail(db: &Connection, code: &str, detail: &Detail) -> Result<(), String> {
    let body = serde_json::to_string(detail).map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE roles SET detail=?1,detail_at=?2,detail_error=NULL,needs_refresh=0 WHERE code=?3",
        params![body, now(), code],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn detail_error(db: &Connection, code: &str, error: &str) -> Result<(), String> {
    db.execute(
        "UPDATE roles SET detail_error=?1 WHERE code=?2",
        params![error, code],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_refresh(db: &Connection, server: &str) -> Result<(), String> {
    db.execute(
        "UPDATE roles SET needs_refresh=1 WHERE server_id=?1",
        [server],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_unavailable(db: &Connection, server: &str, code: &str) -> Result<usize, String> {
    db.execute(
        "DELETE FROM roles WHERE server_id=?1 AND code=?2",
        params![server, code],
    )
    .map_err(|e| e.to_string())
}

pub fn prune(db: &mut Connection, server: &str, keep: &HashSet<String>) -> Result<usize, String> {
    // An empty response is ambiguous; retain old data until a nonempty complete crawl.
    if keep.is_empty() {
        return Ok(0);
    }
    let tx = db.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS keep_codes(code TEXT PRIMARY KEY); DELETE FROM keep_codes;").map_err(|e| e.to_string())?;
    for code in keep {
        tx.execute("INSERT INTO keep_codes VALUES(?1)", [code])
            .map_err(|e| e.to_string())?;
    }
    let removed = tx
        .execute(
            "DELETE FROM roles WHERE server_id=?1 AND code NOT IN (SELECT code FROM keep_codes)",
            [server],
        )
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(removed)
}

pub fn save_areas(db: &Connection, areas: &[Area]) -> Result<(), String> {
    db.execute(
        "INSERT OR REPLACE INTO meta(key,value) VALUES('areas',?1)",
        [serde_json::to_string(areas).map_err(|e| e.to_string())?],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_areas(db: &Connection) -> Result<Vec<Area>, String> {
    let value: Option<String> = db
        .query_row("SELECT value FROM meta WHERE key='areas'", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
        .unwrap_or(Ok(vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn role(code: &str, sid: &str) -> Role {
        serde_json::from_value(serde_json::json!({"code":code,"server_id":sid,"name":"角色","title":"","job":"剑客","price":1,"status":2,"listed_at":"","offline_at":""})).unwrap()
    }
    fn memory() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE roles(code TEXT PRIMARY KEY,server_id TEXT,body TEXT,detail TEXT,detail_at INTEGER,detail_error TEXT,needs_refresh INTEGER NOT NULL DEFAULT 0)").unwrap();
        db
    }

    #[test]
    fn unstable_listing_removes_only_individually_confirmed_unavailable_goods() {
        let mut db = memory();
        let sold = role("1622610310017090018", "104");
        save_roles(&mut db, &[sold.clone(), role("2", "104"), role("3", "615")]).unwrap();
        let html = include_str!("../tests/sold.html");
        // Repeated pagination leaves missing goods ambiguous; only this role has explicit sale evidence.
        assert!(
            crate::parser::unavailable(html, &sold.code, &sold.server_id, sold.goods_type).unwrap()
        );
        assert_eq!(remove_unavailable(&db, "615", &sold.code).unwrap(), 0);
        assert_eq!(remove_unavailable(&db, "104", &sold.code).unwrap(), 1);
        let remaining = load_roles(&db).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|r| r.code == "2"));
        assert!(remaining.iter().any(|r| r.code == "3"));
        assert_eq!(remove_unavailable(&db, "104", &sold.code).unwrap(), 0);
    }

    #[test]
    fn legacy_cache_learns_list_attributes_without_refreshing_all_details() {
        let mut db = memory();
        let legacy = role("1", "104");
        save_roles(&mut db, &[legacy.clone()]).unwrap();
        let detail = crate::parser::parse(include_str!("../tests/detail.html")).unwrap();
        save_detail(&db, "1", &detail).unwrap();
        let mut current = legacy;
        current.listing_updated_at = "2026-11-03 09:12:00".into();
        current.list_attrs.insert(
            "等级".into(),
            crate::model::Attr {
                raw: "159级".into(),
                num: Some(159.0),
            },
        );
        save_roles(&mut db, &[current.clone()]).unwrap();
        assert!(
            !load_roles(&db).unwrap()[0].needs_refresh,
            "Newly available metadata must establish a baseline, not refresh every legacy detail"
        );
        current.list_attrs.get_mut("等级").unwrap().raw = "160级".into();
        current.list_attrs.get_mut("等级").unwrap().num = Some(160.0);
        save_roles(&mut db, &[current]).unwrap();
        assert!(load_roles(&db).unwrap()[0].needs_refresh);
    }

    #[test]
    fn identical_lists_skip_details_and_ignore_local_check_times() {
        let mut db = memory();
        let mut item = role("1", "104");
        save_roles(&mut db, &[item.clone()]).unwrap();
        let detail = crate::parser::parse(include_str!("../tests/detail.html")).unwrap();
        save_detail(&db, "1", &detail).unwrap();
        let previous_at = load_roles(&db).unwrap()[0].detail_at;
        item.checked_at += 100;
        // An old cache learns the timestamp without forcing every old detail to refresh.
        item.listing_updated_at = "2026-11-03 01:00:00".into();
        for _ in 0..2 {
            save_roles(&mut db, &[item.clone()]).unwrap();
        }
        let cached = load_roles(&db).unwrap().remove(0);
        assert!(!cached.needs_refresh);
        assert_eq!(cached.detail_at, previous_at);
        assert_eq!(cached.listing_updated_at, item.listing_updated_at);
        assert_eq!(cached.detail.unwrap().attrs["等级"].num, Some(159.0));
    }

    #[test]
    fn changed_listings_refresh_once_and_preserve_old_details() {
        let mut original = role("1", "104");
        original.listing_updated_at = "2026-11-03 01:00:00".into();
        original.list_attrs.insert(
            "等级".into(),
            crate::model::Attr {
                raw: "159级".into(),
                num: Some(159.0),
            },
        );
        let detail = crate::parser::parse(include_str!("../tests/detail.html")).unwrap();
        let mut variants = vec![original.clone(); 10];
        variants[0].price = Some(2.0);
        variants[1].status = 1;
        variants[2].name = "新名称".into();
        variants[3].title = "新标题".into();
        variants[4].job = "术士".into();
        variants[5].listed_at = "2026-11-03 02:00:00".into();
        variants[6].offline_at = "2026-11-17 02:00:00".into();
        variants[7].listing_updated_at = "2026-11-03 02:00:00".into();
        variants[8].list_attrs.insert(
            "强化".into(),
            crate::model::Attr {
                raw: "8".into(),
                num: Some(8.0),
            },
        );
        variants[9].goods_type = 5;
        for changed in variants {
            let mut db = memory();
            save_roles(&mut db, &[original.clone(), role("2", "104")]).unwrap();
            save_detail(&db, "1", &detail).unwrap();
            save_roles(&mut db, &[changed.clone()]).unwrap();
            detail_error(&db, "1", "网络超时").unwrap();
            // Saving the same list after failure/pause must not lose pending work.
            save_roles(&mut db, &[changed.clone()]).unwrap();
            let cached = load_roles(&db).unwrap();
            let first = cached.iter().find(|item| item.code == "1").unwrap();
            assert!(first.needs_refresh);
            assert_eq!(first.detail_error.as_deref(), Some("网络超时"));
            assert_eq!(
                first.detail.as_ref().unwrap().attrs["等级"].num,
                Some(159.0)
            );
            assert!(
                !cached
                    .iter()
                    .find(|item| item.code == "2")
                    .unwrap()
                    .needs_refresh
            );
            save_detail(&db, "1", &detail).unwrap();
            save_roles(&mut db, &[changed]).unwrap();
            let cached = load_roles(&db).unwrap();
            let first = cached.iter().find(|item| item.code == "1").unwrap();
            assert!(!first.needs_refresh);
            assert!(first.detail_error.is_none());
        }
    }

    #[test]
    fn forced_refresh_survives_matching_lists_and_corrupt_cache_rolls_back() {
        let mut db = memory();
        let item = role("1", "104");
        save_roles(&mut db, &[item.clone()]).unwrap();
        mark_refresh(&db, "104").unwrap();
        save_roles(&mut db, &[item.clone()]).unwrap();
        assert!(load_roles(&db).unwrap()[0].needs_refresh);
        db.execute("UPDATE roles SET body='broken' WHERE code='1'", [])
            .unwrap();
        assert!(save_roles(&mut db, &[role("2", "104"), item]).is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM roles", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn updates_keep_details_and_prune_is_scoped() {
        let mut db = memory();
        save_roles(
            &mut db,
            &[role("1", "104"), role("2", "104"), role("3", "615")],
        )
        .unwrap();
        let detail = crate::parser::parse(include_str!("../tests/detail.html")).unwrap();
        save_detail(&db, "1", &detail).unwrap();
        save_roles(&mut db, &[role("1", "104")]).unwrap();
        detail_error(&db, "1", "超时").unwrap();
        mark_refresh(&db, "104").unwrap();
        assert!(
            load_roles(&db)
                .unwrap()
                .iter()
                .find(|r| r.code == "1")
                .unwrap()
                .needs_refresh
        );
        assert!(
            !load_roles(&db)
                .unwrap()
                .iter()
                .find(|r| r.code == "3")
                .unwrap()
                .needs_refresh
        );
        save_detail(&db, "1", &detail).unwrap();
        assert!(
            !load_roles(&db)
                .unwrap()
                .iter()
                .find(|r| r.code == "1")
                .unwrap()
                .needs_refresh
        );
        assert!(load_roles(&db)
            .unwrap()
            .iter()
            .find(|r| r.code == "1")
            .unwrap()
            .detail
            .is_some());
        assert_eq!(prune(&mut db, "104", &HashSet::new()).unwrap(), 0);
        assert_eq!(
            prune(&mut db, "104", &HashSet::from(["1".into()])).unwrap(),
            1
        );
        let roles = load_roles(&db).unwrap();
        assert_eq!(roles.len(), 2);
        assert!(roles.iter().any(|r| r.code == "3"));
    }
}
