use rusqlite::Connection;

use super::error::AdapterError;

/// `changelog` 테이블에 한 행을 INSERT한다.
///
/// # Errors
///
/// DB 쓰기 실패 시 `AdapterError::Database`.
pub fn save_changelog(
    conn: &Connection,
    id: &str,
    description: &str,
    timestamp: i64,
) -> Result<(), AdapterError> {
    conn.execute(
        "INSERT INTO changelog (id, description, timestamp) VALUES (?1, ?2, ?3)",
        (id, description, timestamp),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;

    #[test]
    fn test_save_changelog() {
        let conn = db::initialize_in_memory().unwrap();

        save_changelog(
            &conn,
            "abcdef1234567890abcdef1234567890",
            "test change",
            1_713_000_000_000,
        )
        .unwrap();

        let (id, desc, ts): (String, String, i64) = conn
            .query_row(
                "SELECT id, description, timestamp FROM changelog LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        assert_eq!(id, "abcdef1234567890abcdef1234567890");
        assert_eq!(desc, "test change");
        assert_eq!(ts, 1_713_000_000_000);
    }
}
