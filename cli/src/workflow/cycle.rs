use chrono::Utc;
use rusqlite::Connection;

use crate::adapter::{cycle_repo, workspace_repo};
use crate::domain::cycle::{self, Cycle};
use crate::domain::error::DomainError;

/// Cycle을 생성한다.
///
/// # Errors
///
/// - 워크스페이스 미존재 → `DomainError::Validation`
/// - 빈 이름, 잘못된 날짜, start > end → `DomainError::Validation`
/// - DB 에러 → `DomainError::Database`
pub fn create(
    conn: &Connection,
    workspace_name: &str,
    name: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Cycle, DomainError> {
    let workspace = workspace_repo::find_by_name(conn, workspace_name)?.ok_or_else(|| {
        DomainError::Validation(format!("Workspace \"{workspace_name}\" not found"))
    })?;

    let cycle = Cycle::new(workspace.id(), name, start_date, end_date, Utc::now())?;
    cycle_repo::save(conn, &cycle)?;
    Ok(cycle)
}

/// Cycle 목록을 조회한다.
///
/// `workspace_name`이 `Some`이면 해당 워크스페이스만 필터링한다.
/// 존재하지 않는 워크스페이스 이름이면 빈 목록을 반환한다.
///
/// # Errors
///
/// DB 에러 → `DomainError::Database`.
pub fn list(
    conn: &Connection,
    workspace_name: Option<&str>,
) -> Result<Vec<cycle_repo::CycleListRow>, DomainError> {
    let rows = cycle_repo::list_detailed(conn, workspace_name)?;
    Ok(rows)
}

/// Cycle을 수정한다.
///
/// name, `start_date`, `end_date` 중 하나 이상 제공해야 한다.
/// 날짜 변경 시 `start_date` <= `end_date` 제약을 검증한다.
///
/// # Errors
///
/// - 필드 전부 None → `DomainError::Validation`
/// - 존재하지 않는 `cycle_id` → `DomainError::Validation`
/// - 빈 이름 → `DomainError::Validation`
/// - 잘못된 날짜 형식 → `DomainError::Validation`
/// - 결과 start > end → `DomainError::Validation`
/// - DB 에러 → `DomainError::Database`
pub fn update(
    conn: &Connection,
    cycle_id: &str,
    name: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<(), DomainError> {
    if name.is_none() && start_date.is_none() && end_date.is_none() {
        return Err(DomainError::Validation(
            "At least one field (name, start, end) must be provided".to_string(),
        ));
    }

    if let Some(n) = name
        && n.is_empty()
    {
        return Err(DomainError::Validation(
            "Cycle name must not be empty".to_string(),
        ));
    }

    let existing = cycle_repo::find_by_id(conn, cycle_id)?
        .ok_or_else(|| DomainError::Validation(format!("Cycle \"{cycle_id}\" not found")))?;

    // 날짜 검증: 변경 후 최종 start/end로 검증
    if start_date.is_some() || end_date.is_some() {
        let final_start = start_date.unwrap_or(existing.start_date());
        let final_end = end_date.unwrap_or(existing.end_date());
        cycle::validate_date_order(final_start, final_end)?;
    }

    cycle_repo::update(conn, cycle_id, name, start_date, end_date)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;

    fn setup_workspace(conn: &Connection) {
        crate::workflow::workspace::create(conn, "Seogi", Some("SEO"), "하니스 계측").unwrap();
    }

    // Q19: create 성공 시 Cycle 반환, DB에 1건 저장
    #[test]
    fn test_create_success() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        assert_eq!(cycle.name(), "Sprint 1");
        assert_eq!(cycle.start_date(), "2026-05-01");

        let all = cycle_repo::list_all(&conn).unwrap();
        assert_eq!(all.len(), 1);
    }

    // Q20: create 존재하지 않는 워크스페이스 → 에러
    #[test]
    fn test_create_unknown_workspace() {
        let conn = initialize_in_memory().unwrap();
        let result = create(&conn, "NonExistent", "Sprint 1", "2026-05-01", "2026-05-14");
        assert!(result.is_err());
    }

    // Q21: create 빈 이름 → 에러
    #[test]
    fn test_create_empty_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "", "2026-05-01", "2026-05-14");
        assert!(result.is_err());
    }

    // Q22: create 잘못된 날짜 형식 → 에러
    #[test]
    fn test_create_invalid_date() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "Sprint 1", "not-a-date", "2026-05-14");
        assert!(result.is_err());
    }

    // Q23: create start > end → 에러
    #[test]
    fn test_create_start_after_end() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "Sprint 1", "2026-05-15", "2026-05-01");
        assert!(result.is_err());
    }

    // Q24: list 워크스페이스 필터 적용
    #[test]
    fn test_list_with_filter() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        crate::workflow::workspace::create(&conn, "Other", Some("OTH"), "other").unwrap();

        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        create(&conn, "Other", "Sprint A", "2026-06-01", "2026-06-14").unwrap();

        let rows = list(&conn, Some("Seogi")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Sprint 1");
    }

    // Q25: list 필터 없이 전체 반환
    #[test]
    fn test_list_all() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        create(&conn, "Seogi", "Sprint 2", "2026-05-15", "2026-05-28").unwrap();

        let rows = list(&conn, None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    // Q26: update 이름만 변경 → 성공, updated_at 갱신
    #[test]
    fn test_update_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        update(&conn, cycle.id(), Some("Updated"), None, None).unwrap();

        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.name(), "Updated");
        assert!(found.updated_at() >= cycle.updated_at());
    }

    // Q27: update 시작일/종료일 변경 → 성공
    #[test]
    fn test_update_dates() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        update(
            &conn,
            cycle.id(),
            None,
            Some("2026-06-01"),
            Some("2026-06-14"),
        )
        .unwrap();

        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.start_date(), "2026-06-01");
        assert_eq!(found.end_date(), "2026-06-14");
    }

    // Q28: update 없는 cycle_id → 에러
    #[test]
    fn test_update_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = update(&conn, "nonexistent", Some("name"), None, None);
        assert!(result.is_err());
    }

    // Q29: update 필드 전부 None → 에러
    #[test]
    fn test_update_no_fields() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let result = update(&conn, cycle.id(), None, None, None);
        assert!(result.is_err());
    }

    // Q30: update 잘못된 날짜 형식 → 에러
    #[test]
    fn test_update_invalid_date() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let result = update(&conn, cycle.id(), None, Some("bad-date"), None);
        assert!(result.is_err());
    }

    // Q31: update 결과 start > end → 에러
    #[test]
    fn test_update_start_after_end() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        // start만 변경하여 기존 end보다 뒤가 되는 경우
        let result = update(&conn, cycle.id(), None, Some("2026-06-01"), None);
        assert!(result.is_err());
    }

    // Q32: update 빈 이름 → 에러
    #[test]
    fn test_update_empty_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let result = update(&conn, cycle.id(), Some(""), None, None);
        assert!(result.is_err());
    }

    // update end_date만 변경 → 성공 (end_date.is_some() 분기 커버)
    #[test]
    fn test_update_end_only() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        update(&conn, cycle.id(), None, None, Some("2026-05-21")).unwrap();

        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.end_date(), "2026-05-21");
    }

    // update start_date만 변경 → 성공 (start_date.is_some() 분기 커버)
    #[test]
    fn test_update_start_only() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-05", "2026-05-14").unwrap();

        update(&conn, cycle.id(), None, Some("2026-05-01"), None).unwrap();

        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.start_date(), "2026-05-01");
    }

    // Q33: list 존재하지 않는 워크스페이스 필터 → 빈 목록 반환
    #[test]
    fn test_list_unknown_workspace() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let rows = list(&conn, Some("NonExistent")).unwrap();
        assert!(rows.is_empty());
    }
}
