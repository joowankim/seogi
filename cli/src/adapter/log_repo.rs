use rusqlite::Connection;

use super::error::AdapterError;
use super::mapper;
use crate::domain::log::{SystemEvent, ToolFailure, ToolUse};

/// `tool_uses` 테이블에 한 행을 INSERT한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError::Database`.
pub fn save_tool_use(conn: &Connection, tool_use: &ToolUse) -> Result<(), AdapterError> {
    conn.execute(
        "INSERT OR IGNORE INTO tool_uses (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            tool_use.id(),
            tool_use.session_id().as_str(),
            tool_use.project(),
            tool_use.project_path(),
            tool_use.tool_name(),
            tool_use.tool_input(),
            tool_use.duration().value(),
            tool_use.timestamp().value(),
        ),
    )?;
    Ok(())
}

/// 주어진 세션의 모든 도구 사용 기록을 조회한다.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn list_by_session(conn: &Connection, session_id: &str) -> Result<Vec<ToolUse>, AdapterError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp FROM tool_uses WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let rows = stmt
        .query_map([session_id], mapper::tool_use_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// `tool_failures` 테이블에 한 행을 INSERT한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError::Database`.
pub fn save_tool_failure(
    conn: &Connection,
    tool_failure: &ToolFailure,
) -> Result<(), AdapterError> {
    conn.execute(
        "INSERT OR IGNORE INTO tool_failures (id, session_id, project, project_path, tool_name, error, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            tool_failure.id(),
            tool_failure.session_id().as_str(),
            tool_failure.project(),
            tool_failure.project_path(),
            tool_failure.tool_name(),
            tool_failure.error(),
            tool_failure.timestamp().value(),
        ),
    )?;
    Ok(())
}

/// 주어진 세션의 모든 도구 실패 기록을 조회한다.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn list_failures_by_session(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<ToolFailure>, AdapterError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, project, project_path, tool_name, error, timestamp FROM tool_failures WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let rows = stmt
        .query_map([session_id], mapper::tool_failure_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// `system_events` 테이블에 한 행을 INSERT한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError::Database`.
pub fn save_system_event(conn: &Connection, event: &SystemEvent) -> Result<(), AdapterError> {
    conn.execute(
        "INSERT OR IGNORE INTO system_events (id, session_id, project, project_path, event_type, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            event.id(),
            event.session_id().as_str(),
            event.project(),
            event.project_path(),
            event.event_type(),
            event.content(),
            event.timestamp().value(),
        ),
    )?;
    Ok(())
}

/// 주어진 세션의 모든 시스템 이벤트를 조회한다.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn list_system_events_by_session(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<SystemEvent>, AdapterError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, project, project_path, event_type, content, timestamp FROM system_events WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let rows = stmt
        .query_map([session_id], mapper::system_event_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// 지정 기간 내 고유 `session_id` 목록을 조회한다.
///
/// `project`가 `Some`이면 해당 프로젝트만 필터. `None`이면 전체.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn list_session_ids_by_range(
    conn: &Connection,
    from_ts: i64,
    to_ts: i64,
    project: Option<&str>,
) -> Result<Vec<String>, AdapterError> {
    let rows = if let Some(p) = project {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT session_id FROM tool_uses WHERE timestamp >= ?1 AND timestamp <= ?2 AND project = ?3 ORDER BY session_id",
        )?;
        stmt.query_map((from_ts, to_ts, p), |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT session_id FROM tool_uses WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY session_id",
        )?;
        stmt.query_map((from_ts, to_ts), |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;
    use crate::domain::value::{Ms, SessionId, Timestamp};

    fn sample_tool_use() -> ToolUse {
        ToolUse::new(
            "abcdef1234567890abcdef1234567890".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            r#"{"command":"ls -la"}"#.to_string(),
            Ms::zero(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn test_save_tool_use_inserts_row() {
        let conn = db::initialize_in_memory().unwrap();
        let tu = sample_tool_use();

        save_tool_use(&conn, &tu).unwrap();

        let (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp): (
            String, String, String, String, String, String, i64, i64,
        ) = conn
            .query_row(
                "SELECT id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp FROM tool_uses LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
            )
            .unwrap();

        assert_eq!(id, "abcdef1234567890abcdef1234567890");
        assert_eq!(session_id, "sess-1");
        assert_eq!(project, "seogi");
        assert_eq!(project_path, "/Users/kim/projects/seogi");
        assert_eq!(tool_name, "Bash");
        assert_eq!(tool_input, r#"{"command":"ls -la"}"#);
        assert_eq!(duration_ms, 0);
        assert_eq!(timestamp, 1_713_000_000_000);
    }

    #[test]
    fn test_list_by_session_returns_saved() {
        let conn = db::initialize_in_memory().unwrap();
        let tu = sample_tool_use();

        save_tool_use(&conn, &tu).unwrap();

        let results = list_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], tu);
    }

    #[test]
    fn test_list_by_session_empty() {
        let conn = db::initialize_in_memory().unwrap();

        let results = list_by_session(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }

    fn sample_tool_failure() -> ToolFailure {
        ToolFailure::new(
            "abcdef1234567890abcdef1234567890".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            "Permission denied".to_string(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn test_save_tool_failure_inserts_row() {
        let conn = db::initialize_in_memory().unwrap();
        let tf = sample_tool_failure();

        save_tool_failure(&conn, &tf).unwrap();

        let (id, session_id, project, project_path, tool_name, error, timestamp): (
            String, String, String, String, String, String, i64,
        ) = conn
            .query_row(
                "SELECT id, session_id, project, project_path, tool_name, error, timestamp FROM tool_failures LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .unwrap();

        assert_eq!(id, "abcdef1234567890abcdef1234567890");
        assert_eq!(session_id, "sess-1");
        assert_eq!(project, "seogi");
        assert_eq!(project_path, "/Users/kim/projects/seogi");
        assert_eq!(tool_name, "Bash");
        assert_eq!(error, "Permission denied");
        assert_eq!(timestamp, 1_713_000_000_000);
    }

    #[test]
    fn test_list_failures_by_session_returns_saved() {
        let conn = db::initialize_in_memory().unwrap();
        let tf = sample_tool_failure();

        save_tool_failure(&conn, &tf).unwrap();

        let results = list_failures_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], tf);
    }

    #[test]
    fn test_list_failures_by_session_empty() {
        let conn = db::initialize_in_memory().unwrap();

        let results = list_failures_by_session(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }

    fn sample_system_event() -> SystemEvent {
        SystemEvent::new(
            "abcdef1234567890abcdef1234567890".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Notification".to_string(),
            "Permission required".to_string(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn test_save_system_event_inserts_row() {
        let conn = db::initialize_in_memory().unwrap();
        let se = sample_system_event();

        save_system_event(&conn, &se).unwrap();

        let (id, session_id, project, project_path, event_type, content, timestamp): (
            String, String, String, String, String, String, i64,
        ) = conn
            .query_row(
                "SELECT id, session_id, project, project_path, event_type, content, timestamp FROM system_events LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .unwrap();

        assert_eq!(id, "abcdef1234567890abcdef1234567890");
        assert_eq!(session_id, "sess-1");
        assert_eq!(project, "seogi");
        assert_eq!(project_path, "/Users/kim/projects/seogi");
        assert_eq!(event_type, "Notification");
        assert_eq!(content, "Permission required");
        assert_eq!(timestamp, 1_713_000_000_000);
    }

    #[test]
    fn test_list_system_events_by_session_returns_saved() {
        let conn = db::initialize_in_memory().unwrap();
        let se = sample_system_event();

        save_system_event(&conn, &se).unwrap();

        let results = list_system_events_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], se);
    }

    #[test]
    fn test_list_system_events_by_session_empty() {
        let conn = db::initialize_in_memory().unwrap();

        let results = list_system_events_by_session(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }

    fn make_tool_use_with_project(session_id: &str, project: &str, ts: i64) -> ToolUse {
        ToolUse::new(
            format!("id-{session_id}-{ts}"),
            SessionId::new(session_id),
            project.to_string(),
            format!("/{project}"),
            "Read".to_string(),
            "{}".to_string(),
            Ms::zero(),
            Timestamp::new(ts),
        )
    }

    #[test]
    fn test_list_session_ids_by_range() {
        let conn = db::initialize_in_memory().unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s1", "p1", 1000)).unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s2", "p1", 3000)).unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s3", "p2", 5000)).unwrap();

        let ids = list_session_ids_by_range(&conn, 1000, 5000, None).unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_list_session_ids_by_project() {
        let conn = db::initialize_in_memory().unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s1", "p1", 1000)).unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s2", "p2", 2000)).unwrap();

        let ids = list_session_ids_by_range(&conn, 1000, 5000, Some("p1")).unwrap();
        assert_eq!(ids, vec!["s1"]);
    }

    #[test]
    fn test_list_session_ids_out_of_range() {
        let conn = db::initialize_in_memory().unwrap();
        save_tool_use(&conn, &make_tool_use_with_project("s1", "p1", 1000)).unwrap();

        let ids = list_session_ids_by_range(&conn, 5000, 9000, None).unwrap();
        assert!(ids.is_empty());
    }
}
