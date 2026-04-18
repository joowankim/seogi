use std::io::Write;
use std::process::{Command, Stdio};

use rusqlite::Connection;

fn run_hook(db_path: &std::path::Path, stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["hook", "post-tool-failure"])
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

fn valid_hook_json(cwd: &str) -> String {
    format!(
        r#"{{
            "session_id": "sess-fail-123",
            "tool_name": "Bash",
            "tool_input": {{"command": "rm -rf /"}},
            "error": "Permission denied",
            "tool_use_id": "toolu_01FAIL",
            "cwd": "{cwd}",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PostToolUseFailure"
        }}"#
    )
}

#[test]
fn test_post_tool_failure_hook_saves_to_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let cwd = "/Users/kim/projects/seogi";
    let input = valid_hook_json(cwd);

    let output = run_hook(&db_path, input.as_bytes());

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tool_failures", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let (session_id, project, tool_name, error): (String, String, String, String) = conn
        .query_row(
            "SELECT session_id, project, tool_name, error FROM tool_failures LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(session_id, "sess-fail-123");
    assert_eq!(project, "seogi");
    assert_eq!(tool_name, "Bash");
    assert_eq!(error, "Permission denied");
}

#[test]
fn test_post_tool_failure_hook_empty_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, b"");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_post_tool_failure_hook_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_hook(&db_path, b"{invalid}");

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_post_tool_failure_hook_missing_session_id() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"tool_name":"Bash","error":"fail","cwd":"/test","tool_use_id":"t1","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"PostToolUseFailure"}"#;

    let output = run_hook(&db_path, input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_post_tool_failure_hook_missing_tool_name() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","error":"fail","cwd":"/test","tool_use_id":"t1","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"PostToolUseFailure"}"#;

    let output = run_hook(&db_path, input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_post_tool_failure_hook_missing_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","tool_name":"Bash","cwd":"/test","tool_use_id":"t1","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"PostToolUseFailure"}"#;

    let output = run_hook(&db_path, input.as_bytes());

    assert!(output.status.success(), "hooks should exit 0 even on error");
}
