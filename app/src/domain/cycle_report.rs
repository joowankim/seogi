use crate::domain::cycle::Assigned;
use crate::domain::report::{self, TaskReport};
use crate::domain::status::StatusCategory;
use crate::domain::value::Ms;

/// Cycle report에서 태스크를 분류하는 카테고리.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleReportCategory {
    PlannedDone,
    PlannedIncomplete,
    UnplannedDone,
}

/// `assigned` + `status_category` 조합으로 리포트 분류를 결정한다.
///
/// `assigned=auto AND status_category!=Completed`는 리포트 대상에서 제외 (None 반환).
#[must_use]
pub fn classify(
    assigned: Assigned,
    status_category: StatusCategory,
) -> Option<CycleReportCategory> {
    match (assigned, status_category) {
        (Assigned::Planned, StatusCategory::Completed) => Some(CycleReportCategory::PlannedDone),
        (Assigned::Planned, _) => Some(CycleReportCategory::PlannedIncomplete),
        (Assigned::Auto, StatusCategory::Completed) => Some(CycleReportCategory::UnplannedDone),
        (Assigned::Auto, _) => None,
    }
}

/// Planned Incomplete 태스크의 표시 정보.
#[derive(Debug)]
pub struct IncompleteTask {
    pub id: String,
    pub title: String,
    pub status_name: String,
    pub issue_age: Option<Ms>,
}

/// Cycle 전체 요약 지표.
#[derive(Debug)]
pub struct CycleSummary {
    pub completion_rate: f64,
    pub planned_done_count: usize,
    pub planned_total: usize,
    pub throughput: usize,
    pub planned_throughput: usize,
    pub unplanned_throughput: usize,
    pub avg_flow_eff: Option<f64>,
    pub ftd_rate: f64,
}

/// 요약 지표를 계산한다.
#[must_use]
pub fn compute_summary(
    planned_done: usize,
    planned_incomplete: usize,
    done_reports: &[TaskReport],
) -> CycleSummary {
    let planned_total = planned_done + planned_incomplete;
    let completion_rate = if planned_total == 0 {
        0.0
    } else {
        planned_done as f64 / planned_total as f64
    };

    let throughput = done_reports.len();
    let unplanned_throughput = throughput.saturating_sub(planned_done);

    let effs: Vec<f64> = done_reports
        .iter()
        .filter_map(|r| r.flow_efficiency)
        .collect();
    let avg_flow_eff = if effs.is_empty() {
        None
    } else {
        Some(effs.iter().sum::<f64>() / effs.len() as f64)
    };

    let rework_count = done_reports.iter().filter(|r| r.has_rework).count();
    let ftd_rate = if done_reports.is_empty() {
        1.0
    } else {
        (done_reports.len() - rework_count) as f64 / done_reports.len() as f64
    };

    CycleSummary {
        completion_rate,
        planned_done_count: planned_done,
        planned_total,
        throughput,
        planned_throughput: planned_done,
        unplanned_throughput,
        avg_flow_eff,
        ftd_rate,
    }
}

/// `format_cycle_report` 입력 파라미터.
pub struct FormatCycleReportInput<'a> {
    pub cycle_name: &'a str,
    pub start_date: &'a str,
    pub end_date: &'a str,
    pub status: &'a str,
    pub planned_done: &'a [TaskReport],
    pub planned_incomplete: &'a [IncompleteTask],
    pub unplanned_done: &'a [TaskReport],
    pub summary: &'a CycleSummary,
}

/// Cycle report 전체를 포맷한다.
#[must_use]
pub fn format_cycle_report(input: &FormatCycleReportInput<'_>) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "=== Cycle Report: \"{}\" ({} ~ {}, {}) ===",
        input.cycle_name, input.start_date, input.end_date, input.status
    ));

    if !input.planned_done.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "--- Planned Done ({}/{} tasks) ---",
            input.planned_done.len(),
            input.summary.planned_total
        ));
        lines.push(format_done_header());
        for r in input.planned_done {
            lines.push(format_done_row(r));
        }
    }

    if !input.planned_incomplete.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "--- Planned Incomplete ({}/{} tasks) ---",
            input.planned_incomplete.len(),
            input.summary.planned_total
        ));
        lines.push(format_incomplete_header());
        for t in input.planned_incomplete {
            lines.push(format_incomplete_row(t));
        }
    }

    if !input.unplanned_done.is_empty() {
        lines.push(String::new());
        let label = if input.unplanned_done.len() == 1 {
            "task"
        } else {
            "tasks"
        };
        lines.push(format!(
            "--- Unplanned Done ({} {label}) ---",
            input.unplanned_done.len()
        ));
        lines.push(format_done_header());
        for r in input.unplanned_done {
            lines.push(format_done_row(r));
        }
    }

    lines.push(String::new());
    lines.push("--- Summary ---".to_string());
    let pct = (input.summary.completion_rate * 100.0) as u32;
    lines.push(format!(
        "completion_rate: {pct}% ({}/{}  planned)",
        input.summary.planned_done_count, input.summary.planned_total
    ));
    lines.push(format!(
        "throughput: {} tasks ({} planned + {} unplanned)",
        input.summary.throughput,
        input.summary.planned_throughput,
        input.summary.unplanned_throughput
    ));
    let flow_str = input
        .summary
        .avg_flow_eff
        .map_or_else(|| "\u{2014}".to_string(), |f| format!("{f:.2}"));
    lines.push(format!("flow_efficiency(avg): {flow_str}"));
    let ftd_pct = (input.summary.ftd_rate * 100.0) as u32;
    lines.push(format!("first_time_done: {ftd_pct}%"));

    lines.join("\n")
}

fn format_done_header() -> String {
    format!(
        "{:<8}{:<20}{:<9}{:<9}{:<9}{:<8}{}",
        "ID", "TITLE", "CYCLE", "LEAD", "TOKENS", "SIZE", "REWORK"
    )
}

fn format_done_row(r: &TaskReport) -> String {
    let cycle = r
        .cycle_time
        .as_ref()
        .map_or_else(|| "\u{2014}".to_string(), report::format_ms);
    let lead = r
        .lead_time
        .as_ref()
        .map_or_else(|| "\u{2014}".to_string(), report::format_ms);
    let tokens = r
        .tokens
        .as_ref()
        .map_or_else(|| "\u{2014}".to_string(), |t| format_number(t.total()));
    let size = r
        .task_size
        .as_ref()
        .map_or_else(|| "\u{2014}".to_string(), |s| format!("+{}", s.additions));
    let rework = if r.has_rework { "yes" } else { "no" };
    let title = truncate(&r.title, 18);

    format!(
        "{:<8}{:<20}{:<9}{:<9}{:<9}{:<8}{}",
        r.id, title, cycle, lead, tokens, size, rework
    )
}

fn format_incomplete_header() -> String {
    format!("{:<8}{:<20}{:<14}{}", "ID", "TITLE", "STATUS", "AGE")
}

fn format_incomplete_row(t: &IncompleteTask) -> String {
    let age = t
        .issue_age
        .as_ref()
        .map_or_else(|| "\u{2014}".to_string(), report::format_ms);
    let title = truncate(&t.title, 18);

    format!("{:<8}{:<20}{:<14}{}", t.id, title, t.status_name, age)
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{truncated}\u{2026}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task_size::TaskSize;
    use crate::domain::token_usage::TokenUsage;

    // Q2: assigned=planned + Completed → PlannedDone
    #[test]
    fn classify_planned_completed() {
        assert_eq!(
            classify(Assigned::Planned, StatusCategory::Completed),
            Some(CycleReportCategory::PlannedDone)
        );
    }

    // Q3: assigned=planned + Started → PlannedIncomplete
    #[test]
    fn classify_planned_started() {
        assert_eq!(
            classify(Assigned::Planned, StatusCategory::Started),
            Some(CycleReportCategory::PlannedIncomplete)
        );
    }

    // Q3: assigned=planned + Backlog → PlannedIncomplete
    #[test]
    fn classify_planned_backlog() {
        assert_eq!(
            classify(Assigned::Planned, StatusCategory::Backlog),
            Some(CycleReportCategory::PlannedIncomplete)
        );
    }

    // Q4: assigned=auto + Completed → UnplannedDone
    #[test]
    fn classify_auto_completed() {
        assert_eq!(
            classify(Assigned::Auto, StatusCategory::Completed),
            Some(CycleReportCategory::UnplannedDone)
        );
    }

    // Q13: assigned=auto + Started → None (excluded)
    #[test]
    fn classify_auto_not_completed_excluded() {
        assert_eq!(classify(Assigned::Auto, StatusCategory::Started), None);
        assert_eq!(classify(Assigned::Auto, StatusCategory::Backlog), None);
        assert_eq!(classify(Assigned::Auto, StatusCategory::Unstarted), None);
        assert_eq!(classify(Assigned::Auto, StatusCategory::Canceled), None);
    }

    // Q8: completion_rate = planned_done / (planned_done + planned_incomplete)
    #[test]
    fn summary_completion_rate() {
        let reports = vec![make_report("SEO-1", false)];
        let s = compute_summary(1, 1, &reports);
        assert!((s.completion_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(s.planned_done_count, 1);
        assert_eq!(s.planned_total, 2);
    }

    // Q8: planned_total=0 → completion_rate=0
    #[test]
    fn summary_no_planned_tasks() {
        let s = compute_summary(0, 0, &[]);
        assert!((s.completion_rate - 0.0).abs() < f64::EPSILON);
    }

    // Q9: throughput = total done reports
    #[test]
    fn summary_throughput() {
        let reports = vec![
            make_report("SEO-1", false),
            make_report("SEO-2", false),
            make_report("SEO-3", false),
        ];
        let s = compute_summary(2, 1, &reports);
        assert_eq!(s.throughput, 3);
        assert_eq!(s.planned_throughput, 2);
        assert_eq!(s.unplanned_throughput, 1);
    }

    // Q10: flow_efficiency avg
    #[test]
    fn summary_avg_flow_eff() {
        let mut r1 = make_report("SEO-1", false);
        r1.flow_efficiency = Some(0.4);
        let mut r2 = make_report("SEO-2", false);
        r2.flow_efficiency = Some(0.6);
        let s = compute_summary(2, 0, &[r1, r2]);
        assert!((s.avg_flow_eff.unwrap() - 0.5).abs() < f64::EPSILON);
    }

    // Q10: no flow_efficiency → None
    #[test]
    fn summary_avg_flow_eff_none() {
        let reports = vec![make_report("SEO-1", false)];
        let s = compute_summary(1, 0, &reports);
        assert!(s.avg_flow_eff.is_none());
    }

    // Q11: first_time_done rate
    #[test]
    fn summary_ftd_rate() {
        let reports = vec![
            make_report("SEO-1", false),
            make_report("SEO-2", true),
            make_report("SEO-3", false),
        ];
        let s = compute_summary(3, 0, &reports);
        assert!((s.ftd_rate - 2.0 / 3.0).abs() < 0.01);
    }

    // Q11: no done reports → ftd_rate=1.0
    #[test]
    fn summary_ftd_no_tasks() {
        let s = compute_summary(0, 2, &[]);
        assert!((s.ftd_rate - 1.0).abs() < f64::EPSILON);
    }

    // Q1: header format
    #[test]
    fn format_report_header() {
        let summary = CycleSummary {
            completion_rate: 0.75,
            planned_done_count: 3,
            planned_total: 4,
            throughput: 4,
            planned_throughput: 3,
            unplanned_throughput: 1,
            avg_flow_eff: Some(0.52),
            ftd_rate: 0.75,
        };
        let output = format_cycle_report(&FormatCycleReportInput {
            cycle_name: "Sprint 1",
            start_date: "2026-05-01",
            end_date: "2026-05-14",
            status: "active",
            planned_done: &[],
            planned_incomplete: &[],
            unplanned_done: &[],
            summary: &summary,
        });
        assert!(
            output.contains("=== Cycle Report: \"Sprint 1\" (2026-05-01 ~ 2026-05-14, active) ===")
        );
    }

    // Q12: empty section omitted
    #[test]
    fn format_report_omits_empty_sections() {
        let summary = compute_summary(0, 0, &[]);
        let output = format_cycle_report(&FormatCycleReportInput {
            cycle_name: "Sprint 1",
            start_date: "2026-05-01",
            end_date: "2026-05-14",
            status: "active",
            planned_done: &[],
            planned_incomplete: &[],
            unplanned_done: &[],
            summary: &summary,
        });
        assert!(!output.contains("Planned Done"));
        assert!(!output.contains("Planned Incomplete"));
        assert!(!output.contains("Unplanned Done"));
        assert!(output.contains("Summary"));
    }

    // Q12: sections present when non-empty
    #[test]
    fn format_report_includes_sections() {
        let planned_done = vec![make_report_with_metrics("SEO-1")];
        let planned_incomplete = vec![IncompleteTask {
            id: "SEO-2".to_string(),
            title: "incomplete task".to_string(),
            status_name: "in_progress".to_string(),
            issue_age: Some(Ms::new(180_000_000)),
        }];
        let unplanned_done = vec![make_report_with_metrics("SEO-3")];
        let summary = compute_summary(
            1,
            1,
            &[
                make_report_with_metrics("SEO-1"),
                make_report_with_metrics("SEO-3"),
            ],
        );

        let output = format_cycle_report(&FormatCycleReportInput {
            cycle_name: "Sprint 1",
            start_date: "2026-05-01",
            end_date: "2026-05-14",
            status: "active",
            planned_done: &planned_done,
            planned_incomplete: &planned_incomplete,
            unplanned_done: &unplanned_done,
            summary: &summary,
        });
        assert!(output.contains("Planned Done (1/2 tasks)"));
        assert!(output.contains("Planned Incomplete (1/2 tasks)"));
        assert!(output.contains("Unplanned Done (1 task)"));
        assert!(output.contains("completion_rate: 50%"));
        assert!(output.contains("throughput: 2 tasks (1 planned + 1 unplanned)"));
    }

    fn make_report(id: &str, has_rework: bool) -> TaskReport {
        TaskReport {
            id: id.to_string(),
            title: "test".to_string(),
            cycle_time: None,
            lead_time: None,
            flow_efficiency: None,
            tokens: None,
            task_size: None,
            has_rework,
            proxy: None,
        }
    }

    fn make_report_with_metrics(id: &str) -> TaskReport {
        TaskReport {
            id: id.to_string(),
            title: "test task".to_string(),
            cycle_time: Some(Ms::new(9_000_000)),
            lead_time: Some(Ms::new(100_800_000)),
            flow_efficiency: Some(0.52),
            tokens: Some(TokenUsage {
                input_tokens: 38_120,
                output_tokens: 7_110,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            task_size: Some(TaskSize {
                additions: 342,
                deletions: 28,
                files_changed: 5,
            }),
            has_rework: false,
            proxy: None,
        }
    }
}
