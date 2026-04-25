use anyhow::Result;
use rusqlite::Connection;

use crate::workflow;

/// `seogi status create` 핸들러.
///
/// # Errors
///
/// 카테고리 검증, 빈 이름, DB 에러 시 `anyhow::Error`.
pub fn create(conn: &Connection, category: &str, name: &str) -> Result<()> {
    let status =
        workflow::status::create(conn, category, name).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Created status \"{}\" ({}, position {})",
        status.name(),
        status.category(),
        status.position()
    );
    Ok(())
}

/// `seogi status list` 핸들러.
///
/// # Errors
///
/// DB 에러, 직렬화 에러 시 `anyhow::Error`.
pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let statuses = workflow::status::list(conn).map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&statuses)
                .map_err(|e| anyhow::anyhow!("Failed to serialize: {e}"))?
        );
    } else {
        println!("{:<36} {:<16} {:<12} POS", "ID", "NAME", "CATEGORY");
        for s in &statuses {
            println!(
                "{:<36} {:<16} {:<12} {}",
                s.id(),
                s.name(),
                s.category(),
                s.position()
            );
        }
    }
    Ok(())
}

/// `seogi status update` 핸들러.
///
/// # Errors
///
/// 빈 이름, 존재하지 않는 id, DB 에러 시 `anyhow::Error`.
pub fn update(conn: &Connection, id: &str, name: &str) -> Result<()> {
    workflow::status::update(conn, id, name).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Updated status {id}");
    Ok(())
}

/// `seogi status delete` 핸들러.
///
/// # Errors
///
/// 존재하지 않는 id, tasks 참조 중, DB 에러 시 `anyhow::Error`.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    workflow::status::delete(conn, id).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Deleted status {id}");
    Ok(())
}
