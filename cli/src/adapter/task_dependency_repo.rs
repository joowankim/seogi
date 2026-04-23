use rusqlite::Connection;

/// 의존 관계를 저장한다.
///
/// # Errors
///
/// INSERT 실패 (중복 포함) 시 `rusqlite::Error`.
pub fn save(conn: &Connection, task_id: &str, depends_on_task_id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO task_dependencies (task_id, depends_on_task_id) VALUES (?1, ?2)",
        (task_id, depends_on_task_id),
    )?;
    Ok(())
}

/// 의존 관계를 삭제한다. 삭제된 행이 있으면 true.
///
/// # Errors
///
/// DELETE 실패 시 `rusqlite::Error`.
pub fn delete(
    conn: &Connection,
    task_id: &str,
    depends_on_task_id: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "DELETE FROM task_dependencies WHERE task_id = ?1 AND depends_on_task_id = ?2",
        (task_id, depends_on_task_id),
    )?;
    Ok(changed > 0)
}

/// 태스크의 의존 대상 ID 목록을 반환한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_dependencies(conn: &Connection, task_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT depends_on_task_id FROM task_dependencies WHERE task_id = ?1")?;
    let rows = stmt.query_map([task_id], |row| row.get::<_, String>(0))?;
    rows.collect()
}

/// 전체 간선 목록을 반환한다 (순환 검증용).
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_all_edges(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT task_id, depends_on_task_id FROM task_dependencies")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

/// 미완료 의존성이 있는 태스크 ID 목록을 반환한다.
///
/// 의존 대상의 status category가 completed/canceled이 아닌 경우 blocked.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_blocked_task_ids(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let sql = "SELECT DISTINCT td.task_id \
               FROM task_dependencies td \
               JOIN tasks t ON td.depends_on_task_id = t.id \
               JOIN statuses s ON t.status_id = s.id \
               WHERE s.category NOT IN ('completed', 'canceled')";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::{task_repo, workspace_repo};
    use crate::domain::task::{Label, Task};
    use crate::domain::workspace::{Workspace, WorkspacePrefix};

    fn setup() -> Connection {
        let conn = initialize_in_memory().unwrap();
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let workspace = Workspace::new("Seogi", &prefix, "goal", chrono::Utc::now()).unwrap();
        workspace_repo::save(&conn, &workspace).unwrap();

        let statuses = crate::adapter::status_repo::list_all(&conn).unwrap();
        let backlog = statuses.iter().find(|s| s.name() == "backlog").unwrap();

        let task1 = Task::new(
            &prefix,
            1,
            "task1",
            "desc1",
            Label::Feature,
            backlog.id(),
            workspace.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        let task2 = Task::new(
            &prefix,
            2,
            "task2",
            "desc2",
            Label::Feature,
            backlog.id(),
            workspace.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        let task3 = Task::new(
            &prefix,
            3,
            "task3",
            "desc3",
            Label::Feature,
            backlog.id(),
            workspace.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        task_repo::save(&conn, &task1).unwrap();
        task_repo::save(&conn, &task2).unwrap();
        task_repo::save(&conn, &task3).unwrap();

        conn
    }

    // Q1: save 성공
    #[test]
    fn test_task_dependency_repo_save() {
        let conn = setup();
        save(&conn, "SEO-2", "SEO-1").unwrap();

        let deps = list_dependencies(&conn, "SEO-2").unwrap();
        assert_eq!(deps, vec!["SEO-1"]);
    }

    // Q2: save 중복 → 에러
    #[test]
    fn test_task_dependency_repo_save_duplicate() {
        let conn = setup();
        save(&conn, "SEO-2", "SEO-1").unwrap();
        assert!(save(&conn, "SEO-2", "SEO-1").is_err());
    }

    // Q3: delete 존재하는 관계 → true
    #[test]
    fn test_task_dependency_repo_delete_found() {
        let conn = setup();
        save(&conn, "SEO-2", "SEO-1").unwrap();
        assert!(delete(&conn, "SEO-2", "SEO-1").unwrap());
    }

    // Q4: delete 없는 관계 → false
    #[test]
    fn test_task_dependency_repo_delete_not_found() {
        let conn = setup();
        assert!(!delete(&conn, "SEO-2", "SEO-1").unwrap());
    }

    // Q5: list_dependencies 목록 반환
    #[test]
    fn test_task_dependency_repo_list_dependencies() {
        let conn = setup();
        save(&conn, "SEO-3", "SEO-1").unwrap();
        save(&conn, "SEO-3", "SEO-2").unwrap();

        let mut deps = list_dependencies(&conn, "SEO-3").unwrap();
        deps.sort();
        assert_eq!(deps, vec!["SEO-1", "SEO-2"]);
    }

    // Q6: list_all_edges 전체 간선 반환
    #[test]
    fn test_task_dependency_repo_list_all_edges() {
        let conn = setup();
        save(&conn, "SEO-2", "SEO-1").unwrap();
        save(&conn, "SEO-3", "SEO-2").unwrap();

        let edges = list_all_edges(&conn).unwrap();
        assert_eq!(edges.len(), 2);
    }
}
