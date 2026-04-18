use anyhow::Result;
use rusqlite::Connection;

use crate::workflow;

/// `seogi task create` 핸들러.
///
/// # Errors
///
/// 프로젝트 미존재, 무효 라벨, 빈 제목/설명, DB 에러 시 `anyhow::Error`.
pub fn create(
    conn: &Connection,
    project: &str,
    title: &str,
    description: &str,
    label: &str,
) -> Result<()> {
    let task = workflow::task::create(conn, project, title, description, label)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Created task {} \"{}\"", task.id(), task.title());
    Ok(())
}

/// `seogi task list` 핸들러.
///
/// # Errors
///
/// DB 에러, 직렬화 에러 시 `anyhow::Error`.
pub fn list(
    conn: &Connection,
    project: Option<&str>,
    status: Option<&str>,
    label: Option<&str>,
    json: bool,
) -> Result<()> {
    let tasks =
        workflow::task::list(conn, project, status, label).map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&tasks)
                .map_err(|e| anyhow::anyhow!("Failed to serialize: {e}"))?
        );
    } else {
        println!("{:<10} {:<24} {:<16} LABEL", "ID", "TITLE", "STATUS");
        for t in &tasks {
            println!(
                "{:<10} {:<24} {:<16} {}",
                t.id, t.title, t.status_name, t.label
            );
        }
    }
    Ok(())
}

/// `seogi task update` 핸들러.
///
/// # Errors
///
/// 태스크 미존재, 옵션 미지정, 빈 제목/설명, 무효 라벨, DB 에러 시 `anyhow::Error`.
pub fn update(
    conn: &Connection,
    task_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    label: Option<&str>,
) -> Result<()> {
    workflow::task::update(conn, task_id, title, description, label)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Updated task {task_id}");
    Ok(())
}
