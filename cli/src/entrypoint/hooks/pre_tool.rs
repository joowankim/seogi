use std::io::Read;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::adapter::timing;

#[derive(Debug, Deserialize)]
struct PreToolInput {
    tool_use_id: String,
}

/// `PreToolUse` 훅 진입점.
///
/// stdin에서 JSON을 읽고, 시작 시간을 파일에 기록한다.
///
/// # Errors
///
/// stdin 읽기, JSON 파싱, 파일 쓰기 실패 시 에러 반환.
pub fn run() -> Result<()> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .context("Failed to read stdin")?;

    let input: PreToolInput =
        serde_json::from_str(&stdin_buf).context("Failed to parse stdin JSON")?;

    let dir = timing::timing_dir();
    timing::save_start_time(&dir, &input.tool_use_id).context("Failed to save start time")?;

    Ok(())
}
