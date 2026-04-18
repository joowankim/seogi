use std::fmt;
use std::str::FromStr;

use crate::domain::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusCategory {
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
}

impl StatusCategory {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Unstarted => "unstarted",
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Canceled => "canceled",
        }
    }
}

impl fmt::Display for StatusCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for StatusCategory {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Self::Backlog),
            "unstarted" => Ok(Self::Unstarted),
            "started" => Ok(Self::Started),
            "completed" => Ok(Self::Completed),
            "canceled" => Ok(Self::Canceled),
            _ => Err(DomainError::Validation(format!(
                "invalid status category: {s}"
            ))),
        }
    }
}

/// 태스크의 상태를 나타내는 엔티티.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    id: String,
    name: String,
    category: StatusCategory,
    position: i64,
}

impl Status {
    /// 새 Status를 생성한다.
    ///
    /// # Errors
    ///
    /// name이 빈 문자열이면 `DomainError::Validation`.
    pub fn new(name: &str, category: StatusCategory, position: i64) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Status name must not be empty".to_string(),
            ));
        }
        Ok(Self {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name: name.to_string(),
            category,
            position,
        })
    }

    /// DB에서 읽은 값으로 복원한다.
    #[must_use]
    pub fn from_row(id: String, name: String, category: StatusCategory, position: i64) -> Self {
        Self {
            id,
            name,
            category,
            position,
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
    pub fn category(&self) -> StatusCategory {
        self.category
    }

    #[must_use]
    pub fn position(&self) -> i64 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_category_variant_count() {
        let all = [
            StatusCategory::Backlog,
            StatusCategory::Unstarted,
            StatusCategory::Started,
            StatusCategory::Completed,
            StatusCategory::Canceled,
        ];
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_status_category_as_str() {
        assert_eq!(StatusCategory::Backlog.as_str(), "backlog");
        assert_eq!(StatusCategory::Unstarted.as_str(), "unstarted");
        assert_eq!(StatusCategory::Started.as_str(), "started");
        assert_eq!(StatusCategory::Completed.as_str(), "completed");
        assert_eq!(StatusCategory::Canceled.as_str(), "canceled");
    }

    #[test]
    fn test_status_category_from_str_valid() {
        assert_eq!(
            "backlog".parse::<StatusCategory>().unwrap(),
            StatusCategory::Backlog
        );
        assert_eq!(
            "unstarted".parse::<StatusCategory>().unwrap(),
            StatusCategory::Unstarted
        );
        assert_eq!(
            "started".parse::<StatusCategory>().unwrap(),
            StatusCategory::Started
        );
        assert_eq!(
            "completed".parse::<StatusCategory>().unwrap(),
            StatusCategory::Completed
        );
        assert_eq!(
            "canceled".parse::<StatusCategory>().unwrap(),
            StatusCategory::Canceled
        );
    }

    #[test]
    fn test_status_category_display() {
        assert_eq!(format!("{}", StatusCategory::Backlog), "backlog");
        assert_eq!(format!("{}", StatusCategory::Canceled), "canceled");
    }

    #[test]
    fn test_status_category_from_str_invalid() {
        assert!("invalid".parse::<StatusCategory>().is_err());
        assert!("".parse::<StatusCategory>().is_err());
        assert!("BACKLOG".parse::<StatusCategory>().is_err());
    }

    // Q1: UUID hex 32글자
    #[test]
    fn test_status_new_id() {
        let status = Status::new("testing", StatusCategory::Started, 7).unwrap();
        assert_eq!(status.id().len(), 32);
        assert!(status.id().chars().all(|c| c.is_ascii_hexdigit()));
    }

    // Q2: name, category, position 보존
    #[test]
    fn test_status_new_fields() {
        let status = Status::new("testing", StatusCategory::Started, 7).unwrap();
        assert_eq!(status.name(), "testing");
        assert_eq!(status.category(), StatusCategory::Started);
        assert_eq!(status.position(), 7);
    }

    // Q3: 빈 이름 → 에러
    #[test]
    fn test_status_new_empty_name() {
        assert!(Status::new("", StatusCategory::Started, 0).is_err());
    }
}
