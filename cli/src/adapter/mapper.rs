use rusqlite::Row;

use crate::domain::log::{SystemEvent, ToolFailure, ToolUse};
use crate::domain::value::{Ms, SessionId, Timestamp};

/// `tool_uses` 테이블의 한 행을 `ToolUse` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn tool_use_from_row(row: &Row<'_>) -> rusqlite::Result<ToolUse> {
    Ok(ToolUse::new(
        row.get("id")?,
        SessionId::new(row.get::<_, String>("session_id")?),
        row.get("project")?,
        row.get("project_path")?,
        row.get("tool_name")?,
        row.get("tool_input")?,
        Ms::new(row.get("duration_ms")?),
        Timestamp::new(row.get("timestamp")?),
    ))
}

/// `tool_failures` 테이블의 한 행을 `ToolFailure` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn tool_failure_from_row(row: &Row<'_>) -> rusqlite::Result<ToolFailure> {
    Ok(ToolFailure::new(
        row.get("id")?,
        SessionId::new(row.get::<_, String>("session_id")?),
        row.get("project")?,
        row.get("project_path")?,
        row.get("tool_name")?,
        row.get("error")?,
        Timestamp::new(row.get("timestamp")?),
    ))
}

/// `system_events` 테이블의 한 행을 `SystemEvent` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn system_event_from_row(row: &Row<'_>) -> rusqlite::Result<SystemEvent> {
    Ok(SystemEvent::new(
        row.get("id")?,
        SessionId::new(row.get::<_, String>("session_id")?),
        row.get("project")?,
        row.get("project_path")?,
        row.get("event_type")?,
        row.get("content")?,
        Timestamp::new(row.get("timestamp")?),
    ))
}
