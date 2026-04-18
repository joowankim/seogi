use std::io::Write;
use std::process::{Command, Stdio};

fn run_report(args: &[&str], db_path: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_seogi"));
    cmd.args(args)
        .env("SEOGI_DB_PATH", db_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.output().unwrap()
}

fn run_hook(db_path: &str, hook: &str, stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["hook", hook])
        .env("SEOGI_DB_PATH", db_path)
        .env(
            "SEOGI_DIR",
            std::path::Path::new(db_path)
                .parent()
                .unwrap_or(std::path::Path::new("/tmp")),
        )
        .env("SEOGI_NO_NOTIFY", "1")
        .env("SEOGI_TIMING_DIR", "/tmp/seogi-test-timing-report")
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

fn insert_tool_use(db_path: &str, session_id: &str, tool_name: &str) {
    let json = format!(
        r#"{{
            "session_id": "{session_id}",
            "tool_name": "{tool_name}",
            "tool_input": {{"command": "ls"}},
            "tool_response": {{"stdout": "ok"}},
            "tool_use_id": "toolu_{session_id}_{tool_name}",
            "cwd": "/Users/kim/projects/seogi",
            "transcript_path": "/tmp/transcript.jsonl",
            "permission_mode": "default",
            "hook_event_name": "PostToolUse"
        }}"#
    );
    let output = run_hook(db_path, "post-tool", json.as_bytes());
    assert!(output.status.success());
}

#[test]
fn test_report_command_outputs_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    insert_tool_use(db, "sess-1", "Read");
    insert_tool_use(db, "sess-1", "Edit");
    insert_tool_use(db, "sess-2", "Bash");

    let output = run_report(
        &["report", "--from", "2020-01-01", "--to", "2030-12-31"],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("n=2"), "should show 2 sessions: {stdout}");
    assert!(stdout.contains("tool_call_count"));
}

#[test]
fn test_report_command_empty_period() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // Initialize DB but no data
    insert_tool_use(db, "sess-1", "Read");

    let output = run_report(
        &["report", "--from", "2099-01-01", "--to", "2099-12-31"],
        db,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("데이터가 없습니다"),
        "should show empty message: {stdout}"
    );
}

#[test]
fn test_report_command_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["report"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_report_command_bad_db() {
    let output = run_report(
        &["report", "--from", "2026-04-01", "--to", "2026-04-30"],
        "/nonexistent/seogi.db",
    );

    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn test_report_command_invalid_date() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    let output = run_report(
        &["report", "--from", "not-a-date", "--to", "2026-04-30"],
        db_path.to_str().unwrap(),
    );

    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}
