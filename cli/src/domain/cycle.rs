use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};

use super::error::DomainError;

/// Cycle의 상태.
///
/// 3개 고정 값: planned, active, completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CycleStatus {
    Planned,
    Active,
    Completed,
}

impl CycleStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }
}

impl FromStr for CycleStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(Self::Planned),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            _ => Err(DomainError::Validation(format!(
                "Invalid CycleStatus: \"{s}\". Must be one of: planned, active, completed"
            ))),
        }
    }
}

impl std::fmt::Display for CycleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 기간별 목표 단위.
///
/// 워크스페이스에 속하며, 태스크를 배정하여 달성도를 측정한다.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Cycle {
    id: String,
    workspace_id: String,
    name: String,
    status: CycleStatus,
    start_date: String,
    end_date: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Cycle {
    /// 새 Cycle을 생성한다.
    ///
    /// 초기 상태는 `planned`. id는 UUID hex 32글자.
    ///
    /// # Errors
    ///
    /// - name이 빈 문자열이면 `DomainError::Validation`.
    /// - `start_date` > `end_date`이면 `DomainError::Validation`.
    pub fn new(
        workspace_id: &str,
        name: &str,
        start_date: &str,
        end_date: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Cycle name must not be empty".to_string(),
            ));
        }

        validate_date_order(start_date, end_date)?;

        Ok(Self {
            id: uuid::Uuid::new_v4().simple().to_string(),
            workspace_id: workspace_id.to_string(),
            name: name.to_string(),
            status: CycleStatus::Planned,
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// DB에서 읽은 값으로 복원한다.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        id: String,
        workspace_id: String,
        name: String,
        status: CycleStatus,
        start_date: String,
        end_date: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            workspace_id,
            name,
            status,
            start_date,
            end_date,
            created_at,
            updated_at,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn status(&self) -> CycleStatus {
        self.status
    }

    #[must_use]
    pub fn start_date(&self) -> &str {
        &self.start_date
    }

    #[must_use]
    pub fn end_date(&self) -> &str {
        &self.end_date
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

/// YYYY-MM-DD 형식의 날짜 문자열을 파싱한다.
///
/// # Errors
///
/// 형식이 올바르지 않으면 `DomainError::Validation`.
pub fn parse_date(date_str: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
        DomainError::Validation(format!(
            "Invalid date format: \"{date_str}\". Expected YYYY-MM-DD"
        ))
    })
}

/// `start_date` <= `end_date` 제약을 검증한다.
///
/// # Errors
///
/// `start_date` > `end_date`이면 `DomainError::Validation`.
pub fn validate_date_order(start_date: &str, end_date: &str) -> Result<(), DomainError> {
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    if start > end {
        return Err(DomainError::Validation(format!(
            "start_date ({start_date}) must not be after end_date ({end_date})"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    // Q1: CycleStatus enum 3개 variant 존재
    #[test]
    fn test_cycle_status_variant_count() {
        let variants = [
            CycleStatus::Planned,
            CycleStatus::Active,
            CycleStatus::Completed,
        ];
        assert_eq!(variants.len(), 3);
    }

    // Q2: CycleStatus::as_str → 소문자 문자열 반환
    #[test]
    fn test_cycle_status_as_str() {
        assert_eq!(CycleStatus::Planned.as_str(), "planned");
        assert_eq!(CycleStatus::Active.as_str(), "active");
        assert_eq!(CycleStatus::Completed.as_str(), "completed");
    }

    // Q3: CycleStatus::from_str 유효값 → 해당 variant
    #[test]
    fn test_cycle_status_from_str_valid() {
        assert_eq!(
            CycleStatus::from_str("planned").unwrap(),
            CycleStatus::Planned
        );
        assert_eq!(
            CycleStatus::from_str("active").unwrap(),
            CycleStatus::Active
        );
        assert_eq!(
            CycleStatus::from_str("completed").unwrap(),
            CycleStatus::Completed
        );
    }

    // Q4: CycleStatus::from_str 무효값 → DomainError::Validation
    #[test]
    fn test_cycle_status_from_str_invalid() {
        assert!(CycleStatus::from_str("invalid").is_err());
        assert!(CycleStatus::from_str("").is_err());
        assert!(CycleStatus::from_str("PLANNED").is_err());
    }

    // Q5: Cycle::new 유효 입력 → id 32글자 hex
    #[test]
    fn test_cycle_new_id_format() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-01", "2026-05-14", now()).unwrap();
        assert_eq!(cycle.id().len(), 32);
        assert!(cycle.id().chars().all(|c| c.is_ascii_hexdigit()));
    }

    // Q6: Cycle::new 유효 입력 → status가 planned
    #[test]
    fn test_cycle_new_initial_status() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-01", "2026-05-14", now()).unwrap();
        assert_eq!(cycle.status(), CycleStatus::Planned);
    }

    // Q7: Cycle::new 유효 입력 → 필드값 보존
    #[test]
    fn test_cycle_new_fields() {
        let ts = now();
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-01", "2026-05-14", ts).unwrap();
        assert_eq!(cycle.workspace_id(), "ws1");
        assert_eq!(cycle.name(), "Sprint 1");
        assert_eq!(cycle.start_date(), "2026-05-01");
        assert_eq!(cycle.end_date(), "2026-05-14");
        assert_eq!(cycle.created_at(), ts);
        assert_eq!(cycle.updated_at(), ts);
    }

    // Q8: Cycle::new 빈 name → DomainError::Validation
    #[test]
    fn test_cycle_new_empty_name() {
        assert!(Cycle::new("ws1", "", "2026-05-01", "2026-05-14", now()).is_err());
    }

    // Q9: Cycle::new start_date > end_date → DomainError::Validation
    #[test]
    fn test_cycle_new_start_after_end() {
        assert!(Cycle::new("ws1", "Sprint 1", "2026-05-15", "2026-05-01", now()).is_err());
    }

    // Q10: Cycle::from_row 필드값 보존
    #[test]
    fn test_cycle_from_row_fields() {
        let ts = now();
        let cycle = Cycle::from_row(
            "abc123".to_string(),
            "ws1".to_string(),
            "Sprint 1".to_string(),
            CycleStatus::Active,
            "2026-05-01".to_string(),
            "2026-05-14".to_string(),
            ts,
            ts,
        );
        assert_eq!(cycle.id(), "abc123");
        assert_eq!(cycle.workspace_id(), "ws1");
        assert_eq!(cycle.name(), "Sprint 1");
        assert_eq!(cycle.status(), CycleStatus::Active);
        assert_eq!(cycle.start_date(), "2026-05-01");
        assert_eq!(cycle.end_date(), "2026-05-14");
        assert_eq!(cycle.created_at(), ts);
        assert_eq!(cycle.updated_at(), ts);
    }
}
