use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::mapper::task_list_row_from_row;
use crate::domain::task::Task;

/// 조회 시 status/project 이름을 포함하는 구조체.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskListRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub label: String,
    pub status_name: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 태스크 목록 조회 필터.
pub struct TaskFilter<'a> {
    pub project_name: Option<&'a str>,
    pub status_name: Option<&'a str>,
    pub label: Option<&'a str>,
}

/// 태스크를 DB에 저장한다.
///
/// # Errors
///
/// INSERT 실패 시 `rusqlite::Error`.
pub fn save(conn: &Connection, task: &Task) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tasks (id, title, description, label, status_id, project_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            task.id(),
            task.title(),
            task.description(),
            task.label().as_str(),
            task.status_id(),
            task.project_id(),
            task.created_at().to_rfc3339(),
            task.updated_at().to_rfc3339(),
        ),
    )?;
    Ok(())
}

/// 필터를 적용하여 태스크 목록을 조회한다.
///
/// status, project 이름을 JOIN으로 포함한다. `created_at` DESC 정렬.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn list_all(conn: &Connection, filter: &TaskFilter<'_>) -> rusqlite::Result<Vec<TaskListRow>> {
    let mut sql = String::from(
        "SELECT t.id, t.title, t.description, t.label, s.name AS status_name, p.name AS project_name, t.created_at, t.updated_at \
         FROM tasks t \
         JOIN statuses s ON t.status_id = s.id \
         JOIN projects p ON t.project_id = p.id \
         WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(project_name) = filter.project_name {
        sql.push_str(" AND p.name = ?");
        params.push(Box::new(project_name.to_string()));
    }
    if let Some(status_name) = filter.status_name {
        sql.push_str(" AND s.name = ?");
        params.push(Box::new(status_name.to_string()));
    }
    if let Some(label) = filter.label {
        sql.push_str(" AND t.label = ?");
        params.push(Box::new(label.to_string()));
    }

    sql.push_str(" ORDER BY t.created_at DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(std::convert::AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), task_list_row_from_row)?;
    rows.collect()
}

/// 태스크의 `id`와 `status_id`를 반환하는 조회 결과.
#[derive(Debug)]
pub struct TaskRow {
    pub id: String,
    pub status_id: String,
}

/// id로 태스크를 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<TaskRow>> {
    let mut stmt = conn.prepare("SELECT id, status_id FROM tasks WHERE id = ?1")?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(TaskRow {
            id: row.get("id")?,
            status_id: row.get("status_id")?,
        })
    })?;
    rows.next().transpose()
}

/// 태스크의 `created_at` (RFC3339 문자열)을 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_created_at(conn: &Connection, task_id: &str) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT created_at FROM tasks WHERE id = ?1")?;
    let mut rows = stmt.query_map([task_id], |row| row.get::<_, String>(0))?;
    rows.next().transpose()
}

/// 태스크 ID에서 title과 project name을 조회한다.
///
/// # Errors
///
/// SELECT 실패 시 `rusqlite::Error`.
pub fn find_title_and_project(
    conn: &Connection,
    task_id: &str,
) -> rusqlite::Result<Option<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT t.title, p.name FROM tasks t JOIN projects p ON t.project_id = p.id WHERE t.id = ?1",
    )?;
    let mut rows = stmt.query_map([task_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.next().transpose()
}

/// 태스크의 `status_id`와 `updated_at`을 변경한다. 변경된 행이 있으면 true.
///
/// # Errors
///
/// UPDATE 실패 시 `rusqlite::Error`.
pub fn update_status(
    conn: &Connection,
    id: &str,
    status_id: &str,
    updated_at: &DateTime<Utc>,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE tasks SET status_id = ?1, updated_at = ?2 WHERE id = ?3",
        (status_id, updated_at.to_rfc3339(), id),
    )?;
    Ok(changed > 0)
}

/// 태스크 업데이트 파라미터.
pub struct TaskUpdate<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub label: Option<&'a str>,
}

/// 태스크의 지정된 필드를 업데이트한다. 변경된 행이 있으면 true.
///
/// `updated_at`은 항상 현재 시각으로 갱신된다.
///
/// # Errors
///
/// UPDATE 실패 시 `rusqlite::Error`.
pub fn update(
    conn: &Connection,
    id: &str,
    params: &TaskUpdate<'_>,
    updated_at: &DateTime<Utc>,
) -> rusqlite::Result<bool> {
    let mut sets = vec!["updated_at = ?"];
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(updated_at.to_rfc3339())];

    if let Some(title) = params.title {
        sets.push("title = ?");
        values.push(Box::new(title.to_string()));
    }
    if let Some(description) = params.description {
        sets.push("description = ?");
        values.push(Box::new(description.to_string()));
    }
    if let Some(label) = params.label {
        sets.push("label = ?");
        values.push(Box::new(label.to_string()));
    }

    let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
    values.push(Box::new(id.to_string()));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        values.iter().map(std::convert::AsRef::as_ref).collect();
    let changed = conn.execute(&sql, param_refs.as_slice())?;
    Ok(changed > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::{project_repo, status_repo};
    use crate::domain::project::{Project, ProjectPrefix};
    use crate::domain::status::StatusCategory;
    use crate::domain::task::{Label, Task};

    fn setup() -> (rusqlite::Connection, String, String) {
        let conn = initialize_in_memory().unwrap();
        let prefix = ProjectPrefix::new("SEO").unwrap();
        let project = Project::new("Seogi", &prefix, "goal", chrono::Utc::now()).unwrap();
        project_repo::save(&conn, &project).unwrap();
        let project_id = project.id().to_string();

        // backlog 상태 가져오기 (시딩된 상태 중 backlog 카테고리)
        let statuses = status_repo::list_all(&conn).unwrap();
        let backlog = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Backlog)
            .unwrap();
        let status_id = backlog.id().to_string();

        (conn, project_id, status_id)
    }

    fn sample_task(prefix_str: &str, seq: i64, status_id: &str, project_id: &str) -> Task {
        let prefix = ProjectPrefix::new(prefix_str).unwrap();
        Task::new(
            &prefix,
            seq,
            "test task",
            "description",
            Label::Feature,
            status_id,
            project_id,
            chrono::Utc::now(),
        )
        .unwrap()
    }

    // Q11: save 후 DB에서 조회 시 필드 일치
    #[test]
    fn test_task_repo_save_and_find() {
        let (conn, project_id, status_id) = setup();
        let task = sample_task("SEO", 1, &status_id, &project_id);
        save(&conn, &task).unwrap();

        let filter = TaskFilter {
            project_name: None,
            status_name: None,
            label: None,
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "SEO-1");
        assert_eq!(rows[0].title, "test task");
        assert_eq!(rows[0].label, "feature");
        assert_eq!(rows[0].status_name, "backlog");
        assert_eq!(rows[0].project_name, "Seogi");
    }

    // Q12: list_all 전체 반환
    #[test]
    fn test_task_repo_list_all() {
        let (conn, project_id, status_id) = setup();

        let task1 = sample_task("SEO", 1, &status_id, &project_id);
        let task2 = Task::new(
            &ProjectPrefix::new("SEO").unwrap(),
            2,
            "second",
            "desc2",
            Label::Bug,
            &status_id,
            &project_id,
            chrono::Utc::now(),
        )
        .unwrap();
        save(&conn, &task1).unwrap();
        save(&conn, &task2).unwrap();

        let filter = TaskFilter {
            project_name: None,
            status_name: None,
            label: None,
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 2);
    }

    // Q13: project 필터
    #[test]
    fn test_task_repo_list_filter_project() {
        let (conn, project_id, status_id) = setup();

        // 두 번째 프로젝트
        let prefix2 = ProjectPrefix::new("LOC").unwrap();
        let project2 = Project::new("Local", &prefix2, "goal2", chrono::Utc::now()).unwrap();
        project_repo::save(&conn, &project2).unwrap();

        let task1 = sample_task("SEO", 1, &status_id, &project_id);
        let task2 = sample_task("LOC", 1, &status_id, project2.id());
        save(&conn, &task1).unwrap();
        save(&conn, &task2).unwrap();

        let filter = TaskFilter {
            project_name: Some("Seogi"),
            status_name: None,
            label: None,
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "SEO-1");
    }

    // Q14: status 필터
    #[test]
    fn test_task_repo_list_filter_status() {
        let (conn, project_id, status_id) = setup();

        // todo 상태 찾기
        let statuses = status_repo::list_all(&conn).unwrap();
        let todo = statuses.iter().find(|s| s.name() == "todo").unwrap();

        let task1 = sample_task("SEO", 1, &status_id, &project_id);
        let task2 = Task::new(
            &ProjectPrefix::new("SEO").unwrap(),
            2,
            "todo task",
            "desc",
            Label::Feature,
            todo.id(),
            &project_id,
            chrono::Utc::now(),
        )
        .unwrap();
        save(&conn, &task1).unwrap();
        save(&conn, &task2).unwrap();

        let filter = TaskFilter {
            project_name: None,
            status_name: Some("backlog"),
            label: None,
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "SEO-1");
    }

    // Q15: label 필터
    #[test]
    fn test_task_repo_list_filter_label() {
        let (conn, project_id, status_id) = setup();

        let task1 = sample_task("SEO", 1, &status_id, &project_id);
        let task2 = Task::new(
            &ProjectPrefix::new("SEO").unwrap(),
            2,
            "bug task",
            "desc",
            Label::Bug,
            &status_id,
            &project_id,
            chrono::Utc::now(),
        )
        .unwrap();
        save(&conn, &task1).unwrap();
        save(&conn, &task2).unwrap();

        let filter = TaskFilter {
            project_name: None,
            status_name: None,
            label: Some("feature"),
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "SEO-1");
    }

    // Q16: 복합 필터
    #[test]
    fn test_task_repo_list_filter_combined() {
        let (conn, project_id, status_id) = setup();

        let prefix2 = ProjectPrefix::new("LOC").unwrap();
        let project2 = Project::new("Local", &prefix2, "goal2", chrono::Utc::now()).unwrap();
        project_repo::save(&conn, &project2).unwrap();

        let task1 = sample_task("SEO", 1, &status_id, &project_id);
        let task2 = Task::new(
            &ProjectPrefix::new("SEO").unwrap(),
            2,
            "seo bug",
            "desc",
            Label::Bug,
            &status_id,
            &project_id,
            chrono::Utc::now(),
        )
        .unwrap();
        let task3 = Task::new(
            &ProjectPrefix::new("LOC").unwrap(),
            1,
            "loc feature",
            "desc",
            Label::Feature,
            &status_id,
            project2.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        save(&conn, &task1).unwrap();
        save(&conn, &task2).unwrap();
        save(&conn, &task3).unwrap();

        let filter = TaskFilter {
            project_name: Some("Seogi"),
            status_name: Some("backlog"),
            label: Some("feature"),
        };
        let rows = list_all(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "SEO-1");
    }

    // Q3: update title만 → title 변경, updated_at 갱신
    #[test]
    fn test_task_repo_update_title_only() {
        let (conn, project_id, status_id) = setup();
        let task = sample_task("SEO", 1, &status_id, &project_id);
        save(&conn, &task).unwrap();

        let params = TaskUpdate {
            title: Some("new title"),
            description: None,
            label: None,
        };
        let changed = update(&conn, "SEO-1", &params, &chrono::Utc::now()).unwrap();
        assert!(changed);

        let (title, desc): (String, String) = conn
            .query_row(
                "SELECT title, description FROM tasks WHERE id = 'SEO-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(title, "new title");
        assert_eq!(desc, "description"); // 변경 안 됨
    }

    // Q4: update 복합 → 모든 필드 변경
    #[test]
    fn test_task_repo_update_combined() {
        let (conn, project_id, status_id) = setup();
        let task = sample_task("SEO", 1, &status_id, &project_id);
        save(&conn, &task).unwrap();

        let params = TaskUpdate {
            title: Some("t2"),
            description: Some("d2"),
            label: Some("bug"),
        };
        let changed = update(&conn, "SEO-1", &params, &chrono::Utc::now()).unwrap();
        assert!(changed);

        let (title, desc, label): (String, String, String) = conn
            .query_row(
                "SELECT title, description, label FROM tasks WHERE id = 'SEO-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(title, "t2");
        assert_eq!(desc, "d2");
        assert_eq!(label, "bug");
    }

    // Q5: update 없는 id → false
    #[test]
    fn test_task_repo_update_not_found() {
        let (conn, _, _) = setup();
        let params = TaskUpdate {
            title: Some("new"),
            description: None,
            label: None,
        };
        let changed = update(&conn, "SEO-99", &params, &chrono::Utc::now()).unwrap();
        assert!(!changed);
    }

    // Q8: find_by_id 존재 → Some
    #[test]
    fn test_task_repo_find_by_id_found() {
        let (conn, project_id, status_id) = setup();
        let task = sample_task("SEO", 1, &status_id, &project_id);
        save(&conn, &task).unwrap();

        let found = find_by_id(&conn, "SEO-1").unwrap();
        assert!(found.is_some());
        let row = found.unwrap();
        assert_eq!(row.id, "SEO-1");
        assert_eq!(row.status_id, status_id);
    }

    // Q9: find_by_id 미존재 → None
    #[test]
    fn test_task_repo_find_by_id_not_found() {
        let (conn, _, _) = setup();
        assert!(find_by_id(&conn, "SEO-99").unwrap().is_none());
    }

    // Q10: update_status 성공
    #[test]
    fn test_task_repo_update_status() {
        let (conn, project_id, status_id) = setup();
        let task = sample_task("SEO", 1, &status_id, &project_id);
        save(&conn, &task).unwrap();

        // todo 상태 찾기
        let statuses = status_repo::list_all(&conn).unwrap();
        let todo = statuses.iter().find(|s| s.name() == "todo").unwrap();

        let changed = update_status(&conn, "SEO-1", todo.id(), &chrono::Utc::now()).unwrap();
        assert!(changed);

        let row = find_by_id(&conn, "SEO-1").unwrap().unwrap();
        assert_eq!(row.status_id, todo.id());
    }
}
