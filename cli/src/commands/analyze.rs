use std::path::Path;

use anyhow::{Context, Result};

use crate::analyzers::session_summary::compute_metrics;
use crate::config::Config;
use crate::log_reader::read_session_logs;

/// `seogi analyze <project> <session_id>` — 세션 로그에서 메트릭을 계산하여 저장
///
/// # Errors
///
/// Returns an error if log files cannot be read or metrics cannot be written.
pub fn run(config: &Config, project: &str, session_id: &str) -> Result<()> {
    let log_dir = config.log_dir_expanded();
    let entries = read_session_logs(&log_dir, project, session_id)?;

    if entries.is_empty() {
        return Ok(());
    }

    let metrics_entry = compute_metrics(&entries, session_id, project);

    let metrics_dir = log_dir.join(project).join("metrics");
    std::fs::create_dir_all(&metrics_dir)
        .with_context(|| format!("failed to create {}", metrics_dir.display()))?;

    let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let metrics_file = metrics_dir.join(format!("{date_str}.jsonl"));

    let line = serde_json::to_string(&metrics_entry)?;
    append_line(&metrics_file, &line)?;

    Ok(())
}

fn append_line(path: &Path, line: &str) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup_test_logs(name: &str) -> (std::path::PathBuf, Config) {
        let dir = std::env::temp_dir().join(format!("seogi_test_analyze_{name}"));
        let project_dir = dir.join("test-project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let log_path = project_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&log_path).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-04-07T11:00:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Read","duration_ms":100}}}}"#).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-04-07T11:01:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Edit","duration_ms":50,"input":{{"file_path":"a.rs"}}}}}}"#).unwrap();

        let config = Config {
            log_dir: dir.to_str().unwrap().to_string(),
            max_file_size_mb: 10,
        };
        (dir, config)
    }

    #[test]
    fn analyze_creates_metrics_file() {
        let (dir, config) = setup_test_logs("creates_file");
        run(&config, "test-project", "s1").unwrap();

        let metrics_dir = dir.join("test-project").join("metrics");
        assert!(metrics_dir.is_dir());

        let files: Vec<_> = std::fs::read_dir(&metrics_dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        assert_eq!(files.len(), 1);

        let content = std::fs::read_to_string(files[0].path()).unwrap();
        let entry: crate::models::SessionMetricsEntry =
            serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry.metrics.read_before_edit_ratio, 1);
        assert_eq!(entry.metrics.tool_call_count, 2);
        assert_eq!(entry.metrics.edit_files, vec!["a.rs"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn analyze_empty_session_no_output() {
        let dir = std::env::temp_dir().join("seogi_test_analyze_empty");
        std::fs::create_dir_all(dir.join("test-project")).unwrap();

        let config = Config {
            log_dir: dir.to_str().unwrap().to_string(),
            max_file_size_mb: 10,
        };

        run(&config, "test-project", "nonexistent").unwrap();

        let metrics_dir = dir.join("test-project").join("metrics");
        assert!(!metrics_dir.exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
