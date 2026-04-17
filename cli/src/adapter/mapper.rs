use rusqlite::Row;

use crate::domain::log::ToolUse;

/// `tool_uses` 테이블의 한 행을 `ToolUse` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn tool_use_from_row(row: &Row<'_>) -> rusqlite::Result<ToolUse> {
    Ok(ToolUse::new(
        row.get("id")?,
        row.get("session_id")?,
        row.get("project")?,
        row.get("project_path")?,
        row.get("tool_name")?,
        row.get("tool_input")?,
        row.get("duration_ms")?,
        row.get("timestamp")?,
    ))
}
