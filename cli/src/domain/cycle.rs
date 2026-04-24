use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};

use super::error::DomainError;

/// Cycle의 상태.
///
/// 3개 고정 값: planned, active, completed.
/// DB에 저장되지 않고 `start_date`/`end_date` + 현재 날짜로 파생된다.
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

/// `cycle_tasks`의 배정 방식.
///
/// `planned`: 명시적 배정. `auto`: 자동 포함 (task create/move 시).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Assigned {
    Planned,
    Auto,
}

impl Assigned {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Auto => "auto",
        }
    }
}

impl FromStr for Assigned {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(Self::Planned),
            "auto" => Ok(Self::Auto),
            _ => Err(DomainError::Validation(format!(
                "Invalid Assigned: \"{s}\". Must be one of: planned, auto"
            ))),
        }
    }
}

impl std::fmt::Display for Assigned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 기간별 목표 단위.
///
/// 워크스페이스에 속하며, 태스크를 배정하여 달성도를 측정한다.
/// status는 DB에 저장되지 않고 `start_date`/`end_date` + 현재 날짜로 파생된다.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Cycle {
    id: String,
    workspace_id: String,
    name: String,
    start_date: String,
    end_date: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Cycle {
    /// 새 Cycle을 생성한다.
    ///
    /// id는 UUID hex 32글자.
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
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// DB에서 읽은 값으로 복원한다.
    #[must_use]
    pub fn from_row(
        id: String,
        workspace_id: String,
        name: String,
        start_date: String,
        end_date: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            workspace_id,
            name,
            start_date,
            end_date,
            created_at,
            updated_at,
        }
    }

    /// 현재 날짜 기준으로 Cycle의 상태를 파생한다.
    #[must_use]
    pub fn status(&self, today: NaiveDate) -> CycleStatus {
        derive_status(&self.start_date, &self.end_date, today)
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

/// `start_date`/`end_date` + 현재 날짜로 `CycleStatus`를 파생한다.
#[must_use]
pub fn derive_status(start_date: &str, end_date: &str, today: NaiveDate) -> CycleStatus {
    let start = parse_date(start_date).unwrap_or(today);
    let end = parse_date(end_date).unwrap_or(today);
    if today < start {
        CycleStatus::Planned
    } else if today > end {
        CycleStatus::Completed
    } else {
        CycleStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn date(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    // Q1: status(today < start_date) → Planned
    #[test]
    fn test_status_planned() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-10", "2026-05-20", now()).unwrap();
        assert_eq!(cycle.status(date("2026-05-09")), CycleStatus::Planned);
    }

    // Q2: status(start_date <= today <= end_date) → Active
    #[test]
    fn test_status_active() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-10", "2026-05-20", now()).unwrap();
        assert_eq!(cycle.status(date("2026-05-15")), CycleStatus::Active);
    }

    // Q3: status(today > end_date) → Completed
    #[test]
    fn test_status_completed() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-10", "2026-05-20", now()).unwrap();
        assert_eq!(cycle.status(date("2026-05-21")), CycleStatus::Completed);
    }

    // Q4: status(today == start_date) → Active
    #[test]
    fn test_status_boundary_start() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-10", "2026-05-20", now()).unwrap();
        assert_eq!(cycle.status(date("2026-05-10")), CycleStatus::Active);
    }

    // Q5: status(today == end_date) → Active
    #[test]
    fn test_status_boundary_end() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-10", "2026-05-20", now()).unwrap();
        assert_eq!(cycle.status(date("2026-05-20")), CycleStatus::Active);
    }

    // Q6-Q9: derive_status 독립 함수 검증
    #[test]
    fn test_derive_status_planned() {
        assert_eq!(
            derive_status("2026-05-10", "2026-05-20", date("2026-05-09")),
            CycleStatus::Planned
        );
    }

    #[test]
    fn test_derive_status_active() {
        assert_eq!(
            derive_status("2026-05-10", "2026-05-20", date("2026-05-15")),
            CycleStatus::Active
        );
    }

    #[test]
    fn test_derive_status_completed() {
        assert_eq!(
            derive_status("2026-05-10", "2026-05-20", date("2026-05-21")),
            CycleStatus::Completed
        );
    }

    #[test]
    fn test_derive_status_boundary() {
        assert_eq!(
            derive_status("2026-05-10", "2026-05-20", date("2026-05-10")),
            CycleStatus::Active
        );
        assert_eq!(
            derive_status("2026-05-10", "2026-05-20", date("2026-05-20")),
            CycleStatus::Active
        );
    }

    // Q10: Cycle::new status 파라미터 없이 생성 성공
    #[test]
    fn test_cycle_new_no_status() {
        let cycle = Cycle::new("ws1", "Sprint 1", "2026-05-01", "2026-05-14", now()).unwrap();
        assert_eq!(cycle.id().len(), 32);
        assert_eq!(cycle.name(), "Sprint 1");
    }

    // Q11: Cycle::from_row status 파라미터 없이 복원 성공
    #[test]
    fn test_cycle_from_row_no_status() {
        let ts = now();
        let cycle = Cycle::from_row(
            "abc123".to_string(),
            "ws1".to_string(),
            "Sprint 1".to_string(),
            "2026-05-01".to_string(),
            "2026-05-14".to_string(),
            ts,
            ts,
        );
        assert_eq!(cycle.id(), "abc123");
        assert_eq!(cycle.name(), "Sprint 1");
        assert_eq!(cycle.start_date(), "2026-05-01");
    }

    // Q1: Assigned enum 2개 variant
    #[test]
    fn test_assigned_variant_count() {
        let variants = [Assigned::Planned, Assigned::Auto];
        assert_eq!(variants.len(), 2);
    }

    // Q2: Assigned::as_str
    #[test]
    fn test_assigned_as_str() {
        assert_eq!(Assigned::Planned.as_str(), "planned");
        assert_eq!(Assigned::Auto.as_str(), "auto");
    }

    // Q3: Assigned::from_str 유효값
    #[test]
    fn test_assigned_from_str_valid() {
        assert_eq!(Assigned::from_str("planned").unwrap(), Assigned::Planned);
        assert_eq!(Assigned::from_str("auto").unwrap(), Assigned::Auto);
    }

    // Q4: Assigned::from_str 무효값
    #[test]
    fn test_assigned_from_str_invalid() {
        assert!(Assigned::from_str("invalid").is_err());
        assert!(Assigned::from_str("").is_err());
    }

    // 기존 테스트: 빈 name → 에러
    #[test]
    fn test_cycle_new_empty_name() {
        assert!(Cycle::new("ws1", "", "2026-05-01", "2026-05-14", now()).is_err());
    }

    // 기존 테스트: start_date > end_date → 에러
    #[test]
    fn test_cycle_new_start_after_end() {
        assert!(Cycle::new("ws1", "Sprint 1", "2026-05-15", "2026-05-01", now()).is_err());
    }
}
