use rusqlite::Connection;

use crate::domain::task::TaskEvent;

/// 태스크 이벤트를 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, event: &TaskEvent) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO task_events (id, task_id, from_status, to_status, session_id, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            event.id(),
            event.task_id(),
            event.from_status(),
            event.to_status(),
            event.session_id(),
            event.timestamp().value(),
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::{project_repo, status_repo, task_repo};
    use crate::domain::project::{Project, ProjectPrefix};
    use crate::domain::status::StatusCategory;
    use crate::domain::task::{Label, Task};
    use crate::domain::value::Timestamp;

    // Q17: save 후 DB에서 조회 시 필드 일치
    #[test]
    fn test_task_event_repo_save() {
        let conn = initialize_in_memory().unwrap();

        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", chrono::Utc::now()).unwrap();
        project_repo::save(&conn, &project).unwrap();

        let statuses = status_repo::list_all(&conn).unwrap();
        let backlog = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Backlog)
            .unwrap();

        let task = Task::new(
            &prefix,
            1,
            "title",
            "desc",
            Label::Feature,
            backlog.id(),
            project.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        task_repo::save(&conn, &task).unwrap();

        let event = TaskEvent::new("SEO-1", None, "backlog", "CLI", Timestamp::new(1_000_000));
        save(&conn, &event).unwrap();

        // DB에서 조회 검증
        let (task_id, from_status, to_status, session_id, timestamp): (
            String,
            Option<String>,
            String,
            String,
            i64,
        ) = conn
            .query_row(
                "SELECT task_id, from_status, to_status, session_id, timestamp FROM task_events WHERE id = ?1",
                [event.id()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();

        assert_eq!(task_id, "SEO-1");
        assert!(from_status.is_none());
        assert_eq!(to_status, "backlog");
        assert_eq!(session_id, "CLI");
        assert_eq!(timestamp, 1_000_000);
    }
}
