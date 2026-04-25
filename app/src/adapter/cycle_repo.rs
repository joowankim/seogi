use chrono::Utc;
use rusqlite::Connection;

use super::mapper::cycle_from_row;
use crate::domain::cycle::{self, Cycle};

const CYCLE_COLUMNS: &str = "id, workspace_id, name, start_date, end_date, created_at, updated_at";

/// Cycle을 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, cycle: &Cycle) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO cycles (id, workspace_id, name, start_date, end_date, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            cycle.id(),
            cycle.workspace_id(),
            cycle.name(),
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

/// 특정 워크스페이스의 active Cycle을 조회한다.
///
/// `today` 기준 `start_date <= today <= end_date`인 Cycle을 반환한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_active_by_workspace(
    conn: &Connection,
    workspace_id: &str,
    today: &str,
) -> rusqlite::Result<Option<Cycle>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CYCLE_COLUMNS} FROM cycles WHERE workspace_id = ?1 AND start_date <= ?2 AND end_date >= ?2 LIMIT 1"
    ))?;
    let mut rows = stmt.query_map([workspace_id, today], cycle_from_row)?;
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

/// 같은 워크스페이스 내에서 날짜 구간이 겹치는 Cycle을 조회한다.
///
/// `exclude_id`가 `Some`이면 해당 ID의 Cycle은 제외한다 (update 시 자기 자신 제외).
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_by_workspace_overlapping(
    conn: &Connection,
    workspace_id: &str,
    start_date: &str,
    end_date: &str,
    exclude_id: Option<&str>,
) -> rusqlite::Result<Vec<Cycle>> {
    let mut sql = format!(
        "SELECT {CYCLE_COLUMNS} FROM cycles WHERE workspace_id = ?1 AND start_date <= ?3 AND end_date >= ?2"
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(workspace_id.to_string()),
        Box::new(start_date.to_string()),
        Box::new(end_date.to_string()),
    ];

    if let Some(eid) = exclude_id {
        sql.push_str(" AND id != ?4");
        params.push(Box::new(eid.to_string()));
    }

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), cycle_from_row)?;
    rows.collect()
}

/// Cycle 조회 결과에 워크스페이스 이름과 파생 status를 포함한 구조체.
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
    let mut sql = "SELECT c.id, w.name AS workspace_name, c.name, c.start_date, c.end_date, c.created_at, c.updated_at FROM cycles c JOIN workspaces w ON c.workspace_id = w.id".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ws) = workspace_name {
        sql.push_str(" WHERE w.name = ?");
        params.push(Box::new(ws.to_string()));
    }

    sql.push_str(" ORDER BY c.created_at DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(AsRef::as_ref).collect();

    let mut stmt = conn.prepare(&sql)?;
    let today = Utc::now().date_naive();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let start_date: String = row.get("start_date")?;
        let end_date: String = row.get("end_date")?;
        let status = cycle::derive_status(&start_date, &end_date, today)
            .as_str()
            .to_string();
        Ok(CycleListRow {
            id: row.get("id")?,
            workspace_name: row.get("workspace_name")?,
            name: row.get("name")?,
            status,
            start_date,
            end_date,
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

    // Q12: save 후 DB에 status 컬럼 없이 저장 확인
    #[test]
    fn test_save_without_status() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let found = find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.id(), cycle.id());
        assert_eq!(found.name(), "Sprint 1");
        assert_eq!(found.start_date(), "2026-05-01");
        assert_eq!(found.end_date(), "2026-05-14");
    }

    // Q13: find_by_id status 없이 Cycle 복원
    #[test]
    fn test_find_by_id_without_status() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let cycle = sample_cycle(ws.id());
        save(&conn, &cycle).unwrap();

        let found = find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.workspace_id(), ws.id());
    }

    // Q14: list_by_workspace_overlapping 겹치는 Cycle 반환
    #[test]
    fn test_list_overlapping_found() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let c = Cycle::new(ws.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        save(&conn, &c).unwrap();

        let result =
            list_by_workspace_overlapping(&conn, ws.id(), "2026-05-10", "2026-05-20", None)
                .unwrap();
        assert_eq!(result.len(), 1);
    }

    // Q15: list_by_workspace_overlapping 겹치지 않으면 빈 Vec
    #[test]
    fn test_list_overlapping_not_found() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let c = Cycle::new(ws.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        save(&conn, &c).unwrap();

        let result =
            list_by_workspace_overlapping(&conn, ws.id(), "2026-05-15", "2026-05-28", None)
                .unwrap();
        assert!(result.is_empty());
    }

    // Q16: list_by_workspace_overlapping 자기 자신 제외
    #[test]
    fn test_list_overlapping_exclude_self() {
        let conn = initialize_in_memory().unwrap();
        let ws = sample_workspace(&conn);
        let c = Cycle::new(ws.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        save(&conn, &c).unwrap();

        let result =
            list_by_workspace_overlapping(&conn, ws.id(), "2026-05-01", "2026-05-14", Some(c.id()))
                .unwrap();
        assert!(result.is_empty());
    }

    // 기존 테스트 유지
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

    #[test]
    fn test_find_by_id_not_found() {
        let conn = initialize_in_memory().unwrap();
        let found = find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

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
