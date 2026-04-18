use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::error::AdapterError;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,
    goal        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS status_categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES status_categories(id),
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    label       TEXT NOT NULL,
    status_id   TEXT NOT NULL REFERENCES statuses(id),
    project_id  TEXT NOT NULL REFERENCES projects(id),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS task_events (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(id),
    from_status TEXT,
    to_status   TEXT NOT NULL,
    session_id  TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_uses (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    tool_input      TEXT NOT NULL,
    duration_ms     INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_failures (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    error           TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS system_events (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    content         TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS session_metrics (
    id                      TEXT PRIMARY KEY,
    session_id              TEXT NOT NULL,
    project                 TEXT NOT NULL,
    read_before_edit_ratio  INTEGER NOT NULL,
    doom_loop_count         INTEGER NOT NULL,
    test_invoked            INTEGER NOT NULL,
    build_invoked           INTEGER NOT NULL,
    lint_invoked            INTEGER NOT NULL,
    typecheck_invoked       INTEGER NOT NULL,
    tool_call_count         INTEGER NOT NULL,
    session_duration_ms     INTEGER NOT NULL,
    edit_files              TEXT NOT NULL,
    bash_error_rate         REAL NOT NULL,
    timestamp               INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS changelog (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);
";

const SCHEMA_VERSION: i64 = 2;

fn apply_schema(conn: &Connection) -> Result<(), AdapterError> {
    Ok(conn.execute_batch(SCHEMA_SQL)?)
}

fn setup_connection(conn: Connection) -> Result<Connection, AdapterError> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    if version < SCHEMA_VERSION {
        apply_schema(&conn)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }

    Ok(conn)
}

/// 지정된 경로에 `SQLite` DB를 생성하고 스키마를 적용한다.
/// 부모 디렉토리가 없으면 자동 생성한다.
///
/// # Errors
///
/// 디렉토리 생성 실패 시 `AdapterError::Io`, DB 연결/스키마 적용 실패 시 `AdapterError::Database`.
pub fn initialize_db(path: &Path) -> Result<Connection, AdapterError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;
    setup_connection(conn)
}

/// 테스트용 인메모리 `SQLite` DB를 생성하고 스키마를 적용한다.
///
/// # Errors
///
/// DB 연결 또는 스키마 적용 실패 시 `AdapterError::Database`.
pub fn initialize_in_memory() -> Result<Connection, AdapterError> {
    let conn = Connection::open_in_memory()?;
    setup_connection(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_TABLES: [&str; 10] = [
        "changelog",
        "projects",
        "session_metrics",
        "status_categories",
        "statuses",
        "system_events",
        "task_events",
        "tasks",
        "tool_failures",
        "tool_uses",
    ];

    #[test]
    fn test_schema_creates_all_tables() {
        let conn = initialize_in_memory().unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(tables, EXPECTED_TABLES);
    }

    type ColumnSpec = (&'static str, &'static str, bool);

    fn assert_table_columns(conn: &Connection, table: &str, expected: &[ColumnSpec]) {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let actual: Vec<(String, String, bool)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();

        let expected: Vec<(String, String, bool)> = expected
            .iter()
            .map(|(name, typ, notnull)| ((*name).to_string(), (*typ).to_string(), *notnull))
            .collect();

        assert_eq!(actual, expected, "Column mismatch in table {table}");
    }

    #[test]
    fn test_schema_columns_projects() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "projects",
            &[
                ("id", "TEXT", false),
                ("name", "TEXT", true),
                ("prefix", "TEXT", true),
                ("goal", "TEXT", true),
                ("created_at", "INTEGER", true),
                ("updated_at", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_status_categories() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "status_categories",
            &[
                ("id", "TEXT", false),
                ("name", "TEXT", true),
                ("position", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_statuses() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "statuses",
            &[
                ("id", "TEXT", false),
                ("name", "TEXT", true),
                ("category_id", "TEXT", true),
                ("position", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_tasks() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "tasks",
            &[
                ("id", "TEXT", false),
                ("title", "TEXT", true),
                ("description", "TEXT", true),
                ("label", "TEXT", true),
                ("status_id", "TEXT", true),
                ("project_id", "TEXT", true),
                ("created_at", "INTEGER", true),
                ("updated_at", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_task_events() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "task_events",
            &[
                ("id", "TEXT", false),
                ("task_id", "TEXT", true),
                ("from_status", "TEXT", false),
                ("to_status", "TEXT", true),
                ("session_id", "TEXT", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_tool_uses() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "tool_uses",
            &[
                ("id", "TEXT", false),
                ("session_id", "TEXT", true),
                ("project", "TEXT", true),
                ("project_path", "TEXT", true),
                ("tool_name", "TEXT", true),
                ("tool_input", "TEXT", true),
                ("duration_ms", "INTEGER", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_tool_failures() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "tool_failures",
            &[
                ("id", "TEXT", false),
                ("session_id", "TEXT", true),
                ("project", "TEXT", true),
                ("project_path", "TEXT", true),
                ("tool_name", "TEXT", true),
                ("error", "TEXT", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_system_events() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "system_events",
            &[
                ("id", "TEXT", false),
                ("session_id", "TEXT", true),
                ("project", "TEXT", true),
                ("project_path", "TEXT", true),
                ("event_type", "TEXT", true),
                ("content", "TEXT", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_columns_session_metrics() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "session_metrics",
            &[
                ("id", "TEXT", false),
                ("session_id", "TEXT", true),
                ("project", "TEXT", true),
                ("read_before_edit_ratio", "INTEGER", true),
                ("doom_loop_count", "INTEGER", true),
                ("test_invoked", "INTEGER", true),
                ("build_invoked", "INTEGER", true),
                ("lint_invoked", "INTEGER", true),
                ("typecheck_invoked", "INTEGER", true),
                ("tool_call_count", "INTEGER", true),
                ("session_duration_ms", "INTEGER", true),
                ("edit_files", "TEXT", true),
                ("bash_error_rate", "REAL", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_idempotent() {
        let conn = initialize_in_memory().unwrap();

        conn.execute(
            "INSERT INTO projects (id, name, prefix, goal, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            ("p1", "test", "TST", "goal", 1000, 1000),
        )
        .unwrap();

        // user_version을 리셋하여 스키마 재적용 강제
        conn.pragma_update(None, "user_version", 0).unwrap();
        apply_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let conn = initialize_in_memory().unwrap();

        let fk: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn test_schema_version_set_after_init() {
        let conn = initialize_in_memory().unwrap();

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_initialize_db_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("a").join("b").join("seogi.db");

        let result = initialize_db(&db_path);
        assert!(result.is_ok());
        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn test_initialize_db_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("seogi.db");

        let conn1 = initialize_db(&db_path).unwrap();
        conn1
            .execute(
                "INSERT INTO projects (id, name, prefix, goal, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                ("p1", "test", "TST", "goal", 1000, 1000),
            )
            .unwrap();
        drop(conn1);

        let conn2 = initialize_db(&db_path).unwrap();
        let count: i64 = conn2
            .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
