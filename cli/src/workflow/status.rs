use std::str::FromStr;

use rusqlite::Connection;

use crate::adapter::status_repo;
use crate::domain::error::DomainError;
use crate::domain::status::{Status, StatusCategory};

/// Status를 생성한다. position은 전체 max + 1로 자동 부여.
///
/// # Errors
///
/// 카테고리 검증 실패, 빈 이름, DB 에러 시 `DomainError`.
pub fn create(conn: &Connection, category_str: &str, name: &str) -> Result<Status, DomainError> {
    let category = StatusCategory::from_str(category_str)?;
    let max = status_repo::max_position(conn)?;
    let position = max.map_or(0, |m| m + 1);
    let status = Status::new(name, category, position)?;
    status_repo::save(conn, &status)?;
    Ok(status)
}

/// 모든 Status를 조회한다.
///
/// # Errors
///
/// DB 에러 시 `DomainError::Database`.
pub fn list(conn: &Connection) -> Result<Vec<Status>, DomainError> {
    let statuses = status_repo::list_all(conn)?;
    Ok(statuses)
}

/// Status 이름을 변경한다.
///
/// # Errors
///
/// 빈 이름, 존재하지 않는 id, DB 에러 시 `DomainError`.
pub fn update(conn: &Connection, id: &str, name: &str) -> Result<(), DomainError> {
    if name.is_empty() {
        return Err(DomainError::Validation(
            "Status name must not be empty".to_string(),
        ));
    }
    let changed = status_repo::update_name(conn, id, name)?;
    if !changed {
        return Err(DomainError::Validation(format!("Status not found: {id}")));
    }
    Ok(())
}

/// Status를 삭제한다. tasks에서 참조 중이면 거부.
///
/// # Errors
///
/// 존재하지 않는 id, tasks 참조 중, DB 에러 시 `DomainError`.
pub fn delete(conn: &Connection, id: &str) -> Result<(), DomainError> {
    let existing = status_repo::find_by_id(conn, id)?;
    if existing.is_none() {
        return Err(DomainError::Validation(format!("Status not found: {id}")));
    }
    if status_repo::is_referenced_by_tasks(conn, id)? {
        return Err(DomainError::Validation(
            "Cannot delete: status is used by tasks".to_string(),
        ));
    }
    status_repo::delete(conn, id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;

    // Q13: position 자동 부여 (시딩 후 7)
    #[test]
    fn test_workflow_create_auto_position() {
        let conn = initialize_in_memory().unwrap();
        let status = create(&conn, "started", "testing").unwrap();
        assert_eq!(status.position(), 7);
        assert_eq!(status.category(), StatusCategory::Started);
    }

    // Q14: 잘못된 category → 에러
    #[test]
    fn test_workflow_create_invalid_category() {
        let conn = initialize_in_memory().unwrap();
        let result = create(&conn, "invalid", "testing");
        assert!(result.is_err());
    }

    // Q15: update name 변경
    #[test]
    fn test_workflow_update_success() {
        let conn = initialize_in_memory().unwrap();
        let id = "00000000000000000000000000000001";
        update(&conn, id, "renamed").unwrap();

        let status = status_repo::find_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(status.name(), "renamed");
    }

    // Q15a: update 빈 이름 → 에러
    #[test]
    fn test_workflow_update_empty_name() {
        let conn = initialize_in_memory().unwrap();
        let result = update(&conn, "00000000000000000000000000000001", "");
        assert!(result.is_err());
    }

    // Q16: update 없는 id → 에러
    #[test]
    fn test_workflow_update_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = update(&conn, "nonexistent", "renamed");
        assert!(result.is_err());
    }

    // Q17: delete 성공
    #[test]
    fn test_workflow_delete_success() {
        let conn = initialize_in_memory().unwrap();
        delete(&conn, "00000000000000000000000000000005").unwrap();

        let found = status_repo::find_by_id(&conn, "00000000000000000000000000000005").unwrap();
        assert!(found.is_none());
    }

    // Q18: delete 없는 id → 에러
    #[test]
    fn test_workflow_delete_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = delete(&conn, "nonexistent");
        assert!(result.is_err());
    }

    // Q19: tasks 참조 → 에러, DB 미변경
    #[test]
    fn test_workflow_delete_referenced() {
        let conn = initialize_in_memory().unwrap();
        let status_id = "00000000000000000000000000000001";

        // 더미 project + task 삽입
        conn.execute(
            "INSERT INTO projects (id, name, prefix, goal, next_seq, created_at, updated_at) VALUES ('p1', 'Test', 'TST', 'goal', 1, '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, description, label, status_id, project_id, created_at, updated_at) VALUES ('t1', 'task', 'desc', 'feature', ?1, 'p1', '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [status_id],
        )
        .unwrap();

        let result = delete(&conn, status_id);
        assert!(result.is_err());

        // DB 미변경 확인
        let found = status_repo::find_by_id(&conn, status_id).unwrap();
        assert!(found.is_some());
    }
}
