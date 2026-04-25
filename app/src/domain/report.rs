use crate::domain::metrics::TaskProxyMetrics;
use crate::domain::task_size::TaskSize;
use crate::domain::token_usage::TokenUsage;
use crate::domain::value::Ms;

/// 태스크별 통합 지표.
#[derive(Debug)]
pub struct TaskReport {
    pub id: String,
    pub title: String,
    pub cycle_time: Option<Ms>,
    pub lead_time: Option<Ms>,
    pub flow_efficiency: Option<f64>,
    pub tokens: Option<TokenUsage>,
    pub task_size: Option<TaskSize>,
    pub has_rework: bool,
    pub proxy: Option<TaskProxyMetrics>,
}

/// 밀리초를 사람이 읽을 수 있는 형식으로 변환한다.
///
/// - 0ms → "0m"
/// - 59999ms 이하 → "Xm"
/// - 60분~24시간 → "Xh Ym"
/// - 24시간 이상 → "Xd Yh"
#[must_use]
pub fn format_ms(ms: &Ms) -> String {
    let total_ms = ms.value();
    if total_ms <= 0 {
        return "0m".to_string();
    }

    let total_minutes = total_ms / 60_000;
    let total_hours = total_minutes / 60;
    let total_days = total_hours / 24;

    if total_days > 0 {
        let remaining_hours = total_hours % 24;
        if remaining_hours > 0 {
            format!("{total_days}d {remaining_hours}h")
        } else {
            format!("{total_days}d")
        }
    } else if total_hours > 0 {
        let remaining_minutes = total_minutes % 60;
        if remaining_minutes > 0 {
            format!("{total_hours}h {remaining_minutes}m")
        } else {
            format!("{total_hours}h")
        }
    } else {
        format!("{total_minutes}m")
    }
}

/// 요약 테이블 문자열을 생성한다.
#[must_use]
pub fn format_summary(
    reports: &[TaskReport],
    throughput: u32,
    avg_flow_eff: Option<f64>,
    ftd_rate: f64,
) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "{:<8}{:<20}{:<9}{:<9}{:<9}{:<8}{}",
        "ID", "TITLE", "CYCLE", "LEAD", "TOKENS", "SIZE", "REWORK"
    ));

    for r in reports {
        let cycle = r
            .cycle_time
            .as_ref()
            .map_or_else(|| "\u{2014}".to_string(), format_ms);
        let lead = r
            .lead_time
            .as_ref()
            .map_or_else(|| "\u{2014}".to_string(), format_ms);
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

        lines.push(format!(
            "{:<8}{:<20}{:<9}{:<9}{:<9}{:<8}{}",
            r.id, title, cycle, lead, tokens, size, rework
        ));
    }

    lines.push("---".to_string());

    let flow_str = avg_flow_eff.map_or_else(|| "\u{2014}".to_string(), |f| format!("{f:.2}"));
    let ftd_pct = (ftd_rate * 100.0) as u32;
    lines.push(format!(
        "throughput: {throughput} tasks    flow_efficiency(avg): {flow_str}    first_time_done: {ftd_pct}%"
    ));

    lines.join("\n")
}

/// 상세 출력 문자열을 생성한다.
#[must_use]
pub fn format_detail(reports: &[TaskReport]) -> String {
    let mut sections = Vec::new();

    for r in reports {
        let mut lines = Vec::new();
        lines.push(format!("=== {}: {} ===", r.id, r.title));

        let cycle = r
            .cycle_time
            .as_ref()
            .map_or_else(|| "\u{2014}".to_string(), format_ms);
        let lead = r
            .lead_time
            .as_ref()
            .map_or_else(|| "\u{2014}".to_string(), format_ms);
        let flow = r
            .flow_efficiency
            .map_or_else(|| "\u{2014}".to_string(), |f| format!("{f:.2}"));
        lines.push(format!(
            "cycle_time: {cycle}    lead_time: {lead}    flow_efficiency: {flow}"
        ));

        let tokens_line = r.tokens.as_ref().map_or_else(
            || "tokens: \u{2014}".to_string(),
            |t| {
                format!(
                    "tokens: {} (input: {} / output: {})",
                    format_number(t.total()),
                    format_number(t.input_tokens),
                    format_number(t.output_tokens)
                )
            },
        );
        lines.push(tokens_line);

        let size_line = r.task_size.as_ref().map_or_else(
            || "task_size: \u{2014}".to_string(),
            |s| {
                format!(
                    "task_size: +{} -{} ({} files)",
                    s.additions, s.deletions, s.files_changed
                )
            },
        );
        lines.push(size_line);

        if let Some(ref proxy) = r.proxy {
            lines.push(format!(
                "test_invoked: {}    doom_loop: {}    bash_error_rate: {:.2}",
                proxy.test_invoked, proxy.doom_loop_count, proxy.bash_error_rate
            ));
        }

        sections.push(lines.join("\n"));
    }

    sections.join("\n\n")
}

/// 숫자를 천 단위 콤마 형식으로 변환한다.
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

/// 문자열을 지정 길이로 자른다.
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

    // --- format_ms ---

    #[test]
    fn format_ms_zero() {
        assert_eq!(format_ms(&Ms::new(0)), "0m");
    }

    #[test]
    fn format_ms_negative() {
        assert_eq!(format_ms(&Ms::new(-1000)), "0m");
    }

    #[test]
    fn format_ms_minutes_only() {
        // 5 minutes = 300_000 ms
        assert_eq!(format_ms(&Ms::new(300_000)), "5m");
    }

    #[test]
    fn format_ms_less_than_one_minute() {
        // 30 seconds = 30_000 ms → 0 minutes
        assert_eq!(format_ms(&Ms::new(30_000)), "0m");
    }

    #[test]
    fn format_ms_hours_and_minutes() {
        // 2h 30m = 9_000_000 ms
        assert_eq!(format_ms(&Ms::new(9_000_000)), "2h 30m");
    }

    #[test]
    fn format_ms_exact_hours() {
        // 3h = 10_800_000 ms
        assert_eq!(format_ms(&Ms::new(10_800_000)), "3h");
    }

    #[test]
    fn format_ms_days_and_hours() {
        // 1d 4h = 100_800_000 ms
        assert_eq!(format_ms(&Ms::new(100_800_000)), "1d 4h");
    }

    #[test]
    fn format_ms_exact_days() {
        // 2d = 172_800_000 ms
        assert_eq!(format_ms(&Ms::new(172_800_000)), "2d");
    }

    // --- format_summary ---

    #[test]
    fn format_summary_with_reports() {
        let reports = vec![TaskReport {
            id: "SEO-1".to_string(),
            title: "MCP bootstrap".to_string(),
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
        }];

        let output = format_summary(&reports, 1, Some(0.52), 1.0);
        assert!(output.contains("SEO-1"));
        assert!(output.contains("MCP bootstrap"));
        assert!(output.contains("2h 30m"));
        assert!(output.contains("1d 4h"));
        assert!(output.contains("45,230"));
        assert!(output.contains("+342"));
        assert!(output.contains("no"));
        assert!(output.contains("throughput: 1 tasks"));
        assert!(output.contains("flow_efficiency(avg): 0.52"));
        assert!(output.contains("first_time_done: 100%"));
    }

    #[test]
    fn format_summary_with_none_values() {
        let reports = vec![TaskReport {
            id: "SEO-2".to_string(),
            title: "test".to_string(),
            cycle_time: None,
            lead_time: None,
            flow_efficiency: None,
            tokens: None,
            task_size: None,
            has_rework: true,
            proxy: None,
        }];

        let output = format_summary(&reports, 0, None, 0.0);
        assert!(output.contains("\u{2014}")); // em dash for None values
        assert!(output.contains("yes"));
    }

    // --- format_detail ---

    #[test]
    fn format_detail_with_full_report() {
        let reports = vec![TaskReport {
            id: "SEO-1".to_string(),
            title: "MCP server bootstrap".to_string(),
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
            proxy: Some(TaskProxyMetrics {
                read_before_edit_ratio: 3,
                doom_loop_count: 0,
                test_invoked: true,
                build_invoked: false,
                lint_invoked: false,
                typecheck_invoked: false,
                tool_call_count: 45,
                bash_error_rate: 0.02,
            }),
        }];

        let output = format_detail(&reports);
        assert!(output.contains("=== SEO-1: MCP server bootstrap ==="));
        assert!(output.contains("cycle_time: 2h 30m"));
        assert!(output.contains("lead_time: 1d 4h"));
        assert!(output.contains("flow_efficiency: 0.52"));
        assert!(output.contains("tokens: 45,230 (input: 38,120 / output: 7,110)"));
        assert!(output.contains("task_size: +342 -28 (5 files)"));
        assert!(output.contains("test_invoked: true"));
        assert!(output.contains("doom_loop: 0"));
        assert!(output.contains("bash_error_rate: 0.02"));
    }

    #[test]
    fn format_detail_with_none_values() {
        let reports = vec![TaskReport {
            id: "SEO-2".to_string(),
            title: "empty task".to_string(),
            cycle_time: None,
            lead_time: None,
            flow_efficiency: None,
            tokens: None,
            task_size: None,
            has_rework: false,
            proxy: None,
        }];

        let output = format_detail(&reports);
        assert!(output.contains("=== SEO-2: empty task ==="));
        assert!(output.contains("tokens: \u{2014}"));
        assert!(output.contains("task_size: \u{2014}"));
    }

    // --- format_number ---

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_with_commas() {
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(45_230), "45,230");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }

    // --- truncate ---

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate("a very long title here", 10);
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with('\u{2026}'));
    }
}
