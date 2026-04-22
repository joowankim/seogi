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

/// 단일 태스크를 조회한다.
///
/// # Errors
///
/// 태스크 미존재, DB 에러 시 `DomainError`.
pub fn get(conn: &Connection, task_id: &str) -> Result<task_repo::TaskListRow, DomainError> {
    let row = task_repo::find_by_id_detailed(conn, task_id)?;
    row.ok_or_else(|| DomainError::Validation(format!("Task not found: \"{task_id}\"")))
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

/// 태스크를 업데이트한다.
///
/// 지정된 필드만 변경하고 `updated_at`을 갱신한다.
///
/// # Errors
///
/// 옵션 미지정, 빈 title/description, 무효 라벨, 태스크 미존재, DB 에러 시 `DomainError`.
pub fn update(
    conn: &Connection,
    task_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    label_str: Option<&str>,
) -> Result<(), DomainError> {
    if title.is_none() && description.is_none() && label_str.is_none() {
        return Err(DomainError::Validation(
            "At least one of --title, --description, or --label must be specified".to_string(),
        ));
    }
    if let Some("") = title {
        return Err(DomainError::Validation(
            "Task title must not be empty".to_string(),
        ));
    }
    if let Some("") = description {
        return Err(DomainError::Validation(
            "Task description must not be empty".to_string(),
        ));
    }
    if let Some(l) = label_str {
        Label::from_str(l)?;
    }

    let params = task_repo::TaskUpdate {
        title,
        description,
        label: label_str,
    };
    let now = Utc::now();
    let changed = task_repo::update(conn, task_id, &params, &now)?;
    if !changed {
        return Err(DomainError::Validation(format!(
            "Task not found: \"{task_id}\""
        )));
    }
    Ok(())
}

/// 태스크 상태를 전환한다.
///
/// FSM 규칙을 검증하고, `status_id`를 변경하고, `task_events`에 기록한다.
///
/// # Errors
///
/// 태스크/상태 미존재, 같은 상태, FSM 위반, DB 에러 시 `DomainError`.
pub fn move_task(
    conn: &Connection,
    task_id: &str,
    target_status_name: &str,
) -> Result<(String, String), DomainError> {
    let task_row = task_repo::find_by_id(conn, task_id)?
        .ok_or_else(|| DomainError::Validation(format!("Task not found: \"{task_id}\"")))?;

    let current_status = status_repo::find_by_id(conn, &task_row.status_id)?
        .ok_or_else(|| DomainError::Validation("Current status not found".to_string()))?;

    let target_status = status_repo::find_by_name(conn, target_status_name)?.ok_or_else(|| {
        DomainError::Validation(format!("Status not found: \"{target_status_name}\""))
    })?;

    if current_status.id() == target_status.id() {
        return Err(DomainError::Validation(format!(
            "Task is already in status \"{target_status_name}\""
        )));
    }

    if !current_status
        .category()
        .can_transition_to(target_status.category())
    {
        let allowed: Vec<&str> = current_status
            .category()
            .allowed_transitions()
            .iter()
            .map(StatusCategory::as_str)
            .collect();
        return Err(DomainError::Validation(format!(
            "Cannot transition from {} ({}) to {} ({}). Allowed: {}",
            current_status.name(),
            current_status.category(),
            target_status.name(),
            target_status.category(),
            allowed.join(", ")
        )));
    }

    let now = Utc::now();
    let changed = task_repo::update_status(conn, task_id, target_status.id(), &now)?;
    if !changed {
        return Err(DomainError::Validation(format!(
            "Task not found: \"{task_id}\""
        )));
    }

    let timestamp = Timestamp::new(now.timestamp_millis());
    let event = TaskEvent::new(
        task_id,
        Some(current_status.name()),
        target_status.name(),
        CLI_SESSION_ID,
        timestamp,
    );
    task_event_repo::save(conn, &event)?;

    Ok((
        current_status.name().to_string(),
        target_status.name().to_string(),
    ))
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

    // Q3: get 존재하는 태스크 → Ok(TaskListRow)
    #[test]
    fn test_get_task_success() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let row = get(&conn, "SEO-1").unwrap();
        assert_eq!(row.id, "SEO-1");
        assert_eq!(row.title, "title");
        assert_eq!(row.description, "desc");
        assert_eq!(row.label, "feature");
        assert_eq!(row.status_name, "backlog");
        assert_eq!(row.project_name, "Seogi");
    }

    // Q4: get 없는 태스크 → DomainError
    #[test]
    fn test_get_task_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = get(&conn, "SEO-99");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Task not found"), "err: {err}");
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

    // Q6: update 성공
    #[test]
    fn test_update_task_success() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = update(&conn, "SEO-1", Some("new title"), None, None);
        assert!(result.is_ok());

        let rows = list(&conn, None, None, None).unwrap();
        assert_eq!(rows[0].title, "new title");
    }

    // Q7: update 존재하지 않는 태스크 → 에러
    #[test]
    fn test_update_task_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = update(&conn, "SEO-99", Some("new"), None, None);
        assert!(result.is_err());
    }

    // Q8: update 옵션 미지정 → 에러
    #[test]
    fn test_update_task_no_options() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = update(&conn, "SEO-1", None, None, None);
        assert!(result.is_err());
    }

    // Q9: update 빈 title → 에러
    #[test]
    fn test_update_task_empty_title() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = update(&conn, "SEO-1", Some(""), None, None);
        assert!(result.is_err());
    }

    // Q10: update 무효 label → 에러
    #[test]
    fn test_update_task_invalid_label() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = update(&conn, "SEO-1", None, None, Some("invalid"));
        assert!(result.is_err());
    }

    // Q13: move_task 허용 전환 → 성공, task_events 기록
    #[test]
    fn test_move_task_success() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let (from, to) = move_task(&conn, "SEO-1", "todo").unwrap();
        assert_eq!(from, "backlog");
        assert_eq!(to, "todo");

        // task_events: create + move = 2건
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    // Q14: move_task 태스크 미존재 → 에러
    #[test]
    fn test_move_task_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = move_task(&conn, "SEO-99", "todo");
        assert!(result.is_err());
    }

    // Q15: move_task 상태 미존재 → 에러
    #[test]
    fn test_move_task_status_not_found() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = move_task(&conn, "SEO-1", "nonexistent");
        assert!(result.is_err());
    }

    // Q16: move_task 허용되지 않은 전환 → 에러
    #[test]
    fn test_move_task_invalid_transition() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        // backlog → done (Backlog→Completed: 불가)
        let result = move_task(&conn, "SEO-1", "done");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Cannot transition"), "err: {err}");
    }

    // Q17: move_task 같은 상태 → 에러
    #[test]
    fn test_move_task_same_status() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        let result = move_task(&conn, "SEO-1", "backlog");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already"), "err: {err}");
    }

    // Q18: move_task 같은 카테고리 내 전환 → 성공
    #[test]
    fn test_move_task_same_category() {
        let conn = initialize_in_memory().unwrap();
        setup_project(&conn);
        create(&conn, "Seogi", "title", "desc", "feature").unwrap();

        // backlog → todo (Backlog→Unstarted)
        move_task(&conn, "SEO-1", "todo").unwrap();
        // todo → in_progress (Unstarted→Started)
        move_task(&conn, "SEO-1", "in_progress").unwrap();
        // in_progress → in_review (Started→Started: 같은 카테고리)
        let (from, to) = move_task(&conn, "SEO-1", "in_review").unwrap();
        assert_eq!(from, "in_progress");
        assert_eq!(to, "in_review");
    }
}
