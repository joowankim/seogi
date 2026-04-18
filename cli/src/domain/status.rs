use std::fmt;
use std::str::FromStr;

use crate::domain::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}
