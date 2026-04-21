use rusqlite::Connection;
use serde::Deserialize;

use crate::adapter::error::AdapterError;
use crate::adapter::log_repo;
use crate::domain::log::{self, SystemEvent};
use crate::domain::value::{SessionId, Timestamp};

#[derive(Debug, Deserialize)]
struct NotificationInput {
    session_id: String,
    message: Option<String>,
    cwd: String,
}

#[derive(Debug, Deserialize)]
struct StopInput {
    session_id: String,
    stop_reason: Option<String>,
    cwd: String,
}

/// `Notification` 훅의 워크플로우.
///
/// Impureim Sandwich: JSON 파싱(불순) → `SystemEvent` 생성(순수) → DB 저장(불순).
///
/// # Errors
///
/// JSON 파싱 실패 시 `AdapterError::Json`, DB 쓰기 실패 시 `AdapterError::Database`.
pub fn run_notification(conn: &Connection, stdin_json: &str) -> Result<(), AdapterError> {
    // [Top: Impure]
    let input: NotificationInput = serde_json::from_str(stdin_json)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let timestamp = Timestamp::now();

    // [Middle: Pure]
    let project = log::extract_project_from_cwd(&input.cwd);
    let event = SystemEvent::new(
        id,
        SessionId::new(input.session_id),
        project,
        input.cwd,
        "Notification".to_string(),
        input.message.unwrap_or_default(),
        timestamp,
    );

    // [Bottom: Impure]
    log_repo::save_system_event(conn, &event)?;

    Ok(())
}

/// `Stop` 훅의 워크플로우.
///
/// Impureim Sandwich: JSON 파싱(불순) → `SystemEvent` 생성(순수) → DB 저장(불순).
///
/// # Errors
///
/// JSON 파싱 실패 시 `AdapterError::Json`, DB 쓰기 실패 시 `AdapterError::Database`.
pub fn run_stop(conn: &Connection, stdin_json: &str) -> Result<(), AdapterError> {
    // [Top: Impure]
    let input: StopInput = serde_json::from_str(stdin_json)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let timestamp = Timestamp::now();

    // [Middle: Pure]
    let project = log::extract_project_from_cwd(&input.cwd);
    let event = SystemEvent::new(
        id,
        SessionId::new(input.session_id),
        project,
        input.cwd,
        "Stop".to_string(),
        input.stop_reason.unwrap_or_default(),
        timestamp,
    );

    // [Bottom: Impure]
    log_repo::save_system_event(conn, &event)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    #[test]
    fn test_run_notification_saves_to_db() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{
            "session_id": "sess-1",
            "message": "Permission required for Bash",
            "notification_type": "permission_prompt",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "hook_event_name": "Notification"
        }"#;

        run_notification(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id().as_str(), "sess-1");
        assert_eq!(results[0].event_type(), "Notification");
        assert_eq!(results[0].content(), "Permission required for Bash");
        assert_eq!(results[0].project(), "seogi");
        assert_eq!(results[0].project_path(), "/Users/kim/projects/seogi");
    }

    #[test]
    fn test_run_stop_saves_to_db() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{
            "session_id": "sess-1",
            "stop_reason": "end_turn",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "Stop"
        }"#;

        run_stop(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "sess-1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id().as_str(), "sess-1");
        assert_eq!(results[0].event_type(), "Stop");
        assert_eq!(results[0].content(), "end_turn");
        assert_eq!(results[0].project(), "seogi");
    }

    #[test]
    fn test_run_notification_invalid_json() {
        let conn = db::initialize_in_memory().unwrap();
        let result = run_notification(&conn, "{invalid}");
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }

    #[test]
    fn test_run_stop_missing_session_id() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"stop_reason":"end_turn","cwd":"/test"}"#;
        let result = run_stop(&conn, json);
        assert!(matches!(result, Err(AdapterError::Json(_))));
    }

    #[test]
    fn test_run_notification_missing_message() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"session_id":"s1","cwd":"/test"}"#;

        run_notification(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "s1").unwrap();
        assert_eq!(results[0].content(), "");
    }

    #[test]
    fn test_run_stop_null_stop_reason() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"session_id":"s1","stop_reason":null,"cwd":"/test"}"#;

        run_stop(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "s1").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content(), "");
    }

    #[test]
    fn test_run_stop_missing_stop_reason() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"session_id":"s1","cwd":"/test"}"#;

        run_stop(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "s1").unwrap();
        assert_eq!(results[0].content(), "");
    }

    #[test]
    fn test_run_notification_null_message() {
        let conn = db::initialize_in_memory().unwrap();
        let json = r#"{"session_id":"s1","message":null,"cwd":"/test"}"#;

        run_notification(&conn, json).unwrap();

        let results = log_repo::list_system_events_by_session(&conn, "s1").unwrap();
        assert_eq!(results[0].content(), "");
    }
}
