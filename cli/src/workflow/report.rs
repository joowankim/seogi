use std::collections::HashSet;

use rusqlite::Connection;

use crate::adapter::{git, log_repo, status_repo, task_event_repo, task_repo, transcript};
use crate::domain::metrics;
use crate::domain::report::{self, TaskReport};
use crate::domain::status::StatusCategory;
use crate::domain::task_metrics;
use crate::domain::token_usage::TokenUsage;
use crate::domain::value::Timestamp;

/// 날짜 문자열(YYYY-MM-DD)을 파싱하여 밀리초 UTC timestamp를 반환한다.
///
/// `is_end`가 true이면 해당 날짜의 23:59:59.999 UTC를 반환한다.
fn parse_date_to_ms(date: &str, is_end: bool) -> Result<i64, anyhow::Error> {
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date format '{date}': {e}"))?;

    let dt = if is_end {
        parsed
            .and_hms_milli_opt(23, 59, 59, 999)
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {date}"))?
    } else {
        parsed
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {date}"))?
    };

    let utc = dt.and_utc();
    Ok(utc.timestamp_millis())
}

/// 태스크 기반 성과 리포트를 생성한다.
///
/// # Errors
///
/// DB 조회, 날짜 파싱, 또는 어댑터 오류 시 에러 반환.
pub fn run(
    conn: &Connection,
    from: &str,
    to: &str,
    project: Option<&str>,
    detail: bool,
) -> Result<String, anyhow::Error> {
    let from_ms = parse_date_to_ms(from, false)?;
    let to_ms = parse_date_to_ms(to, true)?;

    if from_ms > to_ms {
        return Err(anyhow::anyhow!(
            "Invalid date range: 'from' ({from}) is after 'to' ({to})"
        ));
    }

    // StatusMap 구성
    let statuses = status_repo::list_all(conn)?;
    let status_map: Vec<(String, StatusCategory)> = statuses
        .iter()
        .map(|s| (s.name().to_string(), s.category()))
        .collect();

    // 기간 내 completed 이벤트 조회
    let completed_events = task_event_repo::list_completed_in_range(conn, from_ms, to_ms)?;

    // 고유 task_id 추출
    let mut task_ids: Vec<String> = completed_events
        .iter()
        .map(|e| e.task_id().to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    task_ids.sort();

    if task_ids.is_empty() {
        return Ok("No completed tasks in the given period.\n".to_string());
    }

    // project 필터 적용 및 title/project_name 수집
    let mut filtered_tasks: Vec<(String, String)> = Vec::new(); // (task_id, title)
    for task_id in &task_ids {
        if let Some((title, workspace_name)) = task_repo::find_title_and_workspace(conn, task_id)? {
            if let Some(filter_project) = project
                && workspace_name != filter_project
            {
                continue;
            }
            filtered_tasks.push((task_id.clone(), title));
        }
    }

    if filtered_tasks.is_empty() {
        return Ok("No completed tasks in the given period.\n".to_string());
    }

    // 각 태스크에 대해 리포트 구성
    let cwd = std::env::current_dir()?;
    let mut reports = Vec::new();
    let mut all_event_groups: Vec<Vec<crate::domain::task::TaskEvent>> = Vec::new();

    for (task_id, title) in &filtered_tasks {
        let events = task_event_repo::list_by_task_id(conn, task_id)?;

        // created_at 조회
        let created_at_ms = task_repo::find_created_at(conn, task_id)?
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map_or(Timestamp::new(0), |dt| {
                Timestamp::new(dt.timestamp_millis())
            });

        let cycle = task_metrics::cycle_time(&events, &status_map);
        let lead = task_metrics::lead_time(&events, created_at_ms, &status_map);
        let flow_eff = task_metrics::flow_efficiency(cycle, lead);

        // proxy metrics: Started~Completed 범위
        let proxy = compute_proxy(conn, &events, &status_map)?;

        // token usage: tool_uses에서 고유 session_id+project_path 추출
        let tokens = compute_tokens(conn, &events, &status_map)?;

        // git diff stat
        let task_size = git::diff_stat(&cwd, task_id).unwrap_or(None);

        // rework 여부
        let has_rework = task_metrics::has_rework(&events, &status_map);

        reports.push(TaskReport {
            id: task_id.clone(),
            title: title.clone(),
            cycle_time: cycle,
            lead_time: lead,
            flow_efficiency: flow_eff,
            tokens,
            task_size,
            has_rework,
            proxy,
        });

        all_event_groups.push(events);
    }

    // aggregate metrics
    let tp = task_metrics::throughput(&completed_events);
    let group_refs: Vec<&[crate::domain::task::TaskEvent]> =
        all_event_groups.iter().map(Vec::as_slice).collect();
    let ftd_rate = task_metrics::first_time_done_rate(&group_refs, &status_map);

    let avg_flow_eff = {
        let effs: Vec<f64> = reports.iter().filter_map(|r| r.flow_efficiency).collect();
        if effs.is_empty() {
            None
        } else {
            Some(effs.iter().sum::<f64>() / effs.len() as f64)
        }
    };

    let output = if detail {
        report::format_detail(&reports)
    } else {
        report::format_summary(&reports, tp, avg_flow_eff, ftd_rate)
    };

    Ok(format!("{output}\n"))
}

/// Started~Completed 시간 범위(밀리초)를 반환한다.
fn started_completed_range(
    events: &[crate::domain::task::TaskEvent],
    status_map: &[(String, StatusCategory)],
) -> Option<(i64, i64)> {
    let start = task_metrics::first_transition_to(events, StatusCategory::Started, status_map)?;
    let end = task_metrics::first_transition_to(events, StatusCategory::Completed, status_map)?;
    Some((start.value(), end.value()))
}

/// Started~Completed 시간 범위에서 프록시 지표를 계산한다.
fn compute_proxy(
    conn: &Connection,
    events: &[crate::domain::task::TaskEvent],
    status_map: &[(String, StatusCategory)],
) -> Result<Option<crate::domain::metrics::TaskProxyMetrics>, anyhow::Error> {
    let Some((start, end)) = started_completed_range(events, status_map) else {
        return Ok(None);
    };

    let tool_uses = log_repo::list_by_time_range(conn, start, end)?;
    let tool_failures = log_repo::list_failures_by_time_range(conn, start, end)?;
    Ok(Some(metrics::calculate(&tool_uses, &tool_failures)))
}

/// Started~Completed 범위의 `tool_uses`에서 고유 `session_id`+`project_path`를 추출하고
/// transcript에서 token usage를 합산한다.
fn compute_tokens(
    conn: &Connection,
    events: &[crate::domain::task::TaskEvent],
    status_map: &[(String, StatusCategory)],
) -> Result<Option<TokenUsage>, anyhow::Error> {
    let Some((start, end)) = started_completed_range(events, status_map) else {
        return Ok(None);
    };

    let tool_uses = log_repo::list_by_time_range(conn, start, end)?;
    if tool_uses.is_empty() {
        return Ok(None);
    }

    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut total = TokenUsage::zero();

    for tu in &tool_uses {
        let key = (
            tu.session_id().as_str().to_string(),
            tu.workspace_path().to_string(),
        );
        if seen.insert(key) {
            let usage =
                transcript::read_token_usage(tu.workspace_path(), tu.session_id().as_str())?;
            total = total + usage;
        }
    }

    Ok(Some(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::{log_repo, status_repo, task_event_repo, task_repo, workspace_repo};
    use crate::domain::log::ToolUse;
    use crate::domain::status::StatusCategory;
    use crate::domain::task::{Label, Task, TaskEvent};
    use crate::domain::value::{Ms, SessionId, Timestamp};
    use crate::domain::workspace::{Workspace, WorkspacePrefix};

    fn setup_with_completed_task(conn: &Connection) {
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let workspace = Workspace::new("Seogi", &prefix, "goal", chrono::Utc::now()).unwrap();
        workspace_repo::save(conn, &workspace).unwrap();

        let statuses = status_repo::list_all(conn).unwrap();
        let backlog = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Backlog)
            .unwrap();

        let task = Task::new(
            &prefix,
            1,
            "test task",
            "description",
            Label::Feature,
            backlog.id(),
            workspace.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        task_repo::save(conn, &task).unwrap();

        // backlog -> in_progress -> done
        let e1 = TaskEvent::new("SEO-1", None, "backlog", "CLI", Timestamp::new(1_000_000));
        let e2 = TaskEvent::new(
            "SEO-1",
            Some("backlog"),
            "in_progress",
            "CLI",
            Timestamp::new(2_000_000),
        );
        let e3 = TaskEvent::new(
            "SEO-1",
            Some("in_progress"),
            "done",
            "CLI",
            Timestamp::new(5_000_000),
        );
        task_event_repo::save(conn, &e1).unwrap();
        task_event_repo::save(conn, &e2).unwrap();
        task_event_repo::save(conn, &e3).unwrap();
    }

    // --- parse_date_to_ms ---

    #[test]
    fn parse_date_start_of_day() {
        let ms = parse_date_to_ms("2026-01-15", false).unwrap();
        // 2026-01-15 00:00:00 UTC
        assert!(ms > 0);
        assert_eq!(ms % 1000, 0); // exact seconds
    }

    #[test]
    fn parse_date_end_of_day() {
        let ms = parse_date_to_ms("2026-01-15", true).unwrap();
        // Should be 23:59:59.999
        assert_eq!(ms % 1000, 999);
    }

    #[test]
    fn parse_date_invalid_format() {
        assert!(parse_date_to_ms("not-a-date", false).is_err());
        assert!(parse_date_to_ms("2026/01/15", false).is_err());
    }

    // --- run ---

    #[test]
    fn run_from_after_to_returns_error() {
        let conn = initialize_in_memory().unwrap();
        let result = run(&conn, "2026-02-01", "2026-01-01", None, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid date range"));
    }

    #[test]
    fn run_empty_period_returns_no_tasks_message() {
        let conn = initialize_in_memory().unwrap();
        let result = run(&conn, "2020-01-01", "2020-01-31", None, false).unwrap();
        assert!(result.contains("No completed tasks"));
    }

    #[test]
    fn run_with_completed_task_returns_summary() {
        let conn = initialize_in_memory().unwrap();
        setup_with_completed_task(&conn);

        // The events are at timestamp 1_000_000 ~ 5_000_000 ms
        // which is 1970-01-01. Use wide date range.
        let result = run(&conn, "1970-01-01", "1970-12-31", None, false).unwrap();
        assert!(result.contains("SEO-1"));
        assert!(result.contains("throughput"));
    }

    #[test]
    fn run_with_detail_flag() {
        let conn = initialize_in_memory().unwrap();
        setup_with_completed_task(&conn);

        let result = run(&conn, "1970-01-01", "1970-12-31", None, true).unwrap();
        assert!(result.contains("=== SEO-1:"));
        assert!(result.contains("cycle_time:"));
    }

    #[test]
    fn run_with_project_filter_excludes_other_projects() {
        let conn = initialize_in_memory().unwrap();
        setup_with_completed_task(&conn);

        let result = run(
            &conn,
            "1970-01-01",
            "1970-12-31",
            Some("OtherProject"),
            false,
        )
        .unwrap();
        assert!(result.contains("No completed tasks"));
    }

    #[test]
    fn run_with_matching_project_filter() {
        let conn = initialize_in_memory().unwrap();
        setup_with_completed_task(&conn);

        let result = run(&conn, "1970-01-01", "1970-12-31", Some("Seogi"), false).unwrap();
        assert!(result.contains("SEO-1"));
    }

    fn setup_with_tool_uses(conn: &Connection) {
        setup_with_completed_task(conn);

        let tu = ToolUse::new(
            "tu-1".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/nonexistent/project".to_string(),
            "Read".to_string(),
            "{}".to_string(),
            Ms::zero(),
            Timestamp::new(3_000_000),
        );
        log_repo::save_tool_use(conn, &tu).unwrap();
    }

    #[test]
    fn run_with_tool_uses_computes_proxy_and_tokens() {
        let conn = initialize_in_memory().unwrap();
        setup_with_tool_uses(&conn);

        let result = run(&conn, "1970-01-01", "1970-12-31", None, true).unwrap();
        // proxy metrics가 계산되어 detail 출력에 포함
        assert!(result.contains("test_invoked:"));
        assert!(result.contains("doom_loop:"));
        // tokens: transcript 파일이 없으므로 zero → "—"이 아닌 "0" 출력
        assert!(result.contains("tokens:"));
    }

    #[test]
    fn compute_proxy_returns_none_without_started() {
        let conn = initialize_in_memory().unwrap();
        let statuses = status_repo::list_all(&conn).unwrap();
        let status_map: Vec<(String, StatusCategory)> = statuses
            .iter()
            .map(|s| (s.name().to_string(), s.category()))
            .collect();

        // backlog만 있는 이벤트 (Started 없음)
        let events = vec![TaskEvent::new(
            "X-1",
            None,
            "backlog",
            "CLI",
            Timestamp::new(1000),
        )];
        let result = compute_proxy(&conn, &events, &status_map).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn run_with_duplicate_session_tool_uses() {
        let conn = initialize_in_memory().unwrap();
        setup_with_tool_uses(&conn);

        let tu2 = ToolUse::new(
            "tu-2".to_string(),
            SessionId::new("sess-1"),
            "seogi".to_string(),
            "/nonexistent/project".to_string(),
            "Edit".to_string(),
            r#"{"file_path":"a.rs"}"#.to_string(),
            Ms::zero(),
            Timestamp::new(4_000_000),
        );
        log_repo::save_tool_use(&conn, &tu2).unwrap();

        let result = run(&conn, "1970-01-01", "1970-12-31", None, true).unwrap();
        assert!(result.contains("tokens:"));
    }

    #[test]
    fn run_with_no_flow_efficiency_returns_dash() {
        // Completed 이벤트만 있고 Started가 없는 태스크 → cycle_time=None → flow_efficiency=None
        let conn = initialize_in_memory().unwrap();
        let prefix = WorkspacePrefix::new("TST").unwrap();
        let workspace = Workspace::new("Test", &prefix, "goal", chrono::Utc::now()).unwrap();
        workspace_repo::save(&conn, &workspace).unwrap();

        let statuses = status_repo::list_all(&conn).unwrap();
        let backlog = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Backlog)
            .unwrap();

        let task = Task::new(
            &prefix,
            1,
            "no cycle",
            "desc",
            Label::Feature,
            backlog.id(),
            workspace.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        task_repo::save(&conn, &task).unwrap();

        // backlog → done 직접 전환 (Started 없음)
        let e1 = TaskEvent::new("TST-1", None, "backlog", "CLI", Timestamp::new(1_000_000));
        let e2 = TaskEvent::new(
            "TST-1",
            Some("backlog"),
            "done",
            "CLI",
            Timestamp::new(5_000_000),
        );
        task_event_repo::save(&conn, &e1).unwrap();
        task_event_repo::save(&conn, &e2).unwrap();

        let result = run(&conn, "1970-01-01", "1970-12-31", None, false).unwrap();
        assert!(result.contains("TST-1"));
        // flow_efficiency(avg)가 "—"
        assert!(result.contains("\u{2014}"));
    }

    #[test]
    fn compute_tokens_returns_none_without_started() {
        let conn = initialize_in_memory().unwrap();
        let statuses = status_repo::list_all(&conn).unwrap();
        let status_map: Vec<(String, StatusCategory)> = statuses
            .iter()
            .map(|s| (s.name().to_string(), s.category()))
            .collect();

        let events = vec![TaskEvent::new(
            "X-1",
            None,
            "backlog",
            "CLI",
            Timestamp::new(1000),
        )];
        let result = compute_tokens(&conn, &events, &status_map).unwrap();
        assert!(result.is_none());
    }
}
