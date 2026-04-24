use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::error::AdapterError;

const SCHEMA_SQL: &str = include_str!("sql/schema.sql");
const SEED_SQL: &str = include_str!("sql/seed.sql");
const MIGRATION_V2_TO_V3: &str = include_str!("sql/migration_v2_to_v3.sql");
const MIGRATION_V3_TO_V4: &str = include_str!("sql/migration_v3_to_v4.sql");
const MIGRATION_V4_TO_V5: &str = include_str!("sql/migration_v4_to_v5.sql");
const MIGRATION_V5_TO_V6: &str = include_str!("sql/migration_v5_to_v6.sql");
const MIGRATION_V6_TO_V7: &str = include_str!("sql/migration_v6_to_v7.sql");
const MIGRATION_V7_TO_V8: &str = include_str!("sql/migration_v7_to_v8.sql");
const MIGRATION_V8_TO_V9: &str = include_str!("sql/migration_v8_to_v9.sql");

const SCHEMA_VERSION: i64 = 9;

fn apply_schema(conn: &Connection) -> Result<(), AdapterError> {
    conn.execute_batch(SCHEMA_SQL)?;
    conn.execute_batch(SEED_SQL)?;
    Ok(())
}

fn migrate_v2_to_v3(conn: &Connection) -> Result<(), AdapterError> {
    conn.execute_batch(MIGRATION_V2_TO_V3)?;
    Ok(())
}

fn setup_connection(conn: Connection) -> Result<Connection, AdapterError> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    if version < SCHEMA_VERSION {
        if version > 0 && version < 3 {
            migrate_v2_to_v3(&conn)?;
        }
        if version == 3 {
            conn.execute_batch(MIGRATION_V3_TO_V4)?;
        }
        if version < 5 {
            conn.execute_batch(MIGRATION_V4_TO_V5)?;
        }
        if version < 6 {
            conn.execute_batch(MIGRATION_V5_TO_V6)?;
        }
        if version == 6 {
            conn.execute_batch(MIGRATION_V6_TO_V7)?;
        }
        if version < 8 {
            conn.execute_batch(MIGRATION_V7_TO_V8)?;
        }
        if version == 8 {
            conn.execute_batch(MIGRATION_V8_TO_V9)?;
        }
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

    use std::str::FromStr;

    use crate::domain::status::StatusCategory;

    fn insert_test_tool_use(conn: &Connection) {
        conn.execute(
            "INSERT INTO tool_uses (id, session_id, workspace, workspace_path, tool_name, tool_input, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            ("t1", "s1", "proj", "/path", "Bash", "{}", 100, 1000),
        )
        .unwrap();
    }

    const EXPECTED_TABLES: [&str; 11] = [
        "changelog",
        "cycle_tasks",
        "cycles",
        "statuses",
        "system_events",
        "task_dependencies",
        "task_events",
        "tasks",
        "tool_failures",
        "tool_uses",
        "workspaces",
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
    fn test_schema_columns_workspaces() {
        let conn = initialize_in_memory().unwrap();
        assert_table_columns(
            &conn,
            "workspaces",
            &[
                ("id", "TEXT", false),
                ("name", "TEXT", true),
                ("prefix", "TEXT", true),
                ("goal", "TEXT", true),
                ("next_seq", "INTEGER", true),
                ("created_at", "TEXT", true),
                ("updated_at", "TEXT", true),
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
                ("category", "TEXT", true),
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
                ("workspace_id", "TEXT", true),
                ("created_at", "TEXT", true),
                ("updated_at", "TEXT", true),
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
                ("workspace", "TEXT", true),
                ("workspace_path", "TEXT", true),
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
                ("workspace", "TEXT", true),
                ("workspace_path", "TEXT", true),
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
                ("workspace", "TEXT", true),
                ("workspace_path", "TEXT", true),
                ("event_type", "TEXT", true),
                ("content", "TEXT", true),
                ("timestamp", "INTEGER", true),
            ],
        );
    }

    #[test]
    fn test_schema_idempotent() {
        let conn = initialize_in_memory().unwrap();
        insert_test_tool_use(&conn);

        // user_version을 리셋하여 스키마 재적용 강제
        conn.pragma_update(None, "user_version", 0).unwrap();
        apply_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
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
        insert_test_tool_use(&conn1);
        drop(conn1);

        let conn2 = initialize_db(&db_path).unwrap();
        let count: i64 = conn2
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_seed_statuses_count() {
        let conn = initialize_in_memory().unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM statuses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn test_seed_statuses_data() {
        let conn = initialize_in_memory().unwrap();

        let mut stmt = conn
            .prepare("SELECT name, category, position FROM statuses ORDER BY position")
            .unwrap();
        let rows: Vec<(String, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();

        let expected = vec![
            ("backlog".to_string(), "backlog".to_string(), 0),
            ("todo".to_string(), "unstarted".to_string(), 1),
            ("in_progress".to_string(), "started".to_string(), 2),
            ("in_review".to_string(), "started".to_string(), 3),
            ("blocked".to_string(), "started".to_string(), 4),
            ("done".to_string(), "completed".to_string(), 5),
            ("canceled".to_string(), "canceled".to_string(), 6),
        ];

        assert_eq!(rows, expected);
    }

    #[test]
    fn test_seed_statuses_valid_categories() {
        let conn = initialize_in_memory().unwrap();

        let mut stmt = conn.prepare("SELECT category FROM statuses").unwrap();
        let categories: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        for cat in &categories {
            assert!(
                StatusCategory::from_str(cat).is_ok(),
                "invalid category in seed data: {cat}"
            );
        }
    }

    #[test]
    fn test_seed_idempotent() {
        let conn = initialize_in_memory().unwrap();

        // 재적용 강제
        conn.pragma_update(None, "user_version", 0).unwrap();
        apply_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM statuses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn test_schema_preserves_phase1_data() {
        let conn = initialize_in_memory().unwrap();
        insert_test_tool_use(&conn);

        // 재적용 강제
        conn.pragma_update(None, "user_version", 0).unwrap();
        apply_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v2_to_v3_drops_status_categories() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        // v2 스키마 시뮬레이션: status_categories 테이블 생성
        conn.execute_batch(
            "CREATE TABLE status_categories (id TEXT PRIMARY KEY, name TEXT NOT NULL, position INTEGER NOT NULL);",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();

        // v3로 업그레이드
        let conn = setup_connection(conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert!(!tables.contains(&"status_categories".to_string()));
        assert_eq!(tables, EXPECTED_TABLES);
    }

    #[test]
    fn test_migration_v2_to_v3_preserves_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        // v2 스키마 시뮬레이션
        conn.execute_batch(
            "CREATE TABLE status_categories (id TEXT PRIMARY KEY, name TEXT NOT NULL, position INTEGER NOT NULL);
             CREATE TABLE tool_uses (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project TEXT NOT NULL, project_path TEXT NOT NULL, tool_name TEXT NOT NULL, tool_input TEXT NOT NULL, duration_ms INTEGER NOT NULL, timestamp INTEGER NOT NULL);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tool_uses (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            ("t1", "s1", "proj", "/path", "Bash", "{}", 100, 1000),
        ).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();

        // v7로 업그레이드
        let conn = setup_connection(conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v3_to_v4_changes_timestamp_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        // v3 스키마 시뮬레이션: created_at/updated_at이 INTEGER인 projects 테이블
        conn.execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, prefix TEXT NOT NULL UNIQUE, goal TEXT NOT NULL, next_seq INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE tool_uses (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project TEXT NOT NULL, project_path TEXT NOT NULL, tool_name TEXT NOT NULL, tool_input TEXT NOT NULL, duration_ms INTEGER NOT NULL, timestamp INTEGER NOT NULL);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tool_uses (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            ("t1", "s1", "proj", "/path", "Bash", "{}", 100, 1000),
        ).unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();

        // v7로 업그레이드 (v3→v4→v5→v6→v7)
        let conn = setup_connection(conn).unwrap();

        // projects → workspaces, created_at이 TEXT로 변경됨
        assert_table_columns(
            &conn,
            "workspaces",
            &[
                ("id", "TEXT", false),
                ("name", "TEXT", true),
                ("prefix", "TEXT", true),
                ("goal", "TEXT", true),
                ("next_seq", "INTEGER", true),
                ("created_at", "TEXT", true),
                ("updated_at", "TEXT", true),
            ],
        );

        // Phase 1 데이터 보존
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v6_to_v7_renames_project_to_workspace() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        // v6 스키마 시뮬레이션
        conn.execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, prefix TEXT NOT NULL UNIQUE, goal TEXT NOT NULL, next_seq INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE statuses (id TEXT PRIMARY KEY, name TEXT NOT NULL, category TEXT NOT NULL, position INTEGER NOT NULL);
             CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL, label TEXT NOT NULL, status_id TEXT NOT NULL, project_id TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE tool_uses (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project TEXT NOT NULL, project_path TEXT NOT NULL, tool_name TEXT NOT NULL, tool_input TEXT NOT NULL, duration_ms INTEGER NOT NULL, timestamp INTEGER NOT NULL);
             CREATE TABLE tool_failures (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project TEXT NOT NULL, project_path TEXT NOT NULL, tool_name TEXT NOT NULL, error TEXT NOT NULL, timestamp INTEGER NOT NULL);
             CREATE TABLE system_events (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project TEXT NOT NULL, project_path TEXT NOT NULL, event_type TEXT NOT NULL, content TEXT NOT NULL, timestamp INTEGER NOT NULL);
             CREATE TABLE task_dependencies (task_id TEXT NOT NULL, depends_on_task_id TEXT NOT NULL, PRIMARY KEY (task_id, depends_on_task_id));",
        )
        .unwrap();

        // v6 데이터 삽입
        conn.execute(
            "INSERT INTO projects (id, name, prefix, goal, next_seq, created_at, updated_at) VALUES ('p1', 'Seogi', 'SEO', 'goal', 1, '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO tool_uses (id, session_id, project, project_path, tool_name, tool_input, duration_ms, timestamp) VALUES ('t1', 's1', 'seogi', '/path/seogi', 'Bash', '{}', 100, 1000)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO tool_failures (id, session_id, project, project_path, tool_name, error, timestamp) VALUES ('f1', 's1', 'seogi', '/path/seogi', 'Bash', 'err', 2000)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO system_events (id, session_id, project, project_path, event_type, content, timestamp) VALUES ('e1', 's1', 'seogi', '/path/seogi', 'Stop', 'done', 3000)",
            [],
        ).unwrap();

        conn.pragma_update(None, "user_version", 6).unwrap();

        // v7로 업그레이드
        let conn = setup_connection(conn).unwrap();

        // workspaces 테이블 존재, projects 없음
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('projects', 'workspaces') ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(tables, vec!["workspaces"]);

        // workspaces 데이터 보존
        let name: String = conn
            .query_row("SELECT name FROM workspaces WHERE id = 'p1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Seogi");

        // tasks.workspace_id 보존
        assert_table_columns(
            &conn,
            "tasks",
            &[
                ("id", "TEXT", false),
                ("title", "TEXT", true),
                ("description", "TEXT", true),
                ("label", "TEXT", true),
                ("status_id", "TEXT", true),
                ("workspace_id", "TEXT", true),
                ("created_at", "TEXT", true),
                ("updated_at", "TEXT", true),
            ],
        );

        // tool_uses workspace/workspace_path 보존
        let (ws, ws_path): (String, String) = conn
            .query_row(
                "SELECT workspace, workspace_path FROM tool_uses WHERE id = 't1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(ws, "seogi");
        assert_eq!(ws_path, "/path/seogi");

        // tool_failures workspace/workspace_path 보존
        let (ws, ws_path): (String, String) = conn
            .query_row(
                "SELECT workspace, workspace_path FROM tool_failures WHERE id = 'f1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(ws, "seogi");
        assert_eq!(ws_path, "/path/seogi");

        // system_events workspace/workspace_path 보존
        let (ws, ws_path): (String, String) = conn
            .query_row(
                "SELECT workspace, workspace_path FROM system_events WHERE id = 'e1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(ws, "seogi");
        assert_eq!(ws_path, "/path/seogi");

        // schema version = 7
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }
}
