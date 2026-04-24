use chrono::Utc;
use rusqlite::Connection;

use super::mapper::cycle_from_row;
use crate::domain::cycle::Cycle;

const CYCLE_COLUMNS: &str =
    "id, workspace_id, name, status, start_date, end_date, created_at, updated_at";

/// Cycle을 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, cycle: &Cycle) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO cycles (id, workspace_id, name, status, start_date, end_date, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            cycle.id(),
            cycle.workspace_id(),
            cycle.name(),
            cycle.status().as_str(),
            cycle.start_date(),
            cycle.end_date(),
            cycle.created_at().to_rfc3339(),
            cycle.updated_at().to_rfc3339(),
        ),
    )?;
    Ok(())
}

/// 전체 Cycle을 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<Cycle>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CYCLE_COLUMNS} FROM cycles ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map([], cycle_from_row)?;
    rows.collect()
}

/// 특정 워크스페이스의 Cycle을 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_by_workspace(conn: &Connection, workspace_id: &str) -> rusqlite::Result<Vec<Cycle>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CYCLE_COLUMNS} FROM cycles WHERE workspace_id = ?1 ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map([workspace_id], cycle_from_row)?;
    rows.collect()
}

/// ID로 Cycle을 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<Cycle>> {
    let mut stmt = conn.prepare(&format!("SELECT {CYCLE_COLUMNS} FROM cycles WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], cycle_from_row)?;
    rows.next().transpose()
}

/// Cycle을 업데이트한다.
///
/// name, `start_date`, `end_date` 중 `Some`인 필드만 업데이트한다.
/// `updated_at`은 현재 시각으로 갱신된다.
///
/// # Errors
///
/// UPDATE 실패 시 `rusqlite::Error`.
pub fn update(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> rusqlite::Result<usize> {
    let mut sets = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = name {
        sets.push("name = ?");
        params.push(Box::new(n.to_string()));
    }
    if let Some(s) = start_date {
        sets.push("start_date = ?");
        params.push(Box::new(s.to_string()));
    }
    if let Some(e) = end_date {
        sets.push("end_date = ?");
        params.push(Box::new(e.to_string()));
    }

    sets.push("updated_at = ?");
    params.push(Box::new(Utc::now().to_rfc3339()));

    params.push(Box::new(id.to_string()));

    let set_clause = sets.join(", ");
    let sql = format!("UPDATE cycles SET {set_clause} WHERE id = ?");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(AsRef::as_ref).collect();
    conn.execute(&sql, param_refs.as_slice())
}

/// Cycle 조회 결과에 워크스페이스 이름을 포함한 구조체.
#[derive(Debug, serde::Serialize)]
pub struct CycleListRow {
    pub id: String,
    pub workspace_name: String,
    pub name: String,
    pub status: String,
    pub start_date: String,
    pub end_date: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Cycle 목록을 워크스페이스 이름과 함께 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_detailed(
    conn: &Connection,
    workspace_name: Option<&str>,
) -> rusqlite::Result<Vec<CycleListRow>> {
    let mut sql = "SELECT c.id, w.name AS workspace_name, c.name, c.status, c.start_date, c.end_date, c.created_at, c.updated_at FROM cycles c JOIN workspaces w ON c.workspace_id = w.id".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ws) = workspace_name {
        sql.push_str(" WHERE w.name = ?");
        params.push(Box::new(ws.to_string()));
    }

    sql.push_str(" ORDER BY c.created_at DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(AsRef::as_ref).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(CycleListRow {
            id: row.get("id")?,
            workspace_name: row.get("workspace_name")?,
            name: row.get("name")?,
            status: row.get("status")?,
            start_date: row.get("start_date")?,
            end_date: row.get("end_date")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::workspace_repo;
    use crate::domain::cycle::CycleStatus;
    use crate::domain::workspace::Workspace;
    use crate::domain::workspace::WorkspacePrefix;

    fn sample_workspace(conn: &Connection) -> Workspace {
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let ws = Workspace::new("Seogi", &prefix, "test", Utc::now()).unwrap();
        workspace_repo::save(conn, &ws).unwrap();
        ws
    }

    fn sample_cycle(workspace_id: &str) -> Cycle {
        Cycle::new(
            workspace_id,
            "Sprint 1",
            "2026-05-01",
            "2026-05-14",
            Utc::now(),
        )
        .unwrap()
    }

    // Q11: save 후 DB에서 조회 시 필드 일치
    #[test]
    fn test_save_and_find() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let found = find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.id(), cycle.id());
        assert_eq!(found.workspace_id(), ws.id());
        assert_eq!(found.name(), "Sprint 1");
        assert_eq!(found.status(), CycleStatus::Planned);
        assert_eq!(found.start_date(), "2026-05-01");
        assert_eq!(found.end_date(), "2026-05-14");
    }

    // Q12: list_by_workspace 해당 워크스페이스만 반환, created_at DESC 정렬
    #[test]
    fn test_list_by_workspace() {
        let conn = initialize_in_memory().unwrap();
        let ws1 = sample_workspace(&conn);

        let prefix2 = WorkspacePrefix::new("OTH").unwrap();
        let ws2 = Workspace::new("Other", &prefix2, "other", Utc::now()).unwrap();
        workspace_repo::save(&conn, &ws2).unwrap();

        let c1 = Cycle::new(ws1.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        save(&conn, &c1).unwrap();
        let c2 = Cycle::new(ws2.id(), "Sprint A", "2026-06-01", "2026-06-14", Utc::now()).unwrap();
        save(&conn, &c2).unwrap();

        let result = list_by_workspace(&conn, ws1.id()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "Sprint 1");
    }

    // Q13: list_all 전체 반환
    #[test]
    fn test_list_all() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);

        let c1 = Cycle::new(ws.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        let c2 = Cycle::new(ws.id(), "Sprint 2", "2026-05-15", "2026-05-28", Utc::now()).unwrap();
        save(&conn, &c1).unwrap();
        save(&conn, &c2).unwrap();

        let result = list_all(&conn).unwrap();
        assert_eq!(result.len(), 2);
    }

    // Q14: find_by_id 존재하는 ID → Some(Cycle)
    #[test]
    fn test_find_by_id_found() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let found = find_by_id(&conn, cycle.id()).unwrap();
        assert!(found.is_some());
    }

    // Q15: find_by_id 없는 ID → None
    #[test]
    fn test_find_by_id_not_found() {
        let conn = initialize_in_memory().unwrap();
        let found = find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

    // Q16: update 이름 변경 시 DB 반영
    #[test]
    fn test_update_name() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let rows = update(&conn, cycle.id(), Some("Updated"), None, None).unwrap();
        assert_eq!(rows, 1);

        let found = find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.name(), "Updated");
    }

    // Q17: update 시작일/종료일 변경 시 DB 반영
    #[test]
    fn test_update_dates() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let rows = update(
            &conn,
            cycle.id(),
            None,
            Some("2026-06-01"),
            Some("2026-06-14"),
        )
        .unwrap();
        assert_eq!(rows, 1);

        let found = find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.start_date(), "2026-06-01");
        assert_eq!(found.end_date(), "2026-06-14");
    }
}
