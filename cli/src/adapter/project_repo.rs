use rusqlite::Connection;

use super::mapper::project_from_row;
use crate::domain::project::Project;

/// 프로젝트를 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, project: &Project) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO projects (id, name, prefix, goal, next_seq, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            project.id(),
            project.name(),
            project.prefix().as_str(),
            project.goal(),
            project.next_seq(),
            project.created_at().to_rfc3339(),
            project.updated_at().to_rfc3339(),
        ),
    )?;
    Ok(())
}

/// 모든 프로젝트를 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, prefix, goal, next_seq, created_at, updated_at FROM projects ORDER BY created_at",
    )?;
    let rows = stmt.query_map([], project_from_row)?;
    rows.collect()
}

/// prefix로 프로젝트를 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_by_prefix(conn: &Connection, prefix: &str) -> rusqlite::Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, prefix, goal, next_seq, created_at, updated_at FROM projects WHERE prefix = ?1",
    )?;
    let mut rows = stmt.query_map([prefix], project_from_row)?;
    rows.next().transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::domain::project::ProjectPrefix;

    fn sample_project(prefix_str: &str, name: &str) -> Project {
        let prefix = ProjectPrefix::new(prefix_str).unwrap();
        Project::new(name, &prefix, "test goal", chrono::Utc::now()).unwrap()
    }

    // Q10: save → list_all에 포함
    #[test]
    fn test_save_and_list_all() {
        let conn = initialize_in_memory().unwrap();
        let project = sample_project("SEO", "Seogi");
        save(&conn, &project).unwrap();

        let all = list_all(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].prefix().as_str(), "SEO");
        assert_eq!(all[0].name(), "Seogi");
    }

    // Q11: 빈 DB → 빈 Vec
    #[test]
    fn test_list_all_empty() {
        let conn = initialize_in_memory().unwrap();
        let all = list_all(&conn).unwrap();
        assert!(all.is_empty());
    }

    // Q12: 존재하는 prefix → Some(Project)
    #[test]
    fn test_find_by_prefix_found() {
        let conn = initialize_in_memory().unwrap();
        let project = sample_project("SEO", "Seogi");
        save(&conn, &project).unwrap();

        let found = find_by_prefix(&conn, "SEO").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Seogi");
    }

    // Q13: 없는 prefix → None
    #[test]
    fn test_find_by_prefix_not_found() {
        let conn = initialize_in_memory().unwrap();
        let found = find_by_prefix(&conn, "XYZ").unwrap();
        assert!(found.is_none());
    }
}
