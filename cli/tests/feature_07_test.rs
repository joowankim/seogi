use std::io::Write;
use std::process::{Command, Stdio};

use rusqlite::Connection;

fn run_migrate(config_path: &str, db_path: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["--config", config_path, "migrate"])
        .env("SEOGI_DB_PATH", db_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn write_config(dir: &std::path::Path, log_dir: &str) -> std::path::PathBuf {
    let config_path = dir.join("config.json");
    let mut f = std::fs::File::create(&config_path).unwrap();
    write!(f, r#"{{"logDir": "{log_dir}"}}"#).unwrap();
    config_path
}

fn write_compact_jsonl(project_dir: &std::path::Path, filename: &str, entries: &[&str]) {
    let path = project_dir.join(filename);
    let mut f = std::fs::File::create(path).unwrap();
    for entry in entries {
        writeln!(f, "{entry}").unwrap();
    }
}

fn compact_tool_entry(session_id: &str, tool_name: &str, ts: &str) -> String {
    format!(
        r#"{{"timestamp":"{ts}","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"{tool_name}","duration_ms":100}}}}"#
    )
}

#[test]
fn test_migrate_command() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let log_dir = dir.path().join("logs");
    let project_dir = log_dir.join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    let e1 = compact_tool_entry("s1", "Bash", "2026-04-07T11:00:00.000Z");
    let e2 = compact_tool_entry("s1", "Read", "2026-04-07T11:01:00.000Z");
    write_compact_jsonl(&project_dir, "2026-04-07.jsonl", &[&e1, &e2]);

    let config_path = write_config(dir.path(), log_dir.to_str().unwrap());

    let output = run_migrate(config_path.to_str().unwrap(), db_path.to_str().unwrap());

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_migrate_idempotent_e2e() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let log_dir = dir.path().join("logs");
    let project_dir = log_dir.join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    let e1 = compact_tool_entry("s1", "Bash", "2026-04-07T11:00:00.000Z");
    write_compact_jsonl(&project_dir, "2026-04-07.jsonl", &[&e1]);

    let config_path = write_config(dir.path(), log_dir.to_str().unwrap());
    let cfg = config_path.to_str().unwrap();
    let db = db_path.to_str().unwrap();

    // First run
    let output1 = run_migrate(cfg, db);
    assert!(output1.status.success());

    // Second run
    let output2 = run_migrate(cfg, db);
    assert!(output2.status.success());

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "idempotent: should still be 1 row after 2 runs");
}

#[test]
fn test_migrate_no_logdir() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let config_path = write_config(dir.path(), "/nonexistent/logdir");

    let output = run_migrate(config_path.to_str().unwrap(), db_path.to_str().unwrap());

    assert!(output.status.success(), "missing logdir should exit 0");
}
