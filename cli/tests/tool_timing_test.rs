use std::io::Write;
use std::process::{Command, Stdio};

use rusqlite::Connection;

fn run_hook(args: &[&str], env_vars: &[(&str, &str)], stdin_data: &[u8]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_seogi"));
    cmd.args(args)
        .env("SEOGI_NO_NOTIFY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, val) in env_vars {
        cmd.env(key, val);
    }

    let mut child = cmd.spawn().unwrap();

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data).unwrap();
    }

    child.wait_with_output().unwrap()
}

fn pre_tool_json(tool_use_id: &str) -> String {
    format!(
        r#"{{
            "session_id": "sess-1",
            "tool_name": "Bash",
            "tool_input": {{"command": "ls"}},
            "tool_use_id": "{tool_use_id}",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PreToolUse"
        }}"#
    )
}

fn post_tool_json(tool_use_id: &str) -> String {
    format!(
        r#"{{
            "session_id": "sess-1",
            "tool_name": "Bash",
            "tool_input": {{"command": "ls"}},
            "tool_response": {{"stdout": "ok"}},
            "tool_use_id": "{tool_use_id}",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PostToolUse"
        }}"#
    )
}

// --- pre-tool E2E ---

#[test]
fn test_pre_tool_hook_creates_timing_file() {
    let timing_dir = tempfile::tempdir().unwrap();
    let tool_use_id = "toolu_01TIMING";
    let input = pre_tool_json(tool_use_id);

    let output = run_hook(
        &["hook", "pre-tool"],
        &[("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap())],
        input.as_bytes(),
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let timing_file = timing_dir.path().join(format!("{tool_use_id}_start"));
    assert!(timing_file.exists(), "timing file should exist");

    let content = std::fs::read_to_string(&timing_file).unwrap();
    let ts: i64 = content.trim().parse().expect("should be a valid i64");
    let now = chrono::Utc::now().timestamp_millis();
    assert!(
        (now - ts).abs() < 1000,
        "timestamp should be within 1s of now"
    );
}

#[test]
fn test_pre_tool_then_post_tool_duration() {
    let timing_dir = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("seogi.db");
    let tool_use_id = "toolu_02DURATION";

    let env_vars: Vec<(&str, &str)> = vec![
        ("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap()),
        ("SEOGI_DB_PATH", db_path.to_str().unwrap()),
    ];

    // pre-tool
    let pre_output = run_hook(
        &["hook", "pre-tool"],
        &env_vars,
        pre_tool_json(tool_use_id).as_bytes(),
    );
    assert!(pre_output.status.success());

    // small delay to ensure duration > 0
    std::thread::sleep(std::time::Duration::from_millis(5));

    // post-tool
    let post_output = run_hook(
        &["hook", "post-tool"],
        &env_vars,
        post_tool_json(tool_use_id).as_bytes(),
    );
    assert!(
        post_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&post_output.stderr)
    );

    // duration_ms > 0
    let conn = Connection::open(&db_path).unwrap();
    let duration_ms: i64 = conn
        .query_row("SELECT duration_ms FROM tool_uses LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        duration_ms > 0,
        "duration_ms should be > 0, got {duration_ms}"
    );

    // timing file should be deleted
    let timing_file = timing_dir.path().join(format!("{tool_use_id}_start"));
    assert!(
        !timing_file.exists(),
        "timing file should be deleted after post-tool"
    );
}

#[test]
fn test_post_tool_without_pre_tool_fallback() {
    let timing_dir = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("seogi.db");

    let env_vars: Vec<(&str, &str)> = vec![
        ("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap()),
        ("SEOGI_DB_PATH", db_path.to_str().unwrap()),
    ];

    // post-tool only (no pre-tool)
    let output = run_hook(
        &["hook", "post-tool"],
        &env_vars,
        post_tool_json("toolu_03NOPRE").as_bytes(),
    );
    assert!(output.status.success());

    let conn = Connection::open(&db_path).unwrap();
    let duration_ms: i64 = conn
        .query_row("SELECT duration_ms FROM tool_uses LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(duration_ms, 0, "duration_ms should be 0 without pre-tool");
}

#[test]
fn test_pre_tool_hook_empty_stdin() {
    let timing_dir = tempfile::tempdir().unwrap();

    let output = run_hook(
        &["hook", "pre-tool"],
        &[
            ("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap()),
            ("SEOGI_DIR", timing_dir.path().to_str().unwrap()),
        ],
        b"",
    );

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_pre_tool_hook_invalid_json() {
    let timing_dir = tempfile::tempdir().unwrap();

    let output = run_hook(
        &["hook", "pre-tool"],
        &[
            ("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap()),
            ("SEOGI_DIR", timing_dir.path().to_str().unwrap()),
        ],
        b"{invalid}",
    );

    assert!(output.status.success(), "hooks should exit 0 even on error");
}

#[test]
fn test_pre_tool_hook_missing_tool_use_id() {
    let timing_dir = tempfile::tempdir().unwrap();
    let input = r#"{"session_id":"s1","tool_name":"Bash","tool_input":{},"cwd":"/test","transcript_path":"/tmp/t","permission_mode":"default","hook_event_name":"PreToolUse"}"#;

    let output = run_hook(
        &["hook", "pre-tool"],
        &[
            ("SEOGI_TIMING_DIR", timing_dir.path().to_str().unwrap()),
            ("SEOGI_DIR", timing_dir.path().to_str().unwrap()),
        ],
        input.as_bytes(),
    );

    assert!(output.status.success(), "hooks should exit 0 even on error");
}
