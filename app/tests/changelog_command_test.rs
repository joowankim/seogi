use std::process::{Command, Stdio};

use rusqlite::Connection;

fn run_changelog(args: &[&str], db_path: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(args)
        .env("SEOGI_DB_PATH", db_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[test]
fn test_changelog_add_command() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_changelog(&["changelog", "add", "CLAUDE.md 규칙 변경"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Recorded at"), "stdout: {stdout}");

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM changelog", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let desc: String = conn
        .query_row("SELECT description FROM changelog LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(desc, "CLAUDE.md 규칙 변경");
}

#[test]
fn test_changelog_add_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["changelog", "add"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(!output.status.success());
}
