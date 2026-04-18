use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::project::ProjectPrefix;
use super::value::Timestamp;

/// CLI에서 생성한 이벤트의 `session_id`.
pub const CLI_SESSION_ID: &str = "CLI";

/// 태스크의 분류 라벨.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Label {
    Feature,
    Bug,
    Refactor,
    Chore,
    Docs,
}

impl Label {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Bug => "bug",
            Self::Refactor => "refactor",
            Self::Chore => "chore",
            Self::Docs => "docs",
        }
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Label {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "feature" => Ok(Self::Feature),
            "bug" => Ok(Self::Bug),
            "refactor" => Ok(Self::Refactor),
            "chore" => Ok(Self::Chore),
            "docs" => Ok(Self::Docs),
            _ => Err(DomainError::Validation(format!(
                "invalid label: \"{s}\". must be one of: feature, bug, refactor, chore, docs"
            ))),
        }
    }
}

/// 단위 작업.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Task {
    id: String,
    title: String,
    description: String,
    label: Label,
    status_id: String,
    project_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Task {
    /// 새 태스크를 생성한다.
    ///
    /// id는 `{prefix}-{seq}` 형식.
    ///
    /// # Errors
    ///
    /// title 또는 description이 빈 문자열이면 `DomainError::Validation`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prefix: &ProjectPrefix,
        seq: i64,
        title: &str,
        description: &str,
        label: Label,
        status_id: &str,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if title.is_empty() {
            return Err(DomainError::Validation(
                "Task title must not be empty".to_string(),
            ));
        }
        if description.is_empty() {
            return Err(DomainError::Validation(
                "Task description must not be empty".to_string(),
            ));
        }
        Ok(Self {
            id: format!("{prefix}-{seq}"),
            title: title.to_string(),
            description: description.to_string(),
            label,
            status_id: status_id.to_string(),
            project_id: project_id.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// DB에서 읽은 값으로 복원한다.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        id: String,
        title: String,
        description: String,
        label: Label,
        status_id: String,
        project_id: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title,
            description,
            label,
            status_id,
            project_id,
            created_at,
            updated_at,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub fn label(&self) -> Label {
        self.label
    }

    #[must_use]
    pub fn status_id(&self) -> &str {
        &self.status_id
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// 태스크 상태 변경 이벤트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvent {
    id: String,
    task_id: String,
    from_status: Option<String>,
    to_status: String,
    session_id: String,
    timestamp: Timestamp,
}

impl TaskEvent {
    /// 새 이벤트를 생성한다.
    #[must_use]
    pub fn new(
        task_id: &str,
        from_status: Option<&str>,
        to_status: &str,
        session_id: &str,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().simple().to_string(),
            task_id: task_id.to_string(),
            from_status: from_status.map(String::from),
            to_status: to_status.to_string(),
            session_id: session_id.to_string(),
            timestamp,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    #[must_use]
    pub fn from_status(&self) -> Option<&str> {
        self.from_status.as_deref()
    }

    #[must_use]
    pub fn to_status(&self) -> &str {
        &self.to_status
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Q1: Label enum 5개 variant
    #[test]
    fn test_label_variant_count() {
        let all = [
            Label::Feature,
            Label::Bug,
            Label::Refactor,
            Label::Chore,
            Label::Docs,
        ];
        assert_eq!(all.len(), 5);
    }

    // Q2: Label::from_str 유효값
    #[test]
    fn test_label_from_str_valid() {
        assert_eq!("feature".parse::<Label>().unwrap(), Label::Feature);
        assert_eq!("bug".parse::<Label>().unwrap(), Label::Bug);
        assert_eq!("refactor".parse::<Label>().unwrap(), Label::Refactor);
        assert_eq!("chore".parse::<Label>().unwrap(), Label::Chore);
        assert_eq!("docs".parse::<Label>().unwrap(), Label::Docs);
    }

    // Q3: Label::from_str 무효값
    #[test]
    fn test_label_from_str_invalid() {
        assert!("invalid".parse::<Label>().is_err());
        assert!("".parse::<Label>().is_err());
        assert!("FEATURE".parse::<Label>().is_err());
    }

    // Q4: Label::as_str 소문자 문자열
    #[test]
    fn test_label_as_str() {
        assert_eq!(Label::Feature.as_str(), "feature");
        assert_eq!(Label::Bug.as_str(), "bug");
        assert_eq!(Label::Refactor.as_str(), "refactor");
        assert_eq!(Label::Chore.as_str(), "chore");
        assert_eq!(Label::Docs.as_str(), "docs");
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn prefix() -> ProjectPrefix {
        ProjectPrefix::new("SEO").unwrap()
    }

    // Q5: Task::new id 형식
    #[test]
    fn test_task_new_id_format() {
        let task = Task::new(
            &prefix(),
            1,
            "title",
            "desc",
            Label::Feature,
            "status-id",
            "project-id",
            now(),
        )
        .unwrap();
        assert_eq!(task.id(), "SEO-1");
    }

    // Q6: Task::new 필드값 보존
    #[test]
    fn test_task_new_fields() {
        let ts = now();
        let task = Task::new(&prefix(), 42, "title", "desc", Label::Bug, "sid", "pid", ts).unwrap();
        assert_eq!(task.id(), "SEO-42");
        assert_eq!(task.title(), "title");
        assert_eq!(task.description(), "desc");
        assert_eq!(task.label(), Label::Bug);
        assert_eq!(task.status_id(), "sid");
        assert_eq!(task.project_id(), "pid");
        assert_eq!(task.created_at(), ts);
        assert_eq!(task.updated_at(), ts);
    }

    // Q7: 빈 title → 에러
    #[test]
    fn test_task_new_empty_title() {
        let result = Task::new(
            &prefix(),
            1,
            "",
            "desc",
            Label::Feature,
            "sid",
            "pid",
            now(),
        );
        assert!(result.is_err());
    }

    // Q8: 빈 description → 에러
    #[test]
    fn test_task_new_empty_description() {
        let result = Task::new(
            &prefix(),
            1,
            "title",
            "",
            Label::Feature,
            "sid",
            "pid",
            now(),
        );
        assert!(result.is_err());
    }

    // Q9: TaskEvent::new 필드값 보존
    #[test]
    fn test_task_event_new_fields() {
        let ts = Timestamp::new(1_000_000);
        let event = TaskEvent::new("SEO-1", None, "backlog", "CLI", ts);
        assert_eq!(event.task_id(), "SEO-1");
        assert!(event.from_status().is_none());
        assert_eq!(event.to_status(), "backlog");
        assert_eq!(event.session_id(), "CLI");
        assert_eq!(event.timestamp(), ts);
        assert_eq!(event.id().len(), 32);

        let event2 = TaskEvent::new("SEO-1", Some("backlog"), "todo", "session-1", ts);
        assert_eq!(event2.from_status(), Some("backlog"));
    }

    // Q10: CLI_SESSION_ID 상수값
    #[test]
    fn test_cli_session_id_constant() {
        assert_eq!(CLI_SESSION_ID, "CLI");
    }
}
