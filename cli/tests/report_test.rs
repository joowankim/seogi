use std::io::Write;

use seogi::config::Config;
use seogi::metrics_reader::read_metrics;

fn setup_metrics(name: &str) -> (std::path::PathBuf, Config) {
    let dir = std::env::temp_dir().join(format!("seogi_integ_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let metrics_dir = dir.join("test-project").join("metrics");
    std::fs::create_dir_all(&metrics_dir).unwrap();

    let path = metrics_dir.join("2026-04-08.jsonl");
    let mut f = std::fs::File::create(&path).unwrap();

    // 3 sessions with varying metrics
    writeln!(f, r#"{{"timestamp":"2026-04-08T10:00:00.000Z","sessionId":"s1","project":"test-project","metrics":{{"read_before_edit_ratio":5,"doom_loop_count":0,"test_invoked":true,"build_invoked":false,"tool_call_count":50,"session_duration_ms":300000,"edit_files":["a.rs"],"lint_invoked":false,"typecheck_invoked":false,"bash_error_rate":0.0}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-08T11:00:00.000Z","sessionId":"s2","project":"test-project","metrics":{{"read_before_edit_ratio":10,"doom_loop_count":1,"test_invoked":false,"build_invoked":true,"tool_call_count":100,"session_duration_ms":600000,"edit_files":["b.rs","c.rs"],"lint_invoked":true,"typecheck_invoked":false,"bash_error_rate":0.1}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-08T12:00:00.000Z","sessionId":"s3","project":"test-project","metrics":{{"read_before_edit_ratio":15,"doom_loop_count":2,"test_invoked":true,"build_invoked":false,"tool_call_count":150,"session_duration_ms":900000,"edit_files":["d.rs"],"lint_invoked":false,"typecheck_invoked":true,"bash_error_rate":0.2}}}}"#).unwrap();

    let config = Config {
        log_dir: dir.to_str().unwrap().to_string(),
        max_file_size_mb: 10,
    };
    (dir, config)
}

#[test]
fn report_reads_metrics_correctly() {
    let (dir, config) = setup_metrics("report_reads");
    let log_dir = config.log_dir_expanded();

    let entries = read_metrics(&log_dir, Some("test-project"), "2026-04-08", "2026-04-08").unwrap();
    assert_eq!(entries.len(), 3);

    // 지표 값 확인
    let tool_counts: Vec<u32> = entries.iter().map(|e| e.metrics.tool_call_count).collect();
    assert_eq!(tool_counts, vec![50, 100, 150]);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn report_handles_old_schema_entries() {
    let dir = std::env::temp_dir().join("seogi_integ_report_old_schema");
    let _ = std::fs::remove_dir_all(&dir);
    let metrics_dir = dir.join("test-project").join("metrics");
    std::fs::create_dir_all(&metrics_dir).unwrap();

    let path = metrics_dir.join("2026-04-07.jsonl");
    let mut f = std::fs::File::create(&path).unwrap();
    // old schema without lint_invoked, typecheck_invoked, bash_error_rate
    writeln!(f, r#"{{"timestamp":"2026-04-07T10:00:00.000Z","sessionId":"s1","project":"test-project","metrics":{{"read_before_edit_ratio":3,"doom_loop_count":0,"test_invoked":true,"build_invoked":false,"tool_call_count":20,"session_duration_ms":60000,"edit_files":[]}}}}"#).unwrap();

    let config = Config {
        log_dir: dir.to_str().unwrap().to_string(),
        max_file_size_mb: 10,
    };
    let log_dir = config.log_dir_expanded();

    let entries = read_metrics(&log_dir, Some("test-project"), "2026-04-07", "2026-04-07").unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].metrics.lint_invoked.is_none());
    assert!(entries[0].metrics.bash_error_rate.is_none());

    std::fs::remove_dir_all(&dir).ok();
}
