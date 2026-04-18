use chrono::{DateTime, Utc};

use super::error::DomainError;

/// 프로젝트의 대문자 알파벳 3글자 식별자.
///
/// 태스크 ID의 접두사로 사용된다 (e.g., `"SEO"` → `SEO-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct ProjectPrefix(String);

impl ProjectPrefix {
    /// 대문자 알파벳 3글자 검증 후 생성.
    ///
    /// # Errors
    ///
    /// 정확히 대문자 ASCII 알파벳 3글자가 아니면 `DomainError::Validation`.
    pub fn new(value: &str) -> Result<Self, DomainError> {
        if value.len() == 3 && value.chars().all(|c| c.is_ascii_uppercase()) {
            Ok(Self(value.to_string()))
        } else {
            Err(DomainError::Validation(format!(
                "ProjectPrefix must be exactly 3 uppercase ASCII letters, got: \"{value}\""
            )))
        }
    }

    /// 이름 앞 3글자를 대문자로 변환하여 생성.
    ///
    /// # Errors
    ///
    /// 이름이 3글자 미만이거나 앞 3글자가 ASCII 알파벳이 아니면 `DomainError::Validation`.
    pub fn from_name(name: &str) -> Result<Self, DomainError> {
        let chars: Vec<char> = name.chars().take(3).collect();
        if chars.len() < 3 || !chars.iter().all(char::is_ascii_alphabetic) {
            return Err(DomainError::Validation(format!(
                "Cannot derive ProjectPrefix from name \"{name}\": first 3 characters must be ASCII letters. Use --prefix to specify manually."
            )));
        }
        let prefix: String = chars.iter().map(char::to_ascii_uppercase).collect();
        Self::new(&prefix)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 태스크를 묶는 관리 단위.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Project {
    id: String,
    name: String,
    prefix: ProjectPrefix,
    goal: String,
    next_seq: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Project {
    /// 새 프로젝트를 생성한다.
    ///
    /// id는 UUID hex 32글자, `next_seq`은 1.
    ///
    /// # Errors
    ///
    /// name 또는 goal이 빈 문자열이면 `DomainError::Validation`.
    pub fn new(
        name: &str,
        prefix: &ProjectPrefix,
        goal: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Project name must not be empty".to_string(),
            ));
        }
        if goal.is_empty() {
            return Err(DomainError::Validation(
                "Project goal must not be empty".to_string(),
            ));
        }

        Ok(Self {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name: name.to_string(),
            prefix: prefix.clone(),
            goal: goal.to_string(),
            next_seq: 1,
            created_at: now,
            updated_at: now,
        })
    }

    /// DB에서 읽은 값으로 복원한다.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        id: String,
        name: String,
        prefix: ProjectPrefix,
        goal: String,
        next_seq: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            prefix,
            goal,
            next_seq,
            created_at,
            updated_at,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn prefix(&self) -> &ProjectPrefix {
        &self.prefix
    }

    #[must_use]
    pub fn goal(&self) -> &str {
        &self.goal
    }

    #[must_use]
    pub fn next_seq(&self) -> i64 {
        self.next_seq
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

#[cfg(test)]
mod tests {
    use super::*;

    // Q1: 대문자 알파벳 3글자 허용
    #[test]
    fn test_project_prefix_valid() {
        assert!(ProjectPrefix::new("SEO").is_ok());
        assert!(ProjectPrefix::new("LOC").is_ok());
        assert!(ProjectPrefix::new("ABC").is_ok());
    }

    // Q2: 소문자, 숫자, 길이 부적합 거부
    #[test]
    fn test_project_prefix_invalid() {
        assert!(ProjectPrefix::new("seo").is_err());
        assert!(ProjectPrefix::new("SE1").is_err());
        assert!(ProjectPrefix::new("SE").is_err());
        assert!(ProjectPrefix::new("SEOG").is_err());
        assert!(ProjectPrefix::new("").is_err());
        assert!(ProjectPrefix::new("S O").is_err());
    }

    // Q3: as_str() 반환값 일치
    #[test]
    fn test_project_prefix_as_str() {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        assert_eq!(prefix.as_str(), "SEO");
    }

    // Q4: 이름에서 prefix 자동 생성
    #[test]
    fn test_project_prefix_from_name() {
        let prefix = ProjectPrefix::from_name("Seogi").unwrap();
        assert_eq!(prefix.as_str(), "SEO");

        let prefix = ProjectPrefix::from_name("hello").unwrap();
        assert_eq!(prefix.as_str(), "HEL");
    }

    // Q5: 짧은 이름, 비ASCII → 에러
    #[test]
    fn test_project_prefix_from_name_invalid() {
        assert!(ProjectPrefix::from_name("ab").is_err());
        assert!(ProjectPrefix::from_name("서기프").is_err());
        assert!(ProjectPrefix::from_name("a").is_err());
        assert!(ProjectPrefix::from_name("").is_err());
        assert!(ProjectPrefix::from_name("12c").is_err());
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    // Q6: UUID hex 32글자
    #[test]
    fn test_project_new_id() {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", now()).unwrap();
        assert_eq!(project.id().len(), 32);
        assert!(project.id().chars().all(|c| c.is_ascii_hexdigit()));
    }

    // Q7: next_seq == 1
    #[test]
    fn test_project_new_next_seq() {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", now()).unwrap();
        assert_eq!(project.next_seq(), 1);
    }

    // Q8: created_at/updated_at은 전달한 시각과 일치
    #[test]
    fn test_project_new_timestamps() {
        let ts = now();
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", ts).unwrap();

        assert_eq!(project.created_at(), ts);
        assert_eq!(project.updated_at(), ts);
    }

    // Q9: 빈 name 또는 goal → 에러
    #[test]
    fn test_project_new_empty_name_or_goal() {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        assert!(Project::new("", &prefix, "goal", now()).is_err());
        assert!(Project::new("Seogi", &prefix, "", now()).is_err());
    }
}
