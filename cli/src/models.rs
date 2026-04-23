use serde::{Deserialize, Serialize};

/// raw 로그의 tool 필드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub failed: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
}

/// raw 로그 엔트리
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub session_id: String,
    #[serde(alias = "project")]
    pub workspace: String,
    #[serde(alias = "projectPath")]
    pub workspace_path: String,
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool: Option<ToolInfo>,
}

/// 하니스 변경 이력
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub timestamp: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_compact_log_entry() {
        let json = r#"{"timestamp":"2026-04-07T11:20:56.000Z","sessionId":"abc123","project":"locs","projectPath":"/projects/locs","role":"assistant","tool":{"name":"Bash","duration_ms":100,"input":{"command":"ls"}}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.session_id, "abc123");
        assert_eq!(entry.tool.as_ref().unwrap().name, "Bash");
    }

    #[test]
    fn deserialize_log_entry_without_tool() {
        let json = r#"{"timestamp":"2026-04-07T11:18:05.000Z","sessionId":"abc123","project":"locs","projectPath":"/projects/locs","role":"system","content":"[stop] end_turn","tool":null}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert!(entry.tool.is_none());
        assert_eq!(entry.content.as_deref(), Some("[stop] end_turn"));
    }

    #[test]
    fn deserialize_failed_tool_entry() {
        let json = r#"{"timestamp":"2026-04-07T12:00:00.000Z","sessionId":"abc123","project":"locs","projectPath":"/projects/locs","role":"assistant","tool":{"name":"Bash","failed":true,"error":"exit code 1"}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        let tool = entry.tool.unwrap();
        assert_eq!(tool.failed, Some(true));
        assert_eq!(tool.error.as_deref(), Some("exit code 1"));
    }

    #[test]
    fn serialize_changelog_entry() {
        let entry = ChangelogEntry {
            timestamp: "2026-04-15T09:00:00.000Z".to_string(),
            description: "test change".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test change"));
        assert!(json.contains("2026-04-15"));
    }
}
