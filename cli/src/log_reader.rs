use anyhow::{Context, Result};
use std::path::Path;

use crate::models::LogEntry;

/// 프로젝트 로그 디렉토리에서 해당 세션의 모든 엔트리를 추출한다.
///
/// - `log_dir/<project>/*.jsonl` 파일을 순회
/// - `metrics/` 하위 디렉토리는 제외
/// - pretty-printed JSON과 compact JSON 모두 파싱
/// - 타임스탬프 기준 정렬 후 반환
///
/// # Errors
///
/// Returns an error if the log directory cannot be read or a log file cannot be parsed.
pub fn read_session_logs(log_dir: &Path, project: &str, session_id: &str) -> Result<Vec<LogEntry>> {
    let project_dir = log_dir.join(project);
    if !project_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    for entry in std::fs::read_dir(&project_dir)
        .with_context(|| format!("failed to read directory {}", project_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        // metrics 디렉토리 제외
        if path.is_dir() {
            continue;
        }

        // .jsonl 파일만 처리
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            let parsed = parse_jsonl_content(&content);
            for log_entry in parsed {
                if log_entry.session_id == session_id {
                    entries.push(log_entry);
                }
            }
        }
    }

    // 타임스탬프 정렬
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Ok(entries)
}

/// JSONL 콘텐츠를 파싱한다 (compact 한 줄 또는 pretty-printed 멀티라인 모두 지원).
fn parse_jsonl_content(content: &str) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return entries;
    }

    // compact JSONL 시도: 첫 줄이 완전한 JSON인지 확인
    let first_line = trimmed.lines().next().unwrap_or("");
    if first_line.starts_with('{') && first_line.ends_with('}') {
        // compact 모드: 한 줄씩 파싱
        for line in trimmed.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
                entries.push(entry);
            }
        }
    } else {
        // pretty-printed 모드: json stream 파싱
        let decoder = serde_json::Deserializer::from_str(trimmed).into_iter::<LogEntry>();
        for entry in decoder.flatten() {
            entries.push(entry);
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_compact_log(session_id: &str, tool_name: Option<&str>) -> String {
        match tool_name {
            Some(name) => format!(
                r#"{{"timestamp":"2026-04-07T11:20:56.000Z","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"{name}","duration_ms":100}}}}"#
            ),
            None => format!(
                r#"{{"timestamp":"2026-04-07T11:20:56.000Z","sessionId":"{session_id}","project":"test","projectPath":"/test","role":"system","content":"[stop]","tool":null}}"#
            ),
        }
    }

    fn make_pretty_log(session_id: &str) -> String {
        format!(
            r#"{{
  "timestamp": "2026-04-07T11:20:56.000Z",
  "sessionId": "{session_id}",
  "project": "test",
  "projectPath": "/test",
  "role": "assistant",
  "tool": {{
    "name": "Bash",
    "duration_ms": 100
  }}
}}"#
        )
    }

    #[test]
    fn parse_compact_jsonl() {
        let content = format!(
            "{}\n{}\n",
            make_compact_log("s1", Some("Bash")),
            make_compact_log("s1", Some("Read"))
        );
        let entries = parse_jsonl_content(&content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_pretty_printed_json() {
        let content = format!("{}\n{}", make_pretty_log("s1"), make_pretty_log("s1"));
        let entries = parse_jsonl_content(&content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_empty_content() {
        let entries = parse_jsonl_content("");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_whitespace_content() {
        let entries = parse_jsonl_content("  \n  \n  ");
        assert!(entries.is_empty());
    }

    #[test]
    fn session_id_filtering() {
        let dir = std::env::temp_dir().join("seogi_test_session_filter");
        let project_dir = dir.join("test-project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let log_path = project_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&log_path).unwrap();
        writeln!(f, "{}", make_compact_log("session-a", Some("Bash"))).unwrap();
        writeln!(f, "{}", make_compact_log("session-b", Some("Read"))).unwrap();
        writeln!(f, "{}", make_compact_log("session-a", Some("Edit"))).unwrap();

        let entries = read_session_logs(&dir, "test-project", "session-a").unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.session_id == "session-a"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn metrics_directory_excluded() {
        let dir = std::env::temp_dir().join("seogi_test_metrics_excluded");
        let project_dir = dir.join("test-project");
        let metrics_dir = project_dir.join("metrics");
        std::fs::create_dir_all(&metrics_dir).unwrap();

        // raw 로그
        let log_path = project_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&log_path).unwrap();
        writeln!(f, "{}", make_compact_log("s1", Some("Bash"))).unwrap();

        // metrics 파일 (다른 형식이지만 디렉토리이므로 무시됨)
        let metrics_path = metrics_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&metrics_path).unwrap();
        writeln!(f, r#"{{"not":"a log entry"}}"#).unwrap();

        let entries = read_session_logs(&dir, "test-project", "s1").unwrap();
        assert_eq!(entries.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_project_directory() {
        let entries = read_session_logs(Path::new("/nonexistent"), "nope", "s1").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn entries_sorted_by_timestamp() {
        let dir = std::env::temp_dir().join("seogi_test_sorted");
        let project_dir = dir.join("test-project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let log_path = project_dir.join("2026-04-07.jsonl");
        let mut f = std::fs::File::create(&log_path).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-04-07T12:00:00.000Z","sessionId":"s1","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"B","duration_ms":0}}}}"#).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-04-07T11:00:00.000Z","sessionId":"s1","project":"test","projectPath":"/test","role":"assistant","tool":{{"name":"A","duration_ms":0}}}}"#).unwrap();

        let entries = read_session_logs(&dir, "test-project", "s1").unwrap();
        assert_eq!(entries[0].tool.as_ref().unwrap().name, "A");
        assert_eq!(entries[1].tool.as_ref().unwrap().name, "B");

        std::fs::remove_dir_all(&dir).ok();
    }
}
