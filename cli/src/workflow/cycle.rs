use chrono::Utc;
use rusqlite::Connection;

use crate::adapter::task_repo;
use crate::adapter::{cycle_repo, cycle_task_repo, workspace_repo};
use crate::domain::cycle::{self, Assigned, Cycle};
use crate::domain::error::DomainError;

/// Cycle을 생성한다.
///
/// # Errors
///
/// - 워크스페이스 미존재 → `DomainError::Validation`
/// - 빈 이름, 잘못된 날짜, start > end → `DomainError::Validation`
/// - 같은 워크스페이스 내 날짜 구간 겹침 → `DomainError::Validation`
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
    check_overlap(conn, workspace.id(), start_date, end_date, None)?;
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
/// 날짜 변경 시 `start_date` <= `end_date` 제약과 겹침을 검증한다.
///
/// # Errors
///
/// - 필드 전부 None → `DomainError::Validation`
/// - 존재하지 않는 `cycle_id` → `DomainError::Validation`
/// - 빈 이름 → `DomainError::Validation`
/// - 잘못된 날짜 형식 → `DomainError::Validation`
/// - 결과 start > end → `DomainError::Validation`
/// - 날짜 구간 겹침 → `DomainError::Validation`
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

    if start_date.is_some() || end_date.is_some() {
        let final_start = start_date.unwrap_or(existing.start_date());
        let final_end = end_date.unwrap_or(existing.end_date());
        cycle::validate_date_order(final_start, final_end)?;
        check_overlap(
            conn,
            existing.workspace_id(),
            final_start,
            final_end,
            Some(cycle_id),
        )?;
    }

    cycle_repo::update(conn, cycle_id, name, start_date, end_date)?;
    Ok(())
}

/// Cycle에 태스크를 명시적으로 배정한다.
///
/// # Errors
///
/// - Cycle 미존재, 태스크 미존재, 중복 배정 → `DomainError::Validation`
/// - DB 에러 → `DomainError::Database`
pub fn assign(conn: &Connection, cycle_id: &str, task_id: &str) -> Result<(), DomainError> {
    cycle_repo::find_by_id(conn, cycle_id)?
        .ok_or_else(|| DomainError::Validation(format!("Cycle \"{cycle_id}\" not found")))?;
    task_repo::find_by_id(conn, task_id)?
        .ok_or_else(|| DomainError::Validation(format!("Task \"{task_id}\" not found")))?;

    if cycle_task_repo::is_assigned_to_cycle(conn, cycle_id, task_id)? {
        return Err(DomainError::Validation(format!(
            "Task \"{task_id}\" is already assigned to cycle \"{cycle_id}\""
        )));
    }

    cycle_task_repo::save(conn, cycle_id, task_id, Assigned::Planned)?;
    Ok(())
}

/// Cycle에서 태스크 배정을 해제한다.
///
/// # Errors
///
/// - 미배정 → `DomainError::Validation`
/// - DB 에러 → `DomainError::Database`
pub fn unassign(conn: &Connection, cycle_id: &str, task_id: &str) -> Result<(), DomainError> {
    let deleted = cycle_task_repo::delete(conn, cycle_id, task_id)?;
    if !deleted {
        return Err(DomainError::Validation(format!(
            "Task \"{task_id}\" is not assigned to cycle \"{cycle_id}\""
        )));
    }
    Ok(())
}

fn check_overlap(
    conn: &Connection,
    workspace_id: &str,
    start_date: &str,
    end_date: &str,
    exclude_id: Option<&str>,
) -> Result<(), DomainError> {
    let overlapping = cycle_repo::list_by_workspace_overlapping(
        conn,
        workspace_id,
        start_date,
        end_date,
        exclude_id,
    )?;
    if !overlapping.is_empty() {
        return Err(DomainError::Validation(format!(
            "Date range {start_date}~{end_date} overlaps with existing cycle \"{}\"",
            overlapping[0].name()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;

    fn setup_workspace(conn: &Connection) {
        crate::workflow::workspace::create(conn, "Seogi", Some("SEO"), "하니스 계측").unwrap();
    }

    fn setup_task(conn: &Connection) -> String {
        let task =
            crate::workflow::task::create(conn, "Seogi", "Task 1", "desc", "feature").unwrap();
        task.id().to_string()
    }

    // Q15: assign 성공
    #[test]
    fn test_assign_success() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let task_id = setup_task(&conn);

        assign(&conn, cycle.id(), &task_id).unwrap();

        assert!(cycle_task_repo::is_assigned_to_cycle(&conn, cycle.id(), &task_id).unwrap());
    }

    // Q16: assign 존재하지 않는 cycle_id → 에러
    #[test]
    fn test_assign_cycle_not_found() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let task_id = setup_task(&conn);

        let result = assign(&conn, "nonexistent", &task_id);
        assert!(result.is_err());
    }

    // Q17: assign 존재하지 않는 task_id → 에러
    #[test]
    fn test_assign_task_not_found() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let result = assign(&conn, cycle.id(), "SEO-999");
        assert!(result.is_err());
    }

    // Q18: assign 중복 배정 → 에러
    #[test]
    fn test_assign_duplicate() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let task_id = setup_task(&conn);

        assign(&conn, cycle.id(), &task_id).unwrap();
        let result = assign(&conn, cycle.id(), &task_id);
        assert!(result.is_err());
    }

    // Q19: unassign 성공
    #[test]
    fn test_unassign_success() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let task_id = setup_task(&conn);

        assign(&conn, cycle.id(), &task_id).unwrap();
        unassign(&conn, cycle.id(), &task_id).unwrap();

        assert!(!cycle_task_repo::is_assigned_to_cycle(&conn, cycle.id(), &task_id).unwrap());
    }

    // Q20: unassign 미배정 → 에러
    #[test]
    fn test_unassign_not_found() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let task_id = setup_task(&conn);

        let result = unassign(&conn, cycle.id(), &task_id);
        assert!(result.is_err());
    }

    // Q18: create 겹침 없으면 성공
    #[test]
    fn test_create_no_overlap() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        let c1 = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let c2 = create(&conn, "Seogi", "Sprint 2", "2026-05-15", "2026-05-28").unwrap();

        assert_eq!(c1.name(), "Sprint 1");
        assert_eq!(c2.name(), "Sprint 2");
    }

    // Q19: create 겹치면 에러
    #[test]
    fn test_create_overlap_error() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let result = create(&conn, "Seogi", "Sprint 2", "2026-05-10", "2026-05-24");
        assert!(result.is_err());
    }

    // Q20: update 날짜 변경 후 겹침 없으면 성공
    #[test]
    fn test_update_no_overlap() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        let c1 = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        create(&conn, "Seogi", "Sprint 2", "2026-05-15", "2026-05-28").unwrap();

        update(&conn, c1.id(), None, None, Some("2026-05-10")).unwrap();
    }

    // Q21: update 날짜 변경 후 겹치면 에러
    #[test]
    fn test_update_overlap_error() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        let c1 = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        create(&conn, "Seogi", "Sprint 2", "2026-05-15", "2026-05-28").unwrap();

        let result = update(&conn, c1.id(), None, None, Some("2026-05-20"));
        assert!(result.is_err());
    }

    // Q22: list 파생 status 포함
    #[test]
    fn test_list_with_derived_status() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        let rows = list(&conn, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            ["planned", "active", "completed"].contains(&rows[0].status.as_str()),
            "status should be derived: {}",
            rows[0].status
        );
    }

    #[test]
    fn test_create_success() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        assert_eq!(cycle.name(), "Sprint 1");

        let all = cycle_repo::list_all(&conn).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_create_unknown_workspace() {
        let conn = initialize_in_memory().unwrap();
        let result = create(&conn, "NonExistent", "Sprint 1", "2026-05-01", "2026-05-14");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_empty_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "", "2026-05-01", "2026-05-14");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_invalid_date() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "Sprint 1", "not-a-date", "2026-05-14");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_start_after_end() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let result = create(&conn, "Seogi", "Sprint 1", "2026-05-15", "2026-05-01");
        assert!(result.is_err());
    }

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

    #[test]
    fn test_list_all() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);

        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        create(&conn, "Seogi", "Sprint 2", "2026-05-15", "2026-05-28").unwrap();

        let rows = list(&conn, None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_update_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();

        update(&conn, cycle.id(), Some("Updated"), None, None).unwrap();

        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.name(), "Updated");
    }

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
    }

    #[test]
    fn test_update_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = update(&conn, "nonexistent", Some("name"), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_no_fields() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let result = update(&conn, cycle.id(), None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_invalid_date() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let result = update(&conn, cycle.id(), None, Some("bad-date"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_start_after_end() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let result = update(&conn, cycle.id(), None, Some("2026-06-01"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_empty_name() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let result = update(&conn, cycle.id(), Some(""), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_end_only() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        update(&conn, cycle.id(), None, None, Some("2026-05-21")).unwrap();
        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.end_date(), "2026-05-21");
    }

    #[test]
    fn test_update_start_only() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        let cycle = create(&conn, "Seogi", "Sprint 1", "2026-05-05", "2026-05-14").unwrap();
        update(&conn, cycle.id(), None, Some("2026-05-01"), None).unwrap();
        let found = cycle_repo::find_by_id(&conn, cycle.id()).unwrap().unwrap();
        assert_eq!(found.start_date(), "2026-05-01");
    }

    #[test]
    fn test_list_unknown_workspace() {
        let conn = initialize_in_memory().unwrap();
        setup_workspace(&conn);
        create(&conn, "Seogi", "Sprint 1", "2026-05-01", "2026-05-14").unwrap();
        let rows = list(&conn, Some("NonExistent")).unwrap();
        assert!(rows.is_empty());
    }
}
