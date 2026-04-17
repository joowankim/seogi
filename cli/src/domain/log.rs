use std::fmt;

/// 도구 사용 기록.
///
/// Claude Code `PostToolUse` 훅에서 수집된 도구 호출 정보를 표현한다.
/// `tool_uses` 테이블의 한 행에 대응한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUse {
    id: String,
    session_id: String,
    project: String,
    project_path: String,
    tool_name: String,
    tool_input: String,
    duration_ms: i64,
    timestamp: i64,
}

impl ToolUse {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        session_id: String,
        project: String,
        project_path: String,
        tool_name: String,
        tool_input: String,
        duration_ms: i64,
        timestamp: i64,
    ) -> Self {
        Self {
            id,
            session_id,
            project,
            project_path,
            tool_name,
            tool_input,
            duration_ms,
            timestamp,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    #[must_use]
    pub fn project_path(&self) -> &str {
        &self.project_path
    }

    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    #[must_use]
    pub fn tool_input(&self) -> &str {
        &self.tool_input
    }

    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.duration_ms
    }

    #[must_use]
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

impl fmt::Display for ToolUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} ({})", self.session_id, self.tool_name, self.id)
    }
}

/// `cwd` 경로에서 프로젝트 이름을 추출한다.
///
/// 경로의 마지막 컴포넌트를 반환한다. 루트(`/`) 등 컴포넌트가 없으면 `"unknown"`을 반환한다.
#[must_use]
pub fn extract_project_from_cwd(cwd: &str) -> String {
    std::path::Path::new(cwd).file_name().map_or_else(
        || "unknown".to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tool_use() -> ToolUse {
        ToolUse::new(
            "abc123".to_string(),
            "sess-1".to_string(),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            r#"{"command":"ls"}"#.to_string(),
            0,
            1_713_000_000_000,
        )
    }

    #[test]
    fn tool_use_creation_and_getters() {
        let tu = sample_tool_use();

        assert_eq!(tu.id(), "abc123");
        assert_eq!(tu.session_id(), "sess-1");
        assert_eq!(tu.project(), "seogi");
        assert_eq!(tu.project_path(), "/Users/kim/projects/seogi");
        assert_eq!(tu.tool_name(), "Bash");
        assert_eq!(tu.tool_input(), r#"{"command":"ls"}"#);
        assert_eq!(tu.duration_ms(), 0);
        assert_eq!(tu.timestamp(), 1_713_000_000_000);

        // Clone + PartialEq
        assert_eq!(tu.clone(), tu);
    }

    #[test]
    fn extract_project_from_normal_path() {
        assert_eq!(
            extract_project_from_cwd("/Users/kim/projects/seogi"),
            "seogi"
        );
        assert_eq!(extract_project_from_cwd("/home/user/my-app"), "my-app");
    }

    #[test]
    fn extract_project_from_root_path() {
        assert_eq!(extract_project_from_cwd("/"), "unknown");
    }

    #[test]
    fn tool_use_display() {
        let tu = sample_tool_use();
        assert_eq!(format!("{tu}"), "[sess-1] Bash (abc123)");
    }
}
