use rusqlite::Connection;
use serde::Deserialize;

use crate::adapter::error::AdapterError;
use crate::adapter::log_repo;
use crate::domain::log::{self, ToolFailure};
use crate::domain::value::{SessionId, Timestamp};

#[derive(Debug, Deserialize)]
struct HookInput {
    session_id: String,
    tool_name: String,
    error: String,
    cwd: String,
}

/// `PostToolUseFailure` 훅의 워크플로우.
///
/// Impureim Sandwich: JSON 파싱(불순) → `ToolFailure` 생성(순수) → DB 저장(불순).
///
/// # Errors
///
/// JSON 파싱 실패 시 `AdapterError::Json`, DB 쓰기 실패 시 `AdapterError::Database`.
pub fn run(conn: &Connection, stdin_json: &str) -> Result<(), AdapterError> {
    // [Top: Impure] JSON 파싱 + ID/타임스탬프 생성
    let input: HookInput = serde_json::from_str(stdin_json)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let timestamp = Timestamp::now();

    // [Middle: Pure] 도메인 타입 생성
    let workspace = log::extract_workspace_from_cwd(&input.cwd);
    let tool_failure = ToolFailure::new(
        id,
        SessionId::new(input.session_id),
        workspace,
        input.cwd,
        input.tool_name,
        input.error,
        timestamp,
    );

    // [Bottom: Impure] DB 저장
    log_repo::save_tool_failure(conn, &tool_failure)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    #[test]
    fn test_run_saves_tool_failure_to_db() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{
            "session_id": "sess-1",
            "tool_name": "Bash",
            "tool_input": {"command": "rm -rf /"},
            "error": "Permission denied",
            "tool_use_id": "toolu_01",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PostToolUseFailure"
        }"#;

        run(&conn, json).unwrap();

        let results = log_repo::list_failures_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id().as_str(), "sess-1");
        assert_eq!(results[0].tool_name(), "Bash");
        assert_eq!(results[0].error(), "Permission denied");
        assert_eq!(results[0].workspace(), "seogi");
        assert_eq!(results[0].workspace_path(), "/Users/kim/projects/seogi");
    }

    #[test]
    fn test_run_invalid_json_returns_error() {
        let conn = db::initialize_in_memory().unwrap();

        let result = run(&conn, "{invalid}");
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }

    #[test]
    fn test_run_missing_session_id_returns_error() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"tool_name":"Bash","error":"fail","cwd":"/test"}"#;

        let result = run(&conn, json);
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }

    #[test]
    fn test_run_missing_error_field_returns_error() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"session_id":"s1","tool_name":"Bash","cwd":"/test"}"#;

        let result = run(&conn, json);
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }
}
