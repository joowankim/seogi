use rusqlite::Connection;

use super::error::AdapterError;
use super::mapper;
use crate::domain::log::ToolUse;

/// `tool_uses` 테이블에 한 행을 INSERT한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError::Database`.
pub fn save_tool_use(conn: &Connection, tool_use: &ToolUse) -> Result<(), AdapterError> {
    conn.execute(
        "INSERT INTO tool_uses (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            tool_use.id(),
            tool_use.session_id(),
            tool_use.project(),
            tool_use.project_path(),
            tool_use.tool_name(),
            tool_use.tool_input(),
            tool_use.duration_ms(),
            tool_use.timestamp(),
        ),
    )?;
    Ok(())
}

/// 주어진 세션의 모든 도구 사용 기록을 조회한다.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn find_by_session(conn: &Connection, session_id: &str) -> Result<Vec<ToolUse>, AdapterError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp FROM tool_uses WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let rows = stmt
        .query_map([session_id], mapper::tool_use_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    fn sample_tool_use() -> ToolUse {
        ToolUse::new(
            "abcdef1234567890abcdef1234567890".to_string(),
            "sess-1".to_string(),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            r#"{"command":"ls -la"}"#.to_string(),
            0,
            1_713_000_000_000,
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
    fn test_find_by_session_returns_saved() {
        let conn = db::initialize_in_memory().unwrap();
        let tu = sample_tool_use();

        save_tool_use(&conn, &tu).unwrap();

        let results = find_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], tu);
    }

    #[test]
    fn test_find_by_session_empty() {
        let conn = db::initialize_in_memory().unwrap();

        let results = find_by_session(&conn, "nonexistent").unwrap();
        assert!(results.is_empty());
    }
}
