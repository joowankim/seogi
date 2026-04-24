use anyhow::Result;
use rusqlite::Connection;

use crate::workflow;

/// `seogi cycle create` 핸들러.
///
/// # Errors
///
/// 워크스페이스 미존재, 검증 실패, DB 에러 시 `anyhow::Error`.
pub fn create(
    conn: &Connection,
    workspace: &str,
    name: &str,
    start: &str,
    end: &str,
) -> Result<()> {
    let cycle = workflow::cycle::create(conn, workspace, name, start, end)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Created cycle {} \"{}\"", cycle.id(), cycle.name());
    Ok(())
}

/// `seogi cycle list` 핸들러.
///
/// # Errors
///
/// DB 에러, 직렬화 에러 시 `anyhow::Error`.
pub fn list(conn: &Connection, workspace: Option<&str>, json: bool) -> Result<()> {
    let rows = workflow::cycle::list(conn, workspace).map_err(|e| anyhow::anyhow!("{e}"))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| anyhow::anyhow!("Failed to serialize: {e}"))?
        );
    } else {
        let header = format!(
            "{:<12} {:<16} {:<12} {:<14} {:<14} WORKSPACE",
            "ID", "NAME", "STATUS", "START", "END"
        );
        println!("{header}");
        for r in &rows {
            let short_id = if r.id.len() > 10 { &r.id[..10] } else { &r.id };
            println!(
                "{:<12} {:<16} {:<12} {:<14} {:<14} {}",
                short_id, r.name, r.status, r.start_date, r.end_date, r.workspace_name
            );
        }
    }
    Ok(())
}

/// `seogi cycle update` 핸들러.
///
/// # Errors
///
/// cycle 미존재, 검증 실패, DB 에러 시 `anyhow::Error`.
pub fn update(
    conn: &Connection,
    cycle_id: &str,
    name: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<()> {
    workflow::cycle::update(conn, cycle_id, name, start, end)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Updated cycle {cycle_id}");
    Ok(())
}
