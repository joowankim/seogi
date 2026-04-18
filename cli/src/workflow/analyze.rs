use rusqlite::Connection;

use crate::adapter::error::AdapterError;
use crate::adapter::log_repo;
use crate::domain::metrics::{self, SessionMetrics};

/// 세션 분석 워크플로우.
///
/// Impureim Sandwich: DB 조회(불순) → 지표 계산(순수) → 결과 반환.
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
pub fn run(conn: &Connection, session_id: &str) -> Result<SessionMetrics, AdapterError> {
    // [Top: Impure] DB에서 데이터 로드
    let tool_uses = log_repo::list_by_session(conn, session_id)?;
    let tool_failures = log_repo::list_failures_by_session(conn, session_id)?;

    // [Middle: Pure] 지표 계산
    let metrics = metrics::calculate(session_id, &tool_uses, &tool_failures);

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;
    use crate::adapter::log_repo;
    use crate::domain::log::ToolUse;
    use crate::domain::value::{Ms, SessionId, Timestamp};

    fn make_tool_use(name: &str, input: &str, ts: i64) -> ToolUse {
        ToolUse::new(
            format!("id-{ts}"),
            SessionId::new("sess-1"),
            "test".to_string(),
            "/test".to_string(),
            name.to_string(),
            input.to_string(),
            Ms::zero(),
            Timestamp::new(ts),
        )
    }

    #[test]
    fn test_workflow_analyze_run() {
        let conn = db::initialize_in_memory().unwrap();

        let uses = vec![
            make_tool_use("Read", r#"{"file_path":"/src/main.rs"}"#, 1000),
            make_tool_use("Read", r#"{"file_path":"/src/lib.rs"}"#, 2000),
            make_tool_use("Edit", r#"{"file_path":"/src/main.rs"}"#, 3000),
        ];
        for tu in &uses {
            log_repo::save_tool_use(&conn, tu).unwrap();
        }

        let metrics = run(&conn, "sess-1").unwrap();
        assert_eq!(metrics.read_before_edit_ratio(), 2);
        assert_eq!(metrics.tool_call_count(), 3);
    }

    #[test]
    fn test_workflow_analyze_empty_session() {
        let conn = db::initialize_in_memory().unwrap();

        let metrics = run(&conn, "nonexistent").unwrap();
        assert_eq!(metrics.tool_call_count(), 0);
        assert_eq!(metrics.read_before_edit_ratio(), 0);
        assert!(!metrics.test_invoked());
        assert!(metrics.edit_files().is_empty());
    }
}
