use rusqlite::Connection;

use super::mapper::status_from_row;
use crate::domain::status::Status;

/// Status를 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, status: &Status) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO statuses (id, name, category, position) VALUES (?1, ?2, ?3, ?4)",
        (
            status.id(),
            status.name(),
            status.category().as_str(),
            status.position(),
        ),
    )?;
    Ok(())
}

/// 모든 Status를 position 순으로 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<Status>> {
    let mut stmt =
        conn.prepare("SELECT id, name, category, position FROM statuses ORDER BY position")?;
    let rows = stmt.query_map([], status_from_row)?;
    rows.collect()
}

/// id로 Status를 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<Status>> {
    let mut stmt =
        conn.prepare("SELECT id, name, category, position FROM statuses WHERE id = ?1")?;
    let mut rows = stmt.query_map([id], status_from_row)?;
    rows.next().transpose()
}

/// name을 변경한다. 변경된 행이 있으면 true.
///
/// # Errors
///
/// UPDATE 실패 시 `rusqlite::Error`.
pub fn update_name(conn: &Connection, id: &str, name: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute("UPDATE statuses SET name = ?1 WHERE id = ?2", (name, id))?;
    Ok(changed > 0)
}

/// Status를 삭제한다. 삭제된 행이 있으면 true.
///
/// # Errors
///
/// DELETE 실패 시 `rusqlite::Error`.
pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute("DELETE FROM statuses WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

/// 전체 최대 position을 반환한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn max_position(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row("SELECT MAX(position) FROM statuses", [], |r| r.get(0))
}

/// tasks에서 해당 `status_id`를 참조하고 있는지 확인한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn is_referenced_by_tasks(conn: &Connection, status_id: &str) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status_id = ?1",
        [status_id],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::domain::status::StatusCategory;

    fn sample_status(name: &str, category: StatusCategory, position: i64) -> Status {
        Status::new(name, category, position).unwrap()
    }

    // Q4, Q5: save → list_all에 포함, position 순 정렬
    #[test]
    fn test_save_and_list_all() {
        let conn = initialize_in_memory().unwrap();
        let status = sample_status("testing", StatusCategory::Started, 7);
        save(&conn, &status).unwrap();

        let all = list_all(&conn).unwrap();
        assert_eq!(all.len(), 8); // 7 seeded + 1
        assert_eq!(all[7].name(), "testing");
        assert_eq!(all[7].position(), 7);
    }

    // Q6: find_by_id 존재/미존재
    #[test]
    fn test_find_by_id() {
        let conn = initialize_in_memory().unwrap();

        let found = find_by_id(&conn, "00000000000000000000000000000001").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "backlog");

        let not_found = find_by_id(&conn, "nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    // Q7, Q8: update_name 존재/미존재
    #[test]
    fn test_update_name() {
        let conn = initialize_in_memory().unwrap();

        let changed = update_name(&conn, "00000000000000000000000000000001", "renamed").unwrap();
        assert!(changed);

        let status = find_by_id(&conn, "00000000000000000000000000000001")
            .unwrap()
            .unwrap();
        assert_eq!(status.name(), "renamed");

        let not_changed = update_name(&conn, "nonexistent", "renamed").unwrap();
        assert!(!not_changed);
    }

    // Q9, Q10: delete 존재/미존재
    #[test]
    fn test_delete() {
        let conn = initialize_in_memory().unwrap();

        let deleted = delete(&conn, "00000000000000000000000000000005").unwrap();
        assert!(deleted);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM statuses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 6);

        let not_deleted = delete(&conn, "nonexistent").unwrap();
        assert!(!not_deleted);
    }

    // Q11: max_position
    #[test]
    fn test_max_position() {
        let conn = initialize_in_memory().unwrap();

        let max = max_position(&conn).unwrap();
        assert_eq!(max, Some(6)); // 시딩 데이터 최대 position
    }

    // Q12: is_referenced_by_tasks
    #[test]
    fn test_is_referenced_by_tasks() {
        let conn = initialize_in_memory().unwrap();

        // 참조 없음
        let referenced = is_referenced_by_tasks(&conn, "00000000000000000000000000000001").unwrap();
        assert!(!referenced);

        // 더미 task 삽입으로 참조 생성
        conn.execute(
            "INSERT INTO projects (id, name, prefix, goal, next_seq, created_at, updated_at) VALUES ('p1', 'Test', 'TST', 'goal', 1, '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, description, label, status_id, project_id, created_at, updated_at) VALUES ('t1', 'task', 'desc', 'feature', '00000000000000000000000000000001', 'p1', '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        let referenced = is_referenced_by_tasks(&conn, "00000000000000000000000000000001").unwrap();
        assert!(referenced);
    }
}
