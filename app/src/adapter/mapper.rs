use std::str::FromStr;

use chrono::DateTime;
use rusqlite::Row;

use crate::domain::cycle::Cycle;
use crate::domain::log::{SystemEvent, ToolFailure, ToolUse};
use crate::domain::status::{Status, StatusCategory};
use crate::domain::task::TaskEvent;
use crate::domain::value::{Ms, SessionId, Timestamp};
use crate::domain::workspace::{Workspace, WorkspacePrefix};

/// `tool_uses` 테이블의 한 행을 `ToolUse` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn tool_use_from_row(row: &Row<'_>) -> rusqlite::Result<ToolUse> {
    Ok(ToolUse::new(
        row.get("id")?,
        SessionId::new(row.get::<_, String>("session_id")?),
        row.get("workspace")?,
        row.get("workspace_path")?,
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
        row.get("workspace")?,
        row.get("workspace_path")?,
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

/// `workspaces` 테이블의 한 행을 `Workspace` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn workspace_from_row(row: &Row<'_>) -> rusqlite::Result<Workspace> {
    let prefix_str: String = row.get("prefix")?;
    let prefix = WorkspacePrefix::new(&prefix_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Workspace::from_row(
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
        row.get("workspace")?,
        row.get("workspace_path")?,
        row.get("event_type")?,
        row.get("content")?,
        Timestamp::new(row.get("timestamp")?),
    ))
}

/// tasks JOIN statuses JOIN workspaces 쿼리 결과를 `TaskListRow`로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn task_list_row_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<crate::adapter::task_repo::TaskListRow> {
    Ok(crate::adapter::task_repo::TaskListRow {
        id: row.get("id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        label: row.get("label")?,
        status_name: row.get("status_name")?,
        workspace_name: row.get("workspace_name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// `task_events` 테이블의 한 행을 `TaskEvent` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn task_event_from_row(row: &Row<'_>) -> rusqlite::Result<TaskEvent> {
    Ok(TaskEvent::from_row(
        row.get("id")?,
        row.get("task_id")?,
        row.get("from_status")?,
        row.get("to_status")?,
        row.get("session_id")?,
        Timestamp::new(row.get("timestamp")?),
    ))
}

/// `cycles` 테이블의 한 행을 `Cycle` 도메인 타입으로 변환한다.
///
/// # Errors
///
/// 컬럼 읽기 실패 시 `rusqlite::Error`.
pub fn cycle_from_row(row: &Row<'_>) -> rusqlite::Result<Cycle> {
    Ok(Cycle::from_row(
        row.get("id")?,
        row.get("workspace_id")?,
        row.get("name")?,
        row.get("start_date")?,
        row.get("end_date")?,
        parse_datetime(row, "created_at")?,
        parse_datetime(row, "updated_at")?,
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
