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
    pub project: String,
    pub project_path: String,
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool: Option<ToolInfo>,
}

/// 세션 메트릭 (분석기 출력)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetricsEntry {
    pub timestamp: String,
    pub session_id: String,
    pub project: String,
    pub metrics: SessionMetrics,
}

/// 프록시 지표 10개
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub read_before_edit_ratio: u32,
    pub doom_loop_count: u32,
    pub test_invoked: bool,
    pub build_invoked: bool,
    pub tool_call_count: u32,
    pub session_duration_ms: i64,
    pub edit_files: Vec<String>,
    #[serde(default)]
    pub lint_invoked: Option<bool>,
    #[serde(default)]
    pub typecheck_invoked: Option<bool>,
    #[serde(default)]
    pub bash_error_rate: Option<f64>,
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
    fn deserialize_metrics_with_all_fields() {
        let json = r#"{"timestamp":"2026-04-08T12:00:00.000Z","sessionId":"abc","project":"locs","metrics":{"read_before_edit_ratio":5,"doom_loop_count":0,"test_invoked":true,"build_invoked":false,"tool_call_count":42,"session_duration_ms":180000,"edit_files":["a.rs","b.rs"],"lint_invoked":false,"typecheck_invoked":true,"bash_error_rate":0.1}}"#;
        let entry: SessionMetricsEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.metrics.tool_call_count, 42);
        assert_eq!(entry.metrics.lint_invoked, Some(false));
        assert_eq!(entry.metrics.bash_error_rate, Some(0.1));
    }

    #[test]
    fn deserialize_metrics_without_new_fields() {
        let json = r#"{"timestamp":"2026-04-07T12:00:00.000Z","sessionId":"abc","project":"locs","metrics":{"read_before_edit_ratio":3,"doom_loop_count":1,"test_invoked":false,"build_invoked":false,"tool_call_count":10,"session_duration_ms":5000,"edit_files":[]}}"#;
        let entry: SessionMetricsEntry = serde_json::from_str(json).unwrap();
        assert!(entry.metrics.lint_invoked.is_none());
        assert!(entry.metrics.typecheck_invoked.is_none());
        assert!(entry.metrics.bash_error_rate.is_none());
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
