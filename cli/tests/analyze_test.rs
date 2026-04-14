use std::io::Write;

use seogi::commands::analyze;
use seogi::config::Config;
use seogi::models::SessionMetricsEntry;

fn setup_logs(name: &str) -> (std::path::PathBuf, Config) {
    let dir = std::env::temp_dir().join(format!("seogi_integ_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let project_dir = dir.join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    let log_path = project_dir.join("2026-04-07.jsonl");
    let mut f = std::fs::File::create(&log_path).unwrap();

    // Read 2번 → Edit 1번 → Bash(test) 성공 → Bash 실패
    writeln!(f, r#"{{"timestamp":"2026-04-07T11:00:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Read","duration_ms":50}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-07T11:01:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Grep","duration_ms":30}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-07T11:02:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Edit","duration_ms":100,"input":{{"file_path":"src/main.rs"}}}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-07T11:03:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Bash","duration_ms":200,"input":{{"command":"cargo test"}}}}}}"#).unwrap();
    writeln!(f, r#"{{"timestamp":"2026-04-07T11:04:00.000Z","sessionId":"s1","project":"test-project","projectPath":"/test","role":"assistant","tool":{{"name":"Bash","failed":true,"error":"exit code 1"}}}}"#).unwrap();

    let config = Config {
        log_dir: dir.to_str().unwrap().to_string(),
        max_file_size_mb: 10,
    };
    (dir, config)
}

#[test]
fn analyze_produces_correct_metrics() {
    let (dir, config) = setup_logs("analyze_correct");

    analyze::run(&config, "test-project", "s1").unwrap();

    // metrics 파일 확인
    let metrics_dir = dir.join("test-project").join("metrics");
    let files: Vec<_> = std::fs::read_dir(&metrics_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();
    assert_eq!(files.len(), 1);

    let content = std::fs::read_to_string(files[0].path()).unwrap();
    let entry: SessionMetricsEntry = serde_json::from_str(content.trim()).unwrap();

    assert_eq!(entry.session_id, "s1");
    assert_eq!(entry.project, "test-project");
    assert_eq!(entry.metrics.read_before_edit_ratio, 2); // Read + Grep before Edit
    assert_eq!(entry.metrics.doom_loop_count, 0);
    assert!(entry.metrics.test_invoked); // "cargo test"
    assert!(!entry.metrics.build_invoked);
    assert_eq!(entry.metrics.tool_call_count, 5);
    assert_eq!(entry.metrics.edit_files, vec!["src/main.rs"]);
    assert_eq!(entry.metrics.lint_invoked, Some(false));
    assert_eq!(entry.metrics.typecheck_invoked, Some(false));
    // bash_error_rate: 1 failed / 2 total = 0.5
    assert!((entry.metrics.bash_error_rate.unwrap() - 0.5).abs() < f64::EPSILON);
    // session_duration: 11:00 ~ 11:04 = 240000ms
    assert_eq!(entry.metrics.session_duration_ms, 240_000);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn analyze_with_pretty_printed_logs() {
    let dir = std::env::temp_dir().join("seogi_integ_analyze_pretty");
    let _ = std::fs::remove_dir_all(&dir);
    let project_dir = dir.join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    let log_path = project_dir.join("2026-04-07.jsonl");
    let mut f = std::fs::File::create(&log_path).unwrap();
    // pretty-printed format
    write!(
        f,
        r#"{{
  "timestamp": "2026-04-07T11:00:00.000Z",
  "sessionId": "s1",
  "project": "test-project",
  "projectPath": "/test",
  "role": "assistant",
  "tool": {{
    "name": "Read",
    "duration_ms": 50
  }}
}}
{{
  "timestamp": "2026-04-07T11:01:00.000Z",
  "sessionId": "s1",
  "project": "test-project",
  "projectPath": "/test",
  "role": "assistant",
  "tool": {{
    "name": "Edit",
    "duration_ms": 100,
    "input": {{"file_path": "lib.rs"}}
  }}
}}"#
    )
    .unwrap();

    let config = Config {
        log_dir: dir.to_str().unwrap().to_string(),
        max_file_size_mb: 10,
    };

    analyze::run(&config, "test-project", "s1").unwrap();

    let metrics_dir = dir.join("test-project").join("metrics");
    let files: Vec<_> = std::fs::read_dir(&metrics_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();
    assert_eq!(files.len(), 1);

    let content = std::fs::read_to_string(files[0].path()).unwrap();
    let entry: SessionMetricsEntry = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(entry.metrics.read_before_edit_ratio, 1);
    assert_eq!(entry.metrics.edit_files, vec!["lib.rs"]);

    std::fs::remove_dir_all(&dir).ok();
}
