use sha2::{Digest, Sha256};

use crate::domain::log::{ToolFailure, ToolUse};
use crate::domain::value::{Ms, SessionId, Timestamp};
use crate::models::LogEntry;

/// 마이그레이션 변환 결과.
#[derive(Debug)]
pub enum MigratedRecord {
    Use(ToolUse),
    Failure(ToolFailure),
}

/// 마이그레이션 요약.
#[derive(Debug, Default)]
pub struct MigrateSummary {
    pub tool_uses: u32,
    pub tool_failures: u32,
    pub skipped: u32,
    pub files: u32,
}

/// `LogEntry`를 `ToolUse` 또는 `ToolFailure`로 변환한다.
///
/// `tool`이 없는 엔트리는 `None`을 반환한다.
/// RFC3339 timestamp 파싱 실패 시 `None`을 반환한다.
#[must_use]
pub fn convert_entry(entry: &LogEntry) -> Option<MigratedRecord> {
    let tool = entry.tool.as_ref()?;
    let ts = parse_rfc3339_to_millis(&entry.timestamp)?;
    let id = content_based_id(&entry.session_id, &entry.timestamp, &tool.name);
    let tool_input_str = tool
        .input
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())
        .unwrap_or_default();

    if tool.failed == Some(true) {
        let error = tool.error.clone().unwrap_or_default();
        Some(MigratedRecord::Failure(ToolFailure::new(
            id,
            SessionId::new(&entry.session_id),
            entry.workspace.clone(),
            entry.workspace_path.clone(),
            tool.name.clone(),
            error,
            Timestamp::new(ts),
        )))
    } else {
        let duration = tool
            .duration_ms
            .map_or(Ms::zero(), |d| Ms::new(d.cast_signed()));
        Some(MigratedRecord::Use(ToolUse::new(
            id,
            SessionId::new(&entry.session_id),
            entry.workspace.clone(),
            entry.workspace_path.clone(),
            tool.name.clone(),
            tool_input_str,
            duration,
            Timestamp::new(ts),
        )))
    }
}

/// 콘텐츠 기반 결정론적 ID를 생성한다.
///
/// `SHA-256(session_id + timestamp + tool_name)`의 앞 32자 hex.
#[must_use]
pub fn content_based_id(session_id: &str, timestamp: &str, tool_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_id.as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(tool_name.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16])
}

fn parse_rfc3339_to_millis(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ToolInfo;

    fn make_entry(tool_name: &str, failed: Option<bool>) -> LogEntry {
        LogEntry {
            timestamp: "2026-04-07T11:00:00.000Z".to_string(),
            session_id: "sess-1".to_string(),
            workspace: "test".to_string(),
            workspace_path: "/test".to_string(),
            role: "assistant".to_string(),
            content: None,
            tool: Some(ToolInfo {
                name: tool_name.to_string(),
                duration_ms: Some(150),
                input: Some(serde_json::json!({"command": "ls"})),
                failed,
                error: if failed == Some(true) {
                    Some("exit 1".to_string())
                } else {
                    None
                },
            }),
        }
    }

    #[test]
    fn test_convert_tool_use() {
        let entry = make_entry("Bash", None);
        let result = convert_entry(&entry).unwrap();
        match result {
            MigratedRecord::Use(tu) => {
                assert_eq!(tu.tool_name(), "Bash");
                assert_eq!(tu.session_id().as_str(), "sess-1");
                assert_eq!(tu.tool_input(), r#"{"command":"ls"}"#);
                assert_eq!(tu.duration(), Ms::new(150));
                assert!(tu.timestamp().value() > 0);
            }
            MigratedRecord::Failure(_) => panic!("expected Use"),
        }
    }

    #[test]
    fn test_convert_tool_failure() {
        let entry = make_entry("Bash", Some(true));
        let result = convert_entry(&entry).unwrap();
        match result {
            MigratedRecord::Failure(tf) => {
                assert_eq!(tf.tool_name(), "Bash");
                assert_eq!(tf.error(), "exit 1");
            }
            MigratedRecord::Use(_) => panic!("expected Failure"),
        }
    }

    #[test]
    fn test_convert_no_tool() {
        let entry = LogEntry {
            timestamp: "2026-04-07T11:00:00.000Z".to_string(),
            session_id: "sess-1".to_string(),
            workspace: "test".to_string(),
            workspace_path: "/test".to_string(),
            role: "system".to_string(),
            content: Some("[stop] end_turn".to_string()),
            tool: None,
        };
        assert!(convert_entry(&entry).is_none());
    }

    #[test]
    fn test_content_based_id_deterministic() {
        let id1 = content_based_id("sess-1", "2026-04-07T11:00:00Z", "Bash");
        let id2 = content_based_id("sess-1", "2026-04-07T11:00:00Z", "Bash");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_duration_none_fallback() {
        let mut entry = make_entry("Read", None);
        entry.tool.as_mut().unwrap().duration_ms = None;
        let result = convert_entry(&entry).unwrap();
        match result {
            MigratedRecord::Use(tu) => assert_eq!(tu.duration(), Ms::zero()),
            MigratedRecord::Failure(_) => panic!("expected Use"),
        }
    }
}
