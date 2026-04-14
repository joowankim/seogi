use anyhow::{Context, Result};
use std::path::Path;

use crate::models::SessionMetricsEntry;

/// 기간 내 메트릭 파일을 읽어 반환한다.
///
/// - `log_dir/<project>/metrics/*.jsonl` 파일에서 날짜 범위 필터
/// - project가 None이면 모든 프로젝트의 metrics를 읽음
///
/// # Errors
///
/// Returns an error if the metrics directory cannot be read.
pub fn read_metrics(
    log_dir: &Path,
    project: Option<&str>,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<SessionMetricsEntry>> {
    let mut entries = Vec::new();

    let project_dirs: Vec<_> = if let Some(proj) = project {
        let dir = log_dir.join(proj).join("metrics");
        if dir.is_dir() { vec![dir] } else { vec![] }
    } else {
        collect_all_metrics_dirs(log_dir)?
    };

    for metrics_dir in project_dirs {
        read_metrics_from_dir(&metrics_dir, date_from, date_to, &mut entries)?;
    }

    Ok(entries)
}

fn collect_all_metrics_dirs(log_dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut dirs = Vec::new();
    if !log_dir.is_dir() {
        return Ok(dirs);
    }
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let metrics_dir = entry.path().join("metrics");
        if metrics_dir.is_dir() {
            dirs.push(metrics_dir);
        }
    }
    Ok(dirs)
}

fn read_metrics_from_dir(
    metrics_dir: &Path,
    date_from: &str,
    date_to: &str,
    entries: &mut Vec<SessionMetricsEntry>,
) -> Result<()> {
    for file_entry in std::fs::read_dir(metrics_dir)
        .with_context(|| format!("failed to read {}", metrics_dir.display()))?
    {
        let file_entry = file_entry?;
        let path = file_entry.path();

        if path.extension().is_none_or(|ext| ext != "jsonl") {
            continue;
        }

        // 파일명에서 날짜 추출하여 범위 필터
        let filename = path.file_stem().unwrap_or_default().to_string_lossy();
        // 파일명 형식: "2026-04-07" 또는 "2026-04-07_001"
        let file_date = filename.split('_').next().unwrap_or("");

        if file_date < date_from || file_date > date_to {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<SessionMetricsEntry>(line) {
                entries.push(entry);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_metrics_line(session_id: &str, tool_count: u32, with_new_fields: bool) -> String {
        if with_new_fields {
            format!(
                r#"{{"timestamp":"2026-04-08T12:00:00.000Z","sessionId":"{session_id}","project":"test","metrics":{{"read_before_edit_ratio":5,"doom_loop_count":0,"test_invoked":true,"build_invoked":false,"tool_call_count":{tool_count},"session_duration_ms":180000,"edit_files":[],"lint_invoked":false,"typecheck_invoked":true,"bash_error_rate":0.1}}}}"#
            )
        } else {
            format!(
                r#"{{"timestamp":"2026-04-07T12:00:00.000Z","sessionId":"{session_id}","project":"test","metrics":{{"read_before_edit_ratio":3,"doom_loop_count":1,"test_invoked":false,"build_invoked":false,"tool_call_count":{tool_count},"session_duration_ms":5000,"edit_files":[]}}}}"#
            )
        }
    }

    fn setup_metrics_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("seogi_test_{name}"));
        let metrics_dir = dir.join("test-project").join("metrics");
        std::fs::create_dir_all(&metrics_dir).unwrap();
        dir
    }

    #[test]
    fn read_metrics_single_project() {
        let dir = setup_metrics_dir("metrics_single");
        let metrics_dir = dir.join("test-project").join("metrics");
        let path = metrics_dir.join("2026-04-08.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{}", make_metrics_line("s1", 42, true)).unwrap();
        writeln!(f, "{}", make_metrics_line("s2", 10, true)).unwrap();

        let entries = read_metrics(&dir, Some("test-project"), "2026-04-08", "2026-04-08").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].metrics.tool_call_count, 42);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_metrics_date_range_filter() {
        let dir = setup_metrics_dir("metrics_date_range");
        let metrics_dir = dir.join("test-project").join("metrics");

        for date in &["2026-04-07", "2026-04-08", "2026-04-09"] {
            let path = metrics_dir.join(format!("{date}.jsonl"));
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "{}", make_metrics_line("s1", 10, true)).unwrap();
        }

        let entries = read_metrics(&dir, Some("test-project"), "2026-04-07", "2026-04-08").unwrap();
        assert_eq!(entries.len(), 2); // 04-07과 04-08만

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_metrics_all_projects() {
        let dir = setup_metrics_dir("metrics_all_projects");

        for proj in &["proj-a", "proj-b"] {
            let metrics_dir = dir.join(proj).join("metrics");
            std::fs::create_dir_all(&metrics_dir).unwrap();
            let path = metrics_dir.join("2026-04-08.jsonl");
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "{}", make_metrics_line("s1", 10, true)).unwrap();
        }

        let entries = read_metrics(&dir, None, "2026-04-08", "2026-04-08").unwrap();
        assert_eq!(entries.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_metrics_old_schema() {
        let dir = setup_metrics_dir("metrics_old_schema");
        let metrics_dir = dir.join("test-project").join("metrics");
        let path = metrics_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{}", make_metrics_line("s1", 10, false)).unwrap();

        let entries = read_metrics(&dir, Some("test-project"), "2026-04-07", "2026-04-07").unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].metrics.lint_invoked.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_metrics_empty_dir() {
        let entries = read_metrics(
            Path::new("/nonexistent"),
            Some("nope"),
            "2026-04-08",
            "2026-04-08",
        )
        .unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn read_metrics_rollover_files() {
        let dir = setup_metrics_dir("metrics_rollover");
        let metrics_dir = dir.join("test-project").join("metrics");

        let path1 = metrics_dir.join("2026-04-08.jsonl");
        let mut f = std::fs::File::create(&path1).unwrap();
        writeln!(f, "{}", make_metrics_line("s1", 10, true)).unwrap();

        let path2 = metrics_dir.join("2026-04-08_001.jsonl");
        let mut f = std::fs::File::create(&path2).unwrap();
        writeln!(f, "{}", make_metrics_line("s2", 20, true)).unwrap();

        let entries = read_metrics(&dir, Some("test-project"), "2026-04-08", "2026-04-08").unwrap();
        assert_eq!(entries.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }
}
