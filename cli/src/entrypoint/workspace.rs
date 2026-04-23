use anyhow::Result;
use rusqlite::Connection;

use crate::workflow;

/// `seogi workspace create` 핸들러.
///
/// # Errors
///
/// prefix 검증, 중복 prefix, DB 에러 시 `anyhow::Error`.
pub fn create(conn: &Connection, name: &str, prefix: Option<&str>, goal: &str) -> Result<()> {
    let workspace = workflow::workspace::create(conn, name, prefix, goal)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Created workspace \"{}\" ({})",
        workspace.name(),
        workspace.prefix()
    );
    Ok(())
}

/// `seogi workspace list` 핸들러.
///
/// # Errors
///
/// DB 에러, 직렬화 에러 시 `anyhow::Error`.
pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let workspaces = workflow::workspace::list(conn).map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&workspaces)
                .map_err(|e| anyhow::anyhow!("Failed to serialize: {e}"))?
        );
    } else {
        println!("{:<8} {:<20} GOAL", "PREFIX", "NAME");
        for p in &workspaces {
            println!("{:<8} {:<20} {}", p.prefix(), p.name(), p.goal());
        }
    }
    Ok(())
}
