use std::io::Write;
use std::process::{Command, Stdio};

use rusqlite::Connection;

fn run_hook(db_path: &std::path::Path, args: &[&str], stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(args)
        .env("SEOGI_DB_PATH", db_path)
        .env("SEOGI_DIR", db_path.parent().unwrap())
        .env("SEOGI_NO_NOTIFY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data).unwrap();
    }

    child.wait_with_output().unwrap()
}

fn valid_notification_json(cwd: &str) -> String {
    format!(
        r#"{{
            "session_id": "sess-notif-123",
            "message": "Permission required for Bash",
            "notification_type": "permission_prompt",
            "cwd": "{cwd}",
            "transcript_path": "/tmp/transcript.jsonl",
            "hook_event_name": "Notification"
        }}"#
    )
}

fn valid_stop_json(cwd: &str) -> String {
    format!(
        r#"{{
            "session_id": "sess-stop-456",
            "stop_reason": "end_turn",
            "cwd": "{cwd}",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "Stop"
        }}"#
    )
}

// --- Notification E2E ---

#[test]
fn test_notification_hook_saves_to_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let cwd = "/Users/kim/projects/seogi";
    let input = valid_notification_json(cwd);

    let output = run_hook(&db_path, &["hook", "notification"], input.as_bytes());

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM system_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let (session_id, event_type, content, workspace, workspace_path): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT session_id, event_type, content, workspace, workspace_path FROM system_events LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(session_id, "sess-notif-123");
    assert_eq!(event_type, "Notification");
    assert_eq!(content, "Permission required for Bash");
    assert_eq!(workspace, "seogi");
    assert_eq!(workspace_path, "/Users/kim/projects/seogi");
}

#[test]
fn test_notification_hook_empty_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, &["hook", "notification"], b"");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_notification_hook_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, &["hook", "notification"], b"{invalid}");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_notification_hook_missing_session_id() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"message":"hello","cwd":"/test","transcript_path":"/tmp/t","hook_event_name":"Notification"}"#;

    let output = run_hook(&db_path, &["hook", "notification"], input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_notification_hook_missing_message() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","cwd":"/test","transcript_path":"/tmp/t","hook_event_name":"Notification"}"#;

    let output = run_hook(&db_path, &["hook", "notification"], input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

// --- Stop E2E ---

#[test]
fn test_stop_hook_saves_to_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let cwd = "/Users/kim/projects/seogi";
    let input = valid_stop_json(cwd);

    let output = run_hook(&db_path, &["hook", "stop"], input.as_bytes());

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM system_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let (session_id, event_type, content, workspace, workspace_path): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT session_id, event_type, content, workspace, workspace_path FROM system_events LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(session_id, "sess-stop-456");
    assert_eq!(event_type, "Stop");
    assert_eq!(content, "end_turn");
    assert_eq!(workspace, "seogi");
    assert_eq!(workspace_path, "/Users/kim/projects/seogi");
}

#[test]
fn test_stop_hook_empty_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, &["hook", "stop"], b"");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_stop_hook_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, &["hook", "stop"], b"{invalid}");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_stop_hook_missing_session_id() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"stop_reason":"end_turn","cwd":"/test","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"Stop"}"#;

    let output = run_hook(&db_path, &["hook", "stop"], input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_stop_hook_missing_stop_reason() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","cwd":"/test","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"Stop"}"#;

    let output = run_hook(&db_path, &["hook", "stop"], input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}
