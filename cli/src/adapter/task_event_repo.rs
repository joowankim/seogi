use rusqlite::Connection;

use super::mapper::task_event_from_row;
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

/// 특정 태스크의 모든 이벤트를 timestamp 순으로 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_by_task_id(conn: &Connection, task_id: &str) -> rusqlite::Result<Vec<TaskEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, from_status, to_status, session_id, timestamp \
         FROM task_events WHERE task_id = ?1 ORDER BY timestamp",
    )?;
    let rows = stmt.query_map([task_id], task_event_from_row)?;
    rows.collect()
}

/// 기간 내 Completed 카테고리로 전환된 이벤트를 조회한다.
///
/// statuses 테이블을 JOIN하여 `category = 'completed'`인 이벤트만 반환한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_completed_in_range(
    conn: &Connection,
    from_ts: i64,
    to_ts: i64,
) -> rusqlite::Result<Vec<TaskEvent>> {
    let mut stmt = conn.prepare(
        "SELECT te.id, te.task_id, te.from_status, te.to_status, te.session_id, te.timestamp \
         FROM task_events te \
         JOIN statuses s ON s.name = te.to_status \
         WHERE s.category = 'completed' \
           AND te.timestamp >= ?1 \
           AND te.timestamp <= ?2 \
         ORDER BY te.timestamp",
    )?;
    let rows = stmt.query_map([from_ts, to_ts], task_event_from_row)?;
    rows.collect()
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

    fn setup_task(conn: &Connection) -> (String, String) {
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", chrono::Utc::now()).unwrap();
        project_repo::save(conn, &project).unwrap();

        let statuses = status_repo::list_all(conn).unwrap();
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
        task_repo::save(conn, &task).unwrap();
        (task.id().to_string(), project.id().to_string())
    }

    // Q17: save 후 DB에서 조회 시 필드 일치
    #[test]
    fn test_task_event_repo_save() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

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

    // list_by_task_id: 이벤트 있는 경우
    #[test]
    fn test_list_by_task_id() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

        let e1 = TaskEvent::new("SEO-1", None, "backlog", "CLI", Timestamp::new(1000));
        let e2 = TaskEvent::new(
            "SEO-1",
            Some("backlog"),
            "todo",
            "CLI",
            Timestamp::new(2000),
        );
        save(&conn, &e1).unwrap();
        save(&conn, &e2).unwrap();

        let events = list_by_task_id(&conn, "SEO-1").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].to_status(), "backlog");
        assert_eq!(events[1].to_status(), "todo");
        // timestamp 순 정렬 확인
        assert!(events[0].timestamp().value() <= events[1].timestamp().value());
    }

    // list_by_task_id: 이벤트 없는 경우
    #[test]
    fn test_list_by_task_id_empty() {
        let conn = initialize_in_memory().unwrap();
        let events = list_by_task_id(&conn, "NONEXISTENT").unwrap();
        assert!(events.is_empty());
    }

    // list_by_task_id: from_status 보존
    #[test]
    fn test_list_by_task_id_preserves_from_status() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

        let e1 = TaskEvent::new("SEO-1", None, "backlog", "CLI", Timestamp::new(1000));
        let e2 = TaskEvent::new("SEO-1", Some("backlog"), "todo", "s1", Timestamp::new(2000));
        save(&conn, &e1).unwrap();
        save(&conn, &e2).unwrap();

        let events = list_by_task_id(&conn, "SEO-1").unwrap();
        assert!(events[0].from_status().is_none());
        assert_eq!(events[1].from_status(), Some("backlog"));
    }

    // list_completed_in_range: 기간 내 completed 이벤트
    #[test]
    fn test_list_completed_in_range() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

        // "done" status has category "completed" in seed data
        let e1 = TaskEvent::new(
            "SEO-1",
            Some("in_progress"),
            "done",
            "CLI",
            Timestamp::new(5000),
        );
        let e2 = TaskEvent::new(
            "SEO-1",
            Some("backlog"),
            "in_progress",
            "CLI",
            Timestamp::new(3000),
        );
        save(&conn, &e1).unwrap();
        save(&conn, &e2).unwrap();

        let events = list_completed_in_range(&conn, 1000, 10000).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].to_status(), "done");
    }

    // list_completed_in_range: 범위 밖 이벤트 제외
    #[test]
    fn test_list_completed_in_range_excludes_out_of_range() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

        let e1 = TaskEvent::new(
            "SEO-1",
            Some("in_progress"),
            "done",
            "CLI",
            Timestamp::new(5000),
        );
        save(&conn, &e1).unwrap();

        // 범위를 좁혀서 제외
        let events = list_completed_in_range(&conn, 6000, 10000).unwrap();
        assert!(events.is_empty());
    }

    // list_completed_in_range: 비completed 카테고리 제외
    #[test]
    fn test_list_completed_in_range_excludes_non_completed() {
        let conn = initialize_in_memory().unwrap();
        setup_task(&conn);

        // "in_progress" is started category, not completed
        let e1 = TaskEvent::new(
            "SEO-1",
            Some("backlog"),
            "in_progress",
            "CLI",
            Timestamp::new(3000),
        );
        save(&conn, &e1).unwrap();

        let events = list_completed_in_range(&conn, 1000, 10000).unwrap();
        assert!(events.is_empty());
    }

    // list_completed_in_range: 빈 결과
    #[test]
    fn test_list_completed_in_range_empty() {
        let conn = initialize_in_memory().unwrap();
        let events = list_completed_in_range(&conn, 1000, 10000).unwrap();
        assert!(events.is_empty());
    }
}
