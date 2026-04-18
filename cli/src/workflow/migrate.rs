use std::path::Path;

use rusqlite::Connection;

use crate::adapter::error::AdapterError;
use crate::adapter::log_repo;
use crate::domain::migrate::{self, MigrateSummary, MigratedRecord};
use crate::log_reader;

/// JSONL 로그 디렉토리를 `SQLite`로 마이그레이션한다.
///
/// `{log_dir}/{project}/*.jsonl` 파일을 순회하며
/// `LogEntry`를 `ToolUse`/`ToolFailure`로 변환하여 DB에 저장한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError`.
pub fn run(conn: &Connection, log_dir: &Path) -> Result<MigrateSummary, AdapterError> {
    let mut summary = MigrateSummary::default();

    if !log_dir.is_dir() {
        return Ok(summary);
    }

    conn.execute_batch("BEGIN")?;

    let result = (|| {
        for project_entry in std::fs::read_dir(log_dir)?.flatten() {
            let project_path = project_entry.path();
            if project_path.is_dir() {
                process_project(conn, &project_path, &mut summary)?;
            }
        }
        Ok::<(), AdapterError>(())
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(ref e) => {
            eprintln!("warning: migration error, rolling back: {e}");
            conn.execute_batch("ROLLBACK")?;
            return Err(result.unwrap_err());
        }
    }

    Ok(summary)
}

fn process_project(
    conn: &Connection,
    project_dir: &Path,
    summary: &mut MigrateSummary,
) -> Result<(), AdapterError> {
    for file_entry in std::fs::read_dir(project_dir)?.flatten() {
        let file_path = file_entry.path();
        if file_path.is_dir() || file_path.extension().is_none_or(|ext| ext != "jsonl") {
            continue;
        }
        process_jsonl_file(conn, &file_path, summary)?;
    }
    Ok(())
}

fn process_jsonl_file(
    conn: &Connection,
    file_path: &Path,
    summary: &mut MigrateSummary,
) -> Result<(), AdapterError> {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: failed to read {}: {e}", file_path.display());
            return Ok(());
        }
    };

    let entries = log_reader::parse_jsonl_content(&content);
    summary.files += 1;

    for entry in &entries {
        match migrate::convert_entry(entry) {
            Some(MigratedRecord::Use(tu)) => {
                log_repo::save_tool_use(conn, &tu)?;
                summary.tool_uses += 1;
            }
            Some(MigratedRecord::Failure(tf)) => {
                log_repo::save_tool_failure(conn, &tf)?;
                summary.tool_failures += 1;
            }
            None => {
                summary.skipped += 1;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;
    use std::io::Write;

    fn compact_tool(session_id: &str, tool_name: &str, ts: &str) -> String {
        format!(
            r#"{{"timestamp":"{ts}","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"{tool_name}","duration_ms":100}}}}"#
        )
    }

    fn compact_failed(session_id: &str, tool_name: &str, ts: &str) -> String {
        format!(
            r#"{{"timestamp":"{ts}","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"{tool_name}","failed":true,"error":"exit 1"}}}}"#
        )
    }

    fn compact_no_tool(session_id: &str, ts: &str) -> String {
        format!(
            r#"{{"timestamp":"{ts}","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"system","content":"[stop]","tool":null}}"#
        )
    }

    fn pretty_tool(session_id: &str, tool_name: &str, ts: &str) -> String {
        format!(
            r#"{{
  "timestamp": "{ts}",
  "sessionId": "{session_id}",
  "project": "test",
  "projectPath": "/test",
  "role": "assistant",
  "tool": {{
    "name": "{tool_name}",
    "duration_ms": 100
  }}
}}"#
        )
    }

    fn setup_log_dir(dir: &std::path::Path, project: &str) -> std::path::PathBuf {
        let project_dir = dir.join(project);
        std::fs::create_dir_all(&project_dir).unwrap();
        project_dir
    }

    fn write_jsonl(project_dir: &std::path::Path, filename: &str, lines: &[String]) {
        let path = project_dir.join(filename);
        let mut f = std::fs::File::create(path).unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
    }

    #[test]
    fn test_migrate_compact_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[compact_tool("s1", "Bash", "2026-04-07T11:00:00.000Z")],
        );

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_uses, 1);
        assert_eq!(summary.files, 1);
    }

    #[test]
    fn test_migrate_pretty_printed() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        let path = project_dir.join("2026-04-07.jsonl");
        std::fs::write(&path, pretty_tool("s1", "Read", "2026-04-07T11:00:00.000Z")).unwrap();

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_uses, 1);
    }

    #[test]
    fn test_migrate_failure() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[compact_failed("s1", "Bash", "2026-04-07T11:00:00.000Z")],
        );

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_failures, 1);
    }

    #[test]
    fn test_migrate_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[compact_tool("s1", "Bash", "2026-04-07T11:00:00.000Z")],
        );

        let conn = db::initialize_in_memory().unwrap();
        run(&conn, dir.path()).unwrap();
        run(&conn, dir.path()).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migrate_skips_unparseable() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[
                compact_tool("s1", "Bash", "2026-04-07T11:00:00.000Z"),
                "{invalid json}".to_string(),
            ],
        );

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_uses, 1);
    }

    #[test]
    fn test_migrate_skips_no_tool() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[
                compact_no_tool("s1", "2026-04-07T11:00:00.000Z"),
                compact_tool("s1", "Bash", "2026-04-07T11:01:00.000Z"),
            ],
        );

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_uses, 1);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn test_migrate_skips_metrics_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = setup_log_dir(dir.path(), "proj");
        let metrics_dir = project_dir.join("metrics");
        std::fs::create_dir_all(&metrics_dir).unwrap();

        write_jsonl(
            &metrics_dir,
            "2026-04-07.jsonl",
            &[compact_tool("s1", "Read", "2026-04-07T11:00:00.000Z")],
        );

        write_jsonl(
            &project_dir,
            "2026-04-07.jsonl",
            &[compact_tool("s1", "Bash", "2026-04-07T11:01:00.000Z")],
        );

        let conn = db::initialize_in_memory().unwrap();
        let summary = run(&conn, dir.path()).unwrap();
        assert_eq!(summary.tool_uses, 1);
        assert_eq!(summary.files, 1);
    }
}
