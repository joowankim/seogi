use std::str::FromStr;

use chrono::Utc;
use rusqlite::Connection;

use crate::adapter::{project_repo, status_repo, task_event_repo, task_repo};
use crate::domain::error::DomainError;
use crate::domain::status::StatusCategory;
use crate::domain::task::{CLI_SESSION_ID, Label, Task, TaskEvent};
use crate::domain::value::Timestamp;

/// 태스크를 생성한다.
///
/// 프로젝트 조회 → backlog 상태 할당 → `next_seq` 채번 → Task 저장 →
/// `TaskEvent` 기록 → `next_seq` 증가.
///
/// # Errors
///
/// 프로젝트 미존재, 무효 라벨, backlog 상태 부재, DB 에러 시 `DomainError`.
pub fn create(
    conn: &Connection,
    project_name: &str,
    title: &str,
    description: &str,
    label_str: &str,
) -> Result<Task, DomainError> {
    let label = Label::from_str(label_str)?;

    let project = project_repo::find_by_name(conn, project_name)?
        .ok_or_else(|| DomainError::Validation(format!("Project not found: \"{project_name}\"")))?;

    let backlog_status = status_repo::find_by_category(conn, StatusCategory::Backlog.as_str())?
        .ok_or_else(|| {
            DomainError::Validation("No status with backlog category found".to_string())
        })?;

    let now = Utc::now();
    let task = Task::new(
        project.prefix(),
        project.next_seq(),
        title,
        description,
        label,
        backlog_status.id(),
        project.id(),
        now,
    )?;

    task_repo::save(conn, &task)?;

    let timestamp = Timestamp::new(now.timestamp_millis());
    let event = TaskEvent::new(
        task.id(),
        None,
        backlog_status.name(),
        CLI_SESSION_ID,
        timestamp,
    );
    task_event_repo::save(conn, &event)?;

    project_repo::increment_next_seq(conn, project.id())?;

    Ok(task)
}

/// 태스크 목록을 조회한다.
///
/// # Errors
///
/// 무효 라벨, DB 에러 시 `DomainError`.
pub fn list(
    conn: &Connection,
    project_name: Option<&str>,
    status_name: Option<&str>,
    label_str: Option<&str>,
) -> Result<Vec<task_repo::TaskListRow>, DomainError> {
    if let Some(l) = label_str {
        Label::from_str(l)?;
    }
    let filter = task_repo::TaskFilter {
        project_name,
        status_name,
        label: label_str,
    };
    let rows = task_repo::list_all(conn, &filter)?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::project_repo;
    use crate::domain::project::{Project, ProjectPrefix};

    fn setup_project(conn: &Connection) {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", Utc::now()).unwrap();
        project_repo::save(conn, &project).unwrap();
    }

    // Q19: create 성공 시 Task 반환, next_seq 증가, task_events 1건
    #[test]
    fn test_create_task_success() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);

        let task = create(&conn, "Seogi", "title", "desc", "feature").unwrap();
        assert_eq!(task.id(), "SEO-1");
        assert_eq!(task.label(), Label::Feature);

        // next_seq 증가 확인
        let project = project_repo::find_by_name(&conn, "Seogi").unwrap().unwrap();
        assert_eq!(project.next_seq(), 2);

        // task_events 확인
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let (to_status, session_id): (String, String) = conn
            .query_row(
                "SELECT to_status, session_id FROM task_events LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(to_status, "backlog");
        assert_eq!(session_id, "CLI");
    }

    // Q20: 존재하지 않는 프로젝트 → 에러
    #[test]
    fn test_create_task_unknown_project() {
        let conn = initialize_in_memory().unwrap();
        let result = create(&conn, "NonExistent", "title", "desc", "feature");
        assert!(result.is_err());
    }

    // Q21: 무효 라벨 → 에러
    #[test]
    fn test_create_task_invalid_label() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        let result = create(&conn, "Seogi", "title", "desc", "invalid");
        assert!(result.is_err());
    }

    // Q22: backlog 상태 부재 → 에러
    #[test]
    fn test_create_task_no_backlog_status() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);

        // backlog 카테고리 상태 삭제
        conn.execute("DELETE FROM statuses WHERE category = 'backlog'", [])
            .unwrap();

        let result = create(&conn, "Seogi", "title", "desc", "feature");
        assert!(result.is_err());
    }

    // Q23: list 필터 적용
    #[test]
    fn test_list_tasks_with_filter() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);

        create(&conn, "Seogi", "feat task", "desc", "feature").unwrap();
        create(&conn, "Seogi", "bug task", "desc", "bug").unwrap();

        let all = list(&conn, None, None, None).unwrap();
        assert_eq!(all.len(), 2);

        let features = list(&conn, None, None, Some("feature")).unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].title, "feat task");

        let by_project = list(&conn, Some("Seogi"), None, None).unwrap();
        assert_eq!(by_project.len(), 2);

        let by_status = list(&conn, None, Some("backlog"), None).unwrap();
        assert_eq!(by_status.len(), 2);
    }
}
