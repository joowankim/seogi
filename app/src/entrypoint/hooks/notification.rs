use std::io::Read;

use anyhow::{Context, Result};

use crate::adapter::db;
use crate::workflow::log_system;

/// `Notification` 훅 진입점.
///
/// stdin에서 JSON을 읽고, DB에 알림 이벤트를 저장한다.
///
/// # Errors
///
/// stdin 읽기, JSON 파싱, DB 초기화/쓰기 실패 시 에러 반환.
pub fn run() -> Result<()> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .context("Failed to read stdin")?;

    let conn = db::initialize_db(&super::db_path()).context("Failed to initialize database")?;

    log_system::run_notification(&conn, &stdin_buf).context("Failed to save notification")?;

    Ok(())
}
