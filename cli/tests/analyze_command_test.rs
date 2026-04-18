use std::io::Write;
use std::process::{Command, Stdio};

fn run_cmd(args: &[&str], env_vars: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_seogi"));
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, val) in env_vars {
        cmd.env(key, val);
    }

    cmd.output().unwrap()
}

fn run_hook(db_path: &std::path::Path, hook: &str, stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["hook", hook])
        .env("SEOGI_DB_PATH", db_path)
        .env("SEOGI_TIMING_DIR", "/tmp/seogi-test-timing")
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

fn insert_tool_uses(db_path: &std::path::Path) {
    // Insert Read, Read, Edit tool_uses for sess-1
    let tools = [
        ("Read", r#"{"file_path":"/src/main.rs"}"#),
        ("Read", r#"{"file_path":"/src/lib.rs"}"#),
        ("Edit", r#"{"file_path":"/src/main.rs"}"#),
        ("Bash", r#"{"command":"cargo test"}"#),
        ("Bash", r#"{"command":"ls"}"#),
    ];

    for (i, (name, input)) in tools.iter().enumerate() {
        let json = format!(
            r#"{{
                "session_id": "sess-1",
                "tool_name": "{name}",
                "tool_input": {input},
                "tool_response": {{"stdout": "ok"}},
                "tool_use_id": "toolu_{i:02}",
                "cwd": "/Users/kim/projects/seogi",
                "transcript_path": "/tmp/transcript.jsonl",
                "permission_mode": "default",
                "hook_event_name": "PostToolUse"
            }}"#
        );
        let output = run_hook(db_path, "post-tool", json.as_bytes());
        assert!(
            output.status.success(),
            "Failed to insert tool_use {i}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_analyze_command_outputs_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");

    insert_tool_uses(&db_path);

    let output = run_cmd(
        &["analyze", "sess-1"],
        &[("SEOGI_DB_PATH", db_path.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(json["session_id"], "sess-1");
    assert_eq!(json["tool_call_count"], 5);
    assert_eq!(json["read_before_edit_ratio"], 2);
    assert_eq!(json["test_invoked"], true);
}

#[test]
fn test_analyze_command_no_args() {
    let output = run_cmd(&["analyze"], &[]);

    assert!(!output.status.success());
}

#[test]
fn test_analyze_command_bad_db() {
    let output = run_cmd(
        &["analyze", "sess-1"],
        &[("SEOGI_DB_PATH", "/nonexistent/path/seogi.db")],
    );

    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}
