use chrono::Utc;
use rusqlite::Connection;

use crate::adapter::workspace_repo;
use crate::domain::error::DomainError;
use crate::domain::workspace::{Workspace, WorkspacePrefix};

/// 프로젝트를 생성한다.
///
/// prefix가 `None`이면 이름에서 자동 생성한다.
/// 중복 prefix가 존재하면 에러를 반환한다.
///
/// # Errors
///
/// - prefix 검증 실패 → `DomainError::Validation`
/// - 중복 prefix → `DomainError::Validation`
/// - DB 에러 → `DomainError::Database`
pub fn create(
    conn: &Connection,
    name: &str,
    prefix: Option<&str>,
    goal: &str,
) -> Result<Workspace, DomainError> {
    let workspace_prefix = match prefix {
        Some(p) => WorkspacePrefix::new(p)?,
        None => WorkspacePrefix::from_name(name)?,
    };

    let existing = workspace_repo::find_by_prefix(conn, workspace_prefix.as_str())?;
    if existing.is_some() {
        return Err(DomainError::Validation(format!(
            "Project with prefix \"{}\" already exists",
            workspace_prefix.as_str()
        )));
    }

    let workspace = Workspace::new(name, &workspace_prefix, goal, Utc::now())?;
    workspace_repo::save(conn, &workspace)?;
    Ok(workspace)
}

/// 모든 프로젝트를 조회한다.
///
/// # Errors
///
/// DB 에러 → `DomainError::Database`.
pub fn list(conn: &Connection) -> Result<Vec<Workspace>, DomainError> {
    let workspaces = workspace_repo::list_all(conn)?;
    Ok(workspaces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;

    // Q14: workflow create → DB 저장 + 반환
    #[test]
    fn test_workflow_create_success() {
        let conn = initialize_in_memory().unwrap();
        let workspace = create(&conn, "Seogi", Some("SEO"), "하니스 계측").unwrap();

        assert_eq!(workspace.name(), "Seogi");
        assert_eq!(workspace.prefix().as_str(), "SEO");
        assert_eq!(workspace.next_seq(), 1);

        let all = workspace_repo::list_all(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id(), workspace.id());
    }

    // Q15: 중복 prefix → 에러, DB 미변경
    #[test]
    fn test_workflow_create_duplicate_prefix() {
        let conn = initialize_in_memory().unwrap();
        create(&conn, "Seogi", Some("SEO"), "하니스 계측").unwrap();

        let result = create(&conn, "Other", Some("SEO"), "다른 프로젝트");
        assert!(result.is_err());

        let all = workspace_repo::list_all(&conn).unwrap();
        assert_eq!(all.len(), 1);
    }

    // Q16: prefix None → 이름에서 자동 생성
    #[test]
    fn test_workflow_create_auto_prefix() {
        let conn = initialize_in_memory().unwrap();
        let workspace = create(&conn, "Seogi", None, "하니스 계측").unwrap();

        assert_eq!(workspace.prefix().as_str(), "SEO");
    }
}
