use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::adapter::db;
use crate::workflow::log_tool;

fn db_path() -> PathBuf {
    std::env::var("SEOGI_DB_PATH").map_or_else(
        |_| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".seogi").join("seogi.db")
        },
        PathBuf::from,
    )
}

/// `PostToolUse` 훅 진입점.
///
/// stdin에서 JSON을 읽고, DB에 도구 사용 기록을 저장한다.
///
/// # Errors
///
/// stdin 읽기, JSON 파싱, DB 초기화/쓰기 실패 시 에러 반환.
pub fn run() -> Result<()> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .context("Failed to read stdin")?;

    let conn = db::initialize_db(&db_path()).context("Failed to initialize database")?;

    log_tool::run(&conn, &stdin_buf).context("Failed to save tool use")?;

    Ok(())
}
