use rusqlite::Connection;

use crate::adapter::error::AdapterError;
use crate::adapter::log_repo;
use crate::domain::metrics;
use crate::domain::report;
use crate::domain::value::SessionId;

/// 기간별 리포트 워크플로우.
///
/// Impureim Sandwich: DB 조회(불순) → 지표 계산(순수) → 집계/포맷(순수).
///
/// # Errors
///
/// DB 읽기 실패 시 `AdapterError::Database`.
/// 날짜 파싱 실패 시 `AdapterError::Json`.
pub fn run(
    conn: &Connection,
    from: &str,
    to: &str,
    project: Option<&str>,
) -> Result<String, AdapterError> {
    let from_ts = parse_date_to_millis(from)?;
    let to_ts = parse_date_to_millis_end(to)?;

    // [Top: Impure] DB에서 세션 목록 조회
    let session_ids = log_repo::list_session_ids_by_range(conn, from_ts, to_ts, project)?;

    // [Middle: Pure] 각 세션의 메트릭 계산
    let mut all_metrics = Vec::new();
    for sid in &session_ids {
        let tool_uses = log_repo::list_by_session(conn, sid)?;
        let tool_failures = log_repo::list_failures_by_session(conn, sid)?;
        all_metrics.push(metrics::calculate(
            SessionId::new(sid),
            &tool_uses,
            &tool_failures,
        ));
    }

    // [Bottom: Pure] 집계 + 포맷
    let output = report::format_report(&all_metrics, from, to, project);

    Ok(output)
}

fn parse_date_to_millis(date: &str) -> Result<i64, AdapterError> {
    let naive = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| AdapterError::DateParse(format!("invalid date '{date}': {e}")))?;
    let dt = naive.and_hms_opt(0, 0, 0).unwrap().and_utc();
    Ok(dt.timestamp_millis())
}

fn parse_date_to_millis_end(date: &str) -> Result<i64, AdapterError> {
    let naive = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| AdapterError::DateParse(format!("invalid date '{date}': {e}")))?;
    let dt = naive.and_hms_opt(23, 59, 59).unwrap().and_utc();
    Ok(dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db;
    use crate::domain::log::ToolUse;
    use crate::domain::value::{Ms, SessionId, Timestamp};

    fn make_tool_use(session_id: &str, ts: i64) -> ToolUse {
        ToolUse::new(
            format!("id-{session_id}-{ts}"),
            SessionId::new(session_id),
            "proj".to_string(),
            "/proj".to_string(),
            "Read".to_string(),
            "{}".to_string(),
            Ms::zero(),
            Timestamp::new(ts),
        )
    }

    #[test]
    fn test_workflow_report_run() {
        let conn = db::initialize_in_memory().unwrap();

        // 2026-04-07 = 1775692800000 ms (approx)
        let ts1 = 1_775_692_800_000;
        let ts2 = ts1 + 60_000;
        let ts3 = ts1 + 120_000;

        log_repo::save_tool_use(&conn, &make_tool_use("s1", ts1)).unwrap();
        log_repo::save_tool_use(&conn, &make_tool_use("s1", ts2)).unwrap();
        log_repo::save_tool_use(&conn, &make_tool_use("s2", ts3)).unwrap();

        let output = run(&conn, "2026-04-01", "2026-04-30", None).unwrap();
        assert!(output.contains("n=2"), "should show 2 sessions: {output}");
    }
}
