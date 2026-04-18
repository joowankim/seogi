use std::str::FromStr;

use chrono::DateTime;
use rusqlite::Row;

use crate::domain::log::{SystemEvent, ToolFailure, ToolUse};
use crate::domain::project::{Project, ProjectPrefix};
use crate::domain::status::{Status, StatusCategory};
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

fn parse_datetime(row: &Row<'_>, column: &str) -> rusqlite::Result<chrono::DateTime<chrono::Utc>> {
    let s: String = row.get(column)?;
    s.parse::<DateTime<chrono::FixedOffset>>()
        .map(|dt| dt.to_utc())
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

/// `projects` 테이블의 한 행을 `Project` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    let prefix_str: String = row.get("prefix")?;
    let prefix = ProjectPrefix::new(&prefix_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Project::from_row(
        row.get("id")?,
        row.get("name")?,
        prefix,
        row.get("goal")?,
        row.get("next_seq")?,
        parse_datetime(row, "created_at")?,
        parse_datetime(row, "updated_at")?,
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

/// `statuses` 테이블의 한 행을 `Status` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 또는 category 파싱 실패 시 `rusqlite::Error`.
pub fn status_from_row(row: &Row<'_>) -> rusqlite::Result<Status> {
    let category_str: String = row.get("category")?;
    let category = StatusCategory::from_str(&category_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Status::from_row(
        row.get("id")?,
        row.get("name")?,
        category,
        row.get("position")?,
    ))
}
