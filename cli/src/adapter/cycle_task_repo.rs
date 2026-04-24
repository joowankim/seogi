use rusqlite::Connection;

use crate::domain::cycle::Assigned;

/// Cycle에 태스크를 배정한다.
///
/// # Errors
///
/// INSERT 실패 (중복 PK 등) 시 `rusqlite::Error`.
pub fn save(
    conn: &Connection,
    cycle_id: &str,
    task_id: &str,
    assigned: Assigned,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO cycle_tasks (cycle_id, task_id, assigned) VALUES (?1, ?2, ?3)",
        (cycle_id, task_id, assigned.as_str()),
    )?;
    Ok(())
}

/// Cycle에서 태스크 배정을 해제한다.
///
/// 삭제된 행이 있으면 true, 없으면 false.
///
/// # Errors
///
/// DELETE 실패 시 `rusqlite::Error`.
pub fn delete(conn: &Connection, cycle_id: &str, task_id: &str) -> rusqlite::Result<bool> {
    let rows = conn.execute(
        "DELETE FROM cycle_tasks WHERE cycle_id = ?1 AND task_id = ?2",
        (cycle_id, task_id),
    )?;
    Ok(rows > 0)
}

/// 특정 Cycle에 태스크가 배정되어 있는지 확인한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn is_assigned_to_cycle(
    conn: &Connection,
    cycle_id: &str,
    task_id: &str,
) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cycle_tasks WHERE cycle_id = ?1 AND task_id = ?2",
        (cycle_id, task_id),
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

/// 태스크가 어떤 Cycle에든 배정되어 있는지 확인한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn is_task_in_any_cycle(conn: &Connection, task_id: &str) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cycle_tasks WHERE task_id = ?1",
        [task_id],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::cycle_repo;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::workspace_repo;
    use crate::domain::cycle::Cycle;
    use crate::domain::task::{Label, Task};
    use crate::domain::workspace::WorkspacePrefix;
    use chrono::Utc;

    fn setup(conn: &Connection) -> (String, String) {
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let ws =
            crate::domain::workspace::Workspace::new("Seogi", &prefix, "test", Utc::now()).unwrap();
        workspace_repo::save(conn, &ws).unwrap();

        let cycle =
            Cycle::new(ws.id(), "Sprint 1", "2026-05-01", "2026-05-14", Utc::now()).unwrap();
        cycle_repo::save(conn, &cycle).unwrap();

        let backlog = crate::adapter::status_repo::find_by_category(conn, "backlog")
            .unwrap()
            .unwrap();
        let task = Task::new(
            ws.prefix(),
            ws.next_seq(),
            "Task 1",
            "desc",
            Label::Feature,
            backlog.id(),
            ws.id(),
            Utc::now(),
        )
        .unwrap();
        crate::adapter::task_repo::save(conn, &task).unwrap();

        (cycle.id().to_string(), task.id().to_string())
    }

    // Q5: save 후 DB에 행 존재
    #[test]
    fn test_save() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cycle_tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    // Q6: save 중복 → rusqlite::Error
    #[test]
    fn test_save_duplicate() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();
        assert!(save(&conn, &cycle_id, &task_id, Assigned::Auto).is_err());
    }

    // Q7: delete 성공
    #[test]
    fn test_delete() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();
        assert!(delete(&conn, &cycle_id, &task_id).unwrap());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cycle_tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    // Q8: delete 미존재 → false
    #[test]
    fn test_delete_not_found() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        assert!(!delete(&conn, &cycle_id, &task_id).unwrap());
    }

    // Q9: is_assigned_to_cycle 배정됨
    #[test]
    fn test_is_assigned_to_cycle_true() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();
        assert!(is_assigned_to_cycle(&conn, &cycle_id, &task_id).unwrap());
    }

    // Q10: is_assigned_to_cycle 미배정
    #[test]
    fn test_is_assigned_to_cycle_false() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        assert!(!is_assigned_to_cycle(&conn, &cycle_id, &task_id).unwrap());
    }

    // Q11: is_task_in_any_cycle 배정됨
    #[test]
    fn test_is_task_in_any_cycle_true() {
        let conn = initialize_in_memory().unwrap();
        let (cycle_id, task_id) = setup(&conn);

        save(&conn, &cycle_id, &task_id, Assigned::Auto).unwrap();
        assert!(is_task_in_any_cycle(&conn, &task_id).unwrap());
    }

    // Q12: is_task_in_any_cycle 미배정
    #[test]
    fn test_is_task_in_any_cycle_false() {
        let conn = initialize_in_memory().unwrap();
        let (_cycle_id, task_id) = setup(&conn);

        assert!(!is_task_in_any_cycle(&conn, &task_id).unwrap());
    }
}
