use rusqlite::Connection;
use serde::Deserialize;

use crate::adapter::error::AdapterError;
use crate::adapter::{log_repo, timing};
use crate::domain::log::{self, ToolUse};
use crate::domain::value::{Ms, SessionId, Timestamp};

#[derive(Debug, Deserialize)]
struct HookInput {
    session_id: String,
    tool_name: String,
    #[serde(default)]
    tool_input: serde_json::Value,
    #[serde(default)]
    tool_use_id: String,
    cwd: String,
}

/// `PostToolUse` 훅의 워크플로우.
///
/// Impureim Sandwich: JSON 파싱(불순) → `ToolUse` 생성(순수) → DB 저장(불순).
///
/// # Errors
///
/// JSON 파싱 실패 시 `AdapterError::Json`, DB 쓰기 실패 시 `AdapterError::Database`.
pub fn run(conn: &Connection, stdin_json: &str) -> Result<(), AdapterError> {
    // [Top: Impure] JSON 파싱 + ID/타임스탬프 생성
    let input: HookInput = serde_json::from_str(stdin_json)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let now = Timestamp::now();
    let tool_input_str = serde_json::to_string(&input.tool_input)?;
    let timing_dir = timing::timing_dir();
    let start_time = timing::read_and_remove_start_time(&timing_dir, &input.tool_use_id);

    // [Middle: Pure] 도메인 타입 생성
    let project = log::extract_project_from_cwd(&input.cwd);
    let duration = start_time.map_or(Ms::zero(), |start| Ms::new(now.value() - start.value()));
    let tool_use = ToolUse::new(
        id,
        SessionId::new(input.session_id),
        project,
        input.cwd,
        input.tool_name,
        tool_input_str,
        duration,
        now,
    );

    // [Bottom: Impure] DB 저장
    log_repo::save_tool_use(conn, &tool_use)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    #[test]
    fn test_run_saves_tool_use_to_db() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{
            "session_id": "sess-1",
            "tool_name": "Bash",
            "tool_input": {"command": "ls"},
            "tool_response": {"stdout": "ok"},
            "tool_use_id": "toolu_01",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PostToolUse"
        }"#;

        run(&conn, json).unwrap();

        let results = log_repo::list_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id().as_str(), "sess-1");
        assert_eq!(results[0].tool_name(), "Bash");
        assert_eq!(results[0].project(), "seogi");
        assert_eq!(results[0].project_path(), "/Users/kim/projects/seogi");
        assert_eq!(results[0].tool_input(), r#"{"command":"ls"}"#);
        assert_eq!(results[0].duration(), Ms::zero());
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
        let json = r#"{"tool_name":"Bash","tool_input":{},"cwd":"/test"}"#;

        let result = run(&conn, json);
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }
}
