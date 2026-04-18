use rusqlite::Connection;

use crate::adapter::changelog_repo;
use crate::adapter::error::AdapterError;
use crate::domain::value::Timestamp;

/// changelog 추가 워크플로우.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError`.
pub fn run(conn: &Connection, description: &str) -> Result<Timestamp, AdapterError> {
    let id = uuid::Uuid::new_v4().simple().to_string();
    let timestamp = Timestamp::now();

    changelog_repo::save_changelog(conn, &id, description, timestamp.value())?;

    Ok(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    #[test]
    fn test_workflow_changelog_run() {
        let conn = db::initialize_in_memory().unwrap();

        let ts = run(&conn, "test description").unwrap();
        assert!(ts.value() > 0);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM changelog", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
