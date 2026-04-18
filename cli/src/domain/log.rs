use std::fmt;

use super::value::{Ms, SessionId, Timestamp};

/// 도구 사용 기록.
///
/// Claude Code `PostToolUse` 훅에서 수집된 도구 호출 정보를 표현한다.
/// `tool_uses` 테이블의 한 행에 대응한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUse {
    id: String,
    session_id: SessionId,
    project: String,
    project_path: String,
    tool_name: String,
    tool_input: String,
    duration: Ms,
    timestamp: Timestamp,
}

impl ToolUse {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        session_id: SessionId,
        project: String,
        project_path: String,
        tool_name: String,
        tool_input: String,
        duration: Ms,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            id,
            session_id,
            project,
            project_path,
            tool_name,
            tool_input,
            duration,
            timestamp,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn session_id(&self) -> &SessionId {
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
    pub fn duration(&self) -> Ms {
        self.duration
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl fmt::Display for ToolUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} ({})", self.session_id, self.tool_name, self.id)
    }
}

/// 도구 실패 기록.
///
/// Claude Code `PostToolUseFailure` 훅에서 수집된 도구 호출 실패 정보를 표현한다.
/// `tool_failures` 테이블의 한 행에 대응한다.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolFailure {
    id: String,
    session_id: SessionId,
    project: String,
    project_path: String,
    tool_name: String,
    error: String,
    timestamp: Timestamp,
}

impl ToolFailure {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        session_id: SessionId,
        project: String,
        project_path: String,
        tool_name: String,
        error: String,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            id,
            session_id,
            project,
            project_path,
            tool_name,
            error,
            timestamp,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn session_id(&self) -> &SessionId {
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
    pub fn error(&self) -> &str {
        &self.error
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl fmt::Display for ToolFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} FAILED ({})",
            self.session_id, self.tool_name, self.id
        )
    }
}

/// 시스템 이벤트 기록.
///
/// Claude Code `Notification` 또는 `Stop` 훅에서 수집된 이벤트 정보를 표현한다.
/// `system_events` 테이블의 한 행에 대응한다.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemEvent {
    id: String,
    session_id: SessionId,
    project: String,
    project_path: String,
    event_type: String,
    content: String,
    timestamp: Timestamp,
}

impl SystemEvent {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        session_id: SessionId,
        project: String,
        project_path: String,
        event_type: String,
        content: String,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            id,
            session_id,
            project,
            project_path,
            event_type,
            content,
            timestamp,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn session_id(&self) -> &SessionId {
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
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl fmt::Display for SystemEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} ({})", self.session_id, self.event_type, self.id)
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
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            r#"{"command":"ls"}"#.to_string(),
            Ms::zero(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn tool_use_creation_and_getters() {
        let tu = sample_tool_use();

        assert_eq!(tu.id(), "abc123");
        assert_eq!(tu.session_id().as_str(), "sess-1");
        assert_eq!(tu.project(), "seogi");
        assert_eq!(tu.project_path(), "/Users/kim/projects/seogi");
        assert_eq!(tu.tool_name(), "Bash");
        assert_eq!(tu.tool_input(), r#"{"command":"ls"}"#);
        assert_eq!(tu.duration(), Ms::zero());
        assert_eq!(tu.timestamp(), Timestamp::new(1_713_000_000_000));

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

    fn sample_tool_failure() -> ToolFailure {
        ToolFailure::new(
            "fail123".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Bash".to_string(),
            "Permission denied".to_string(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn tool_failure_creation_and_getters() {
        let tf = sample_tool_failure();

        assert_eq!(tf.id(), "fail123");
        assert_eq!(tf.session_id().as_str(), "sess-1");
        assert_eq!(tf.project(), "seogi");
        assert_eq!(tf.project_path(), "/Users/kim/projects/seogi");
        assert_eq!(tf.tool_name(), "Bash");
        assert_eq!(tf.error(), "Permission denied");
        assert_eq!(tf.timestamp(), Timestamp::new(1_713_000_000_000));

        assert_eq!(tf.clone(), tf);
    }

    #[test]
    fn tool_failure_display() {
        let tf = sample_tool_failure();
        assert_eq!(format!("{tf}"), "[sess-1] Bash FAILED (fail123)");
    }

    fn sample_system_event() -> SystemEvent {
        SystemEvent::new(
            "evt123".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/Users/kim/projects/seogi".to_string(),
            "Notification".to_string(),
            "Permission required".to_string(),
            Timestamp::new(1_713_000_000_000),
        )
    }

    #[test]
    fn system_event_creation_and_getters() {
        let se = sample_system_event();

        assert_eq!(se.id(), "evt123");
        assert_eq!(se.session_id().as_str(), "sess-1");
        assert_eq!(se.project(), "seogi");
        assert_eq!(se.project_path(), "/Users/kim/projects/seogi");
        assert_eq!(se.event_type(), "Notification");
        assert_eq!(se.content(), "Permission required");
        assert_eq!(se.timestamp(), Timestamp::new(1_713_000_000_000));

        assert_eq!(se.clone(), se);
    }

    #[test]
    fn system_event_display() {
        let se = sample_system_event();
        assert_eq!(format!("{se}"), "[sess-1] Notification (evt123)");
    }
}
