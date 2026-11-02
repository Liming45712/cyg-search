use crate::model::{now, Area, Detail, Role};
use rusqlite::{params, Connection};
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
        let body = serde_json::to_string(role).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO roles(code,server_id,body) VALUES(?1,?2,?3)
            ON CONFLICT(code) DO UPDATE SET server_id=excluded.server_id,body=excluded.body",
            params![role.code, role.server_id, body],
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
    use rusqlite::OptionalExtension;
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
    #[test]
    fn updates_keep_details_and_prune_is_scoped() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE roles(code TEXT PRIMARY KEY,server_id TEXT,body TEXT,detail TEXT,detail_at INTEGER,detail_error TEXT,needs_refresh INTEGER NOT NULL DEFAULT 0)").unwrap();
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
