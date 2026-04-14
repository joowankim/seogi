use std::collections::HashMap;

use chrono::DateTime;
use regex::Regex;

use crate::models::{LogEntry, SessionMetrics, SessionMetricsEntry};

/// 세션 로그 엔트리에서 프록시 지표 10개를 계산한다.
#[must_use]
pub fn compute_metrics(
    entries: &[LogEntry],
    session_id: &str,
    project: &str,
) -> SessionMetricsEntry {
    let tool_calls: Vec<&LogEntry> = entries.iter().filter(|e| e.tool.is_some()).collect();

    let metrics = SessionMetrics {
        read_before_edit_ratio: calc_read_before_edit(&tool_calls),
        doom_loop_count: calc_doom_loop_count(&tool_calls),
        test_invoked: calc_invoked(&tool_calls, &test_pattern()),
        build_invoked: calc_invoked(&tool_calls, &build_pattern()),
        lint_invoked: Some(calc_invoked(&tool_calls, &lint_pattern())),
        typecheck_invoked: Some(calc_invoked(&tool_calls, &typecheck_pattern())),
        tool_call_count: tool_calls.len() as u32,
        session_duration_ms: calc_session_duration(entries),
        edit_files: calc_edit_files(&tool_calls),
        bash_error_rate: Some(calc_bash_error_rate(entries)),
    };

    SessionMetricsEntry {
        timestamp: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        session_id: session_id.to_string(),
        project: project.to_string(),
        metrics,
    }
}

fn tool_name(entry: &LogEntry) -> &str {
    entry.tool.as_ref().map_or("", |t| t.name.as_str())
}

fn bash_command(entry: &LogEntry) -> String {
    entry
        .tool
        .as_ref()
        .and_then(|t| t.input.as_ref())
        .and_then(|input| input.get("command"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn file_path(entry: &LogEntry) -> Option<String> {
    entry
        .tool
        .as_ref()
        .and_then(|t| t.input.as_ref())
        .and_then(|input| input.get("file_path"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn is_failed(entry: &LogEntry) -> bool {
    entry.tool.as_ref().and_then(|t| t.failed).unwrap_or(false)
}

/// 1. 첫 Edit/Write 전 Read/Grep/Glob 호출 수
fn calc_read_before_edit(tool_calls: &[&LogEntry]) -> u32 {
    let read_tools = ["Read", "Grep", "Glob"];
    let edit_tools = ["Edit", "Write"];

    let first_edit_idx = tool_calls
        .iter()
        .position(|e| edit_tools.contains(&tool_name(e)));

    let end = first_edit_idx.unwrap_or(tool_calls.len());

    tool_calls[..end]
        .iter()
        .filter(|e| read_tools.contains(&tool_name(e)))
        .count() as u32
}

/// 2. 동일 파일 Edit 5회 이상 발생 횟수
fn calc_doom_loop_count(tool_calls: &[&LogEntry]) -> u32 {
    let mut file_counts: HashMap<String, u32> = HashMap::new();

    for entry in tool_calls {
        if tool_name(entry) == "Edit"
            && let Some(path) = file_path(entry)
        {
            *file_counts.entry(path).or_insert(0) += 1;
        }
    }

    file_counts.values().filter(|&&count| count >= 5).count() as u32
}

/// 3-6. Bash command 패턴 매칭으로 bool 지표 계산
fn calc_invoked(tool_calls: &[&LogEntry], pattern: &Regex) -> bool {
    tool_calls
        .iter()
        .filter(|e| tool_name(e) == "Bash")
        .any(|e| pattern.is_match(&bash_command(e)))
}

fn test_pattern() -> Regex {
    Regex::new(r"(?i)\b(test|vitest|playwright|jest|pytest|mocha|karma)\b").unwrap()
}

fn build_pattern() -> Regex {
    Regex::new(r"(?i)\b(build|tsc|webpack|vite build|esbuild|rollup)\b").unwrap()
}

fn lint_pattern() -> Regex {
    Regex::new(r"(?i)\b(lint|eslint|prettier|ruff|biome)\b").unwrap()
}

fn typecheck_pattern() -> Regex {
    Regex::new(r"(?i)\b(tsc\s+--noEmit|mypy|pyright)\b").unwrap()
}

/// 8. 첫 엔트리 ~ 마지막 엔트리 시간차 (ms)
fn calc_session_duration(entries: &[LogEntry]) -> i64 {
    if entries.len() <= 1 {
        return 0;
    }

    let parse_ts = |ts: &str| -> Option<DateTime<chrono::Utc>> {
        DateTime::parse_from_rfc3339(ts).ok().map(|dt| dt.to_utc())
    };

    let first = parse_ts(&entries.first().unwrap().timestamp);
    let last = parse_ts(&entries.last().unwrap().timestamp);

    match (first, last) {
        (Some(f), Some(l)) => (l - f).num_milliseconds(),
        _ => 0,
    }
}

/// 9. Edit/Write한 고유 파일 목록
fn calc_edit_files(tool_calls: &[&LogEntry]) -> Vec<String> {
    let mut files: Vec<String> = tool_calls
        .iter()
        .filter(|e| {
            let name = tool_name(e);
            name == "Edit" || name == "Write"
        })
        .filter_map(|e| file_path(e))
        .collect();

    files.sort();
    files.dedup();
    files
}

/// 10. Bash 실패 비율
fn calc_bash_error_rate(entries: &[LogEntry]) -> f64 {
    let bash_entries: Vec<&LogEntry> = entries
        .iter()
        .filter(|e| e.tool.as_ref().is_some_and(|t| t.name == "Bash"))
        .collect();

    if bash_entries.is_empty() {
        return 0.0;
    }

    let failed = bash_entries.iter().filter(|e| is_failed(e)).count();
    failed as f64 / bash_entries.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ToolInfo;

    fn make_entry(tool_name: &str, timestamp: &str) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            session_id: "s1".to_string(),
            project: "test".to_string(),
            project_path: "/test".to_string(),
            role: "assistant".to_string(),
            content: None,
            tool: Some(ToolInfo {
                name: tool_name.to_string(),
                duration_ms: Some(0),
                input: None,
                failed: None,
                error: None,
            }),
        }
    }

    fn make_bash_entry(command: &str, timestamp: &str) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            session_id: "s1".to_string(),
            project: "test".to_string(),
            project_path: "/test".to_string(),
            role: "assistant".to_string(),
            content: None,
            tool: Some(ToolInfo {
                name: "Bash".to_string(),
                duration_ms: Some(0),
                input: Some(serde_json::json!({"command": command})),
                failed: None,
                error: None,
            }),
        }
    }

    fn make_edit_entry(fp: &str, timestamp: &str) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            session_id: "s1".to_string(),
            project: "test".to_string(),
            project_path: "/test".to_string(),
            role: "assistant".to_string(),
            content: None,
            tool: Some(ToolInfo {
                name: "Edit".to_string(),
                duration_ms: Some(0),
                input: Some(serde_json::json!({"file_path": fp})),
                failed: None,
                error: None,
            }),
        }
    }

    fn make_failed_bash(timestamp: &str) -> LogEntry {
        LogEntry {
            timestamp: timestamp.to_string(),
            session_id: "s1".to_string(),
            project: "test".to_string(),
            project_path: "/test".to_string(),
            role: "assistant".to_string(),
            content: None,
            tool: Some(ToolInfo {
                name: "Bash".to_string(),
                duration_ms: None,
                input: None,
                failed: Some(true),
                error: Some("exit code 1".to_string()),
            }),
        }
    }

    #[test]
    fn test_read_before_edit_ratio() {
        let entries = [
            make_entry("Read", "2026-04-07T11:00:00.000Z"),
            make_entry("Grep", "2026-04-07T11:01:00.000Z"),
            make_entry("Read", "2026-04-07T11:02:00.000Z"),
            make_edit_entry("a.rs", "2026-04-07T11:03:00.000Z"),
        ];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert_eq!(calc_read_before_edit(&refs), 3);
    }

    #[test]
    fn test_read_before_edit_no_edit() {
        let entries = [
            make_entry("Read", "2026-04-07T11:00:00.000Z"),
            make_entry("Glob", "2026-04-07T11:01:00.000Z"),
            make_entry("Read", "2026-04-07T11:02:00.000Z"),
        ];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert_eq!(calc_read_before_edit(&refs), 3);
    }

    #[test]
    fn test_doom_loop_count_above_threshold() {
        let mut entries = Vec::new();
        for i in 0..6 {
            entries.push(make_edit_entry(
                "same.rs",
                &format!("2026-04-07T11:{i:02}:00.000Z"),
            ));
        }
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert_eq!(calc_doom_loop_count(&refs), 1);
    }

    #[test]
    fn test_doom_loop_count_below_threshold() {
        let mut entries = Vec::new();
        for i in 0..4 {
            entries.push(make_edit_entry(
                "same.rs",
                &format!("2026-04-07T11:{i:02}:00.000Z"),
            ));
        }
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert_eq!(calc_doom_loop_count(&refs), 0);
    }

    #[test]
    fn test_test_invoked_true() {
        let entries = [make_bash_entry(
            "npm run pytest",
            "2026-04-07T11:00:00.000Z",
        )];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert!(calc_invoked(&refs, &test_pattern()));
    }

    #[test]
    fn test_test_invoked_false() {
        let entries = [make_bash_entry("ls -la", "2026-04-07T11:00:00.000Z")];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert!(!calc_invoked(&refs, &test_pattern()));
    }

    #[test]
    fn test_lint_invoked() {
        let entries = [make_bash_entry("npx eslint .", "2026-04-07T11:00:00.000Z")];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert!(calc_invoked(&refs, &lint_pattern()));
    }

    #[test]
    fn test_typecheck_invoked() {
        let entries = [make_bash_entry(
            "npx tsc --noEmit",
            "2026-04-07T11:00:00.000Z",
        )];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        assert!(calc_invoked(&refs, &typecheck_pattern()));
    }

    #[test]
    fn test_bash_error_rate() {
        let entries = vec![
            make_bash_entry("ls", "2026-04-07T11:00:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:01:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:02:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:03:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:04:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:05:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:06:00.000Z"),
            make_bash_entry("ls", "2026-04-07T11:07:00.000Z"),
            make_failed_bash("2026-04-07T11:08:00.000Z"),
            make_failed_bash("2026-04-07T11:09:00.000Z"),
        ];
        let rate = calc_bash_error_rate(&entries);
        assert!((rate - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bash_error_rate_no_bash() {
        let entries = vec![make_entry("Read", "2026-04-07T11:00:00.000Z")];
        let rate = calc_bash_error_rate(&entries);
        assert!((rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_edit_files() {
        let entries = [
            make_edit_entry("a.rs", "2026-04-07T11:00:00.000Z"),
            make_edit_entry("b.rs", "2026-04-07T11:01:00.000Z"),
            make_edit_entry("a.rs", "2026-04-07T11:02:00.000Z"),
        ];
        let refs: Vec<&LogEntry> = entries.iter().collect();
        let files = calc_edit_files(&refs);
        assert_eq!(files, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn test_session_duration() {
        let entries = vec![
            make_entry("Read", "2026-04-07T11:00:00.000Z"),
            make_entry("Edit", "2026-04-07T11:05:00.000Z"),
        ];
        let duration = calc_session_duration(&entries);
        assert_eq!(duration, 300_000); // 5 minutes in ms
    }

    #[test]
    fn test_session_duration_single_entry() {
        let entries = vec![make_entry("Read", "2026-04-07T11:00:00.000Z")];
        assert_eq!(calc_session_duration(&entries), 0);
    }

    #[test]
    fn test_empty_session() {
        let result = compute_metrics(&[], "s1", "test");
        assert_eq!(result.metrics.read_before_edit_ratio, 0);
        assert_eq!(result.metrics.doom_loop_count, 0);
        assert!(!result.metrics.test_invoked);
        assert_eq!(result.metrics.tool_call_count, 0);
        assert_eq!(result.metrics.session_duration_ms, 0);
        assert!(result.metrics.edit_files.is_empty());
        assert_eq!(result.metrics.bash_error_rate, Some(0.0));
    }
}
