use rusqlite::Connection;

use crate::adapter::{cycle_repo, cycle_task_repo, git, status_repo, task_event_repo, task_repo};
use crate::domain::cycle_report::{self, CycleReportCategory, IncompleteTask};
use crate::domain::report::TaskReport;
use crate::domain::status::StatusCategory;
use crate::domain::task_metrics;
use crate::domain::value::Timestamp;
use crate::workflow::report;

/// Cycle report를 생성한다.
///
/// # Errors
///
/// - Cycle 미존재 → `anyhow::Error`
/// - DB 에러, 어댑터 오류 시 에러 반환
pub fn run(conn: &Connection, cycle_id: &str) -> Result<String, anyhow::Error> {
    let cycle = cycle_repo::find_by_id(conn, cycle_id)?
        .ok_or_else(|| anyhow::anyhow!("Cycle not found: {cycle_id}"))?;

    let assigned_tasks = cycle_task_repo::list_by_cycle(conn, cycle_id)?;
    if assigned_tasks.is_empty() {
        return Ok("No tasks assigned to this cycle.\n".to_string());
    }

    let statuses = status_repo::list_all(conn)?;
    let status_map: Vec<(String, StatusCategory)> = statuses
        .iter()
        .map(|s| (s.name().to_string(), s.category()))
        .collect();

    let mut planned_done_reports = Vec::new();
    let mut planned_incomplete_tasks = Vec::new();
    let mut unplanned_done_reports = Vec::new();

    let cwd = std::env::current_dir()?;

    for (task_id, assigned) in &assigned_tasks {
        let Some(task_detail) = task_repo::find_by_id_detailed(conn, task_id)? else {
            continue;
        };

        let status_category = statuses
            .iter()
            .find(|s| s.name() == task_detail.status_name)
            .map_or(
                StatusCategory::Backlog,
                crate::domain::status::Status::category,
            );

        let Some(category) = cycle_report::classify(*assigned, status_category) else {
            continue;
        };

        match category {
            CycleReportCategory::PlannedDone | CycleReportCategory::UnplannedDone => {
                let task_report =
                    build_task_report(conn, task_id, &task_detail.title, &status_map, &cwd)?;

                if category == CycleReportCategory::PlannedDone {
                    planned_done_reports.push(task_report);
                } else {
                    unplanned_done_reports.push(task_report);
                }
            }
            CycleReportCategory::PlannedIncomplete => {
                let events = task_event_repo::list_by_task_id(conn, task_id)?;
                let created_at_ms = task_repo::find_created_at(conn, task_id)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map_or(Timestamp::new(0), |dt| {
                        Timestamp::new(dt.timestamp_millis())
                    });
                let now_ms = Timestamp::new(chrono::Utc::now().timestamp_millis());
                let age = task_metrics::issue_age(&events, created_at_ms, now_ms, &status_map);

                planned_incomplete_tasks.push(IncompleteTask {
                    id: task_id.clone(),
                    title: task_detail.title.clone(),
                    status_name: task_detail.status_name.clone(),
                    issue_age: age,
                });
            }
        }
    }

    let mut all_done_reports = Vec::new();
    all_done_reports.extend(planned_done_reports.iter().map(clone_report));
    all_done_reports.extend(unplanned_done_reports.iter().map(clone_report));

    let summary = cycle_report::compute_summary(
        planned_done_reports.len(),
        planned_incomplete_tasks.len(),
        &all_done_reports,
    );

    let today = chrono::Utc::now().date_naive();
    let status = cycle.status(today);

    let output = cycle_report::format_cycle_report(&cycle_report::FormatCycleReportInput {
        cycle_name: cycle.name(),
        start_date: cycle.start_date(),
        end_date: cycle.end_date(),
        status: status.as_str(),
        planned_done: &planned_done_reports,
        planned_incomplete: &planned_incomplete_tasks,
        unplanned_done: &unplanned_done_reports,
        summary: &summary,
    });

    Ok(format!("{output}\n"))
}

fn build_task_report(
    conn: &Connection,
    task_id: &str,
    title: &str,
    status_map: &[(String, StatusCategory)],
    cwd: &std::path::Path,
) -> Result<TaskReport, anyhow::Error> {
    let events = task_event_repo::list_by_task_id(conn, task_id)?;

    let created_at_ms = task_repo::find_created_at(conn, task_id)?
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map_or(Timestamp::new(0), |dt| {
            Timestamp::new(dt.timestamp_millis())
        });

    let cycle_time = task_metrics::cycle_time(&events, status_map);
    let lead_time = task_metrics::lead_time(&events, created_at_ms, status_map);
    let flow_efficiency = task_metrics::flow_efficiency(cycle_time, lead_time);
    let proxy = report::compute_proxy(conn, &events, status_map)?;
    let tokens = report::compute_tokens(conn, &events, status_map)?;
    let task_size = git::diff_stat(cwd, task_id).unwrap_or(None);
    let has_rework = task_metrics::has_rework(&events, status_map);

    Ok(TaskReport {
        id: task_id.to_string(),
        title: title.to_string(),
        cycle_time,
        lead_time,
        flow_efficiency,
        tokens,
        task_size,
        has_rework,
        proxy,
    })
}

fn clone_report(r: &TaskReport) -> TaskReport {
    TaskReport {
        id: r.id.clone(),
        title: r.title.clone(),
        cycle_time: r.cycle_time,
        lead_time: r.lead_time,
        flow_efficiency: r.flow_efficiency,
        tokens: r.tokens.clone(),
        task_size: r.task_size.clone(),
        has_rework: r.has_rework,
        proxy: r.proxy.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::db::initialize_in_memory;
    use crate::adapter::{task_event_repo, workspace_repo};
    use crate::domain::cycle::{Assigned, Cycle};
    use crate::domain::task::{Label, Task, TaskEvent};
    use crate::domain::value::Timestamp;
    use crate::domain::workspace::{Workspace, WorkspacePrefix};

    fn setup_workspace(conn: &Connection) -> String {
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let workspace = Workspace::new("Seogi", &prefix, "test", chrono::Utc::now()).unwrap();
        workspace_repo::save(conn, &workspace).unwrap();
        workspace.id().to_string()
    }

    fn setup_cycle(conn: &Connection, workspace_id: &str) -> String {
        let cycle = Cycle::new(
            workspace_id,
            "Sprint 1",
            "2026-05-01",
            "2026-05-14",
            chrono::Utc::now(),
        )
        .unwrap();
        cycle_repo::save(conn, &cycle).unwrap();
        cycle.id().to_string()
    }

    fn setup_task(conn: &Connection, seq: i64) -> String {
        let prefix = WorkspacePrefix::new("SEO").unwrap();
        let statuses = status_repo::list_all(conn).unwrap();
        let backlog = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Backlog)
            .unwrap();
        let ws = workspace_repo::find_by_name(conn, "Seogi")
            .unwrap()
            .unwrap();
        let task = Task::new(
            &prefix,
            seq,
            &format!("Task {seq}"),
            "desc",
            Label::Feature,
            backlog.id(),
            ws.id(),
            chrono::Utc::now(),
        )
        .unwrap();
        task_repo::save(conn, &task).unwrap();
        task.id().to_string()
    }

    fn complete_task(conn: &Connection, task_id: &str) {
        let e1 = TaskEvent::new(task_id, None, "backlog", "CLI", Timestamp::new(1_000_000));
        let e2 = TaskEvent::new(
            task_id,
            Some("backlog"),
            "in_progress",
            "CLI",
            Timestamp::new(2_000_000),
        );
        let e3 = TaskEvent::new(
            task_id,
            Some("in_progress"),
            "done",
            "CLI",
            Timestamp::new(5_000_000),
        );
        task_event_repo::save(conn, &e1).unwrap();
        task_event_repo::save(conn, &e2).unwrap();
        task_event_repo::save(conn, &e3).unwrap();

        // Update task status to done
        let statuses = status_repo::list_all(conn).unwrap();
        let done = statuses
            .iter()
            .find(|s| s.category() == StatusCategory::Completed)
            .unwrap();
        task_repo::update_status(conn, task_id, done.id(), &chrono::Utc::now()).unwrap();
    }

    // Q14: cycle not found
    #[test]
    fn run_cycle_not_found() {
        let conn = initialize_in_memory().unwrap();
        let result = run(&conn, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle not found"));
    }

    // Q15: no tasks assigned
    #[test]
    fn run_no_tasks_assigned() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);

        let result = run(&conn, &cycle_id).unwrap();
        assert!(result.contains("No tasks assigned"));
    }

    // Q5: planned done with metrics
    #[test]
    fn run_planned_done_with_metrics() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);
        let task_id = setup_task(&conn, 1);

        cycle_task_repo::save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();
        complete_task(&conn, &task_id);

        let result = run(&conn, &cycle_id).unwrap();
        assert!(result.contains("Planned Done"));
        assert!(result.contains("SEO-1"));
        assert!(result.contains("completion_rate: 100%"));
    }

    // Q7: unplanned done
    #[test]
    fn run_unplanned_done() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);
        let task_id = setup_task(&conn, 1);

        cycle_task_repo::save(&conn, &cycle_id, &task_id, Assigned::Auto).unwrap();
        complete_task(&conn, &task_id);

        let result = run(&conn, &cycle_id).unwrap();
        assert!(result.contains("Unplanned Done"));
        assert!(result.contains("SEO-1"));
    }

    // Q6: planned incomplete with age
    #[test]
    fn run_planned_incomplete() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);
        let task_id = setup_task(&conn, 1);

        cycle_task_repo::save(&conn, &cycle_id, &task_id, Assigned::Planned).unwrap();

        let result = run(&conn, &cycle_id).unwrap();
        assert!(result.contains("Planned Incomplete"));
        assert!(result.contains("SEO-1"));
        assert!(result.contains("completion_rate: 0%"));
    }

    // Q13: auto + not completed → excluded
    #[test]
    fn run_auto_not_completed_excluded() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);
        let task_id = setup_task(&conn, 1);

        cycle_task_repo::save(&conn, &cycle_id, &task_id, Assigned::Auto).unwrap();
        // Task stays in backlog (not completed)

        let result = run(&conn, &cycle_id).unwrap();
        // Only the auto+backlog task → excluded → effectively no reportable tasks
        // But assigned_tasks is not empty, so we should still get a report (just with summary)
        assert!(!result.contains("Planned Done"));
        assert!(!result.contains("Unplanned Done"));
        assert!(result.contains("Summary"));
    }

    // Mixed scenario
    #[test]
    fn run_mixed_scenario() {
        let conn = initialize_in_memory().unwrap();
        let ws_id = setup_workspace(&conn);
        let cycle_id = setup_cycle(&conn, &ws_id);

        // Planned done
        let t1 = setup_task(&conn, 1);
        cycle_task_repo::save(&conn, &cycle_id, &t1, Assigned::Planned).unwrap();
        complete_task(&conn, &t1);

        // Planned incomplete
        let t2 = setup_task(&conn, 2);
        cycle_task_repo::save(&conn, &cycle_id, &t2, Assigned::Planned).unwrap();

        // Unplanned done
        let t3 = setup_task(&conn, 3);
        cycle_task_repo::save(&conn, &cycle_id, &t3, Assigned::Auto).unwrap();
        complete_task(&conn, &t3);

        let result = run(&conn, &cycle_id).unwrap();
        assert!(result.contains("Planned Done (1/2 tasks)"));
        assert!(result.contains("Planned Incomplete (1/2 tasks)"));
        assert!(result.contains("Unplanned Done (1 task)"));
        assert!(result.contains("completion_rate: 50%"));
        assert!(result.contains("throughput: 2 tasks (1 planned + 1 unplanned)"));
    }
}
