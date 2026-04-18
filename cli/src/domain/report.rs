use std::fmt::Write;

use crate::domain::metrics::SessionMetrics;

/// 통계 요약.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub mean: f64,
    pub median: f64,
    pub stddev: f64,
    pub p25: f64,
    pub p75: f64,
}

/// 수치 배열의 통계(평균, 중앙값, σ, P25, P75)를 계산한다.
///
/// # Panics
///
/// 배열에 `NaN`이 포함되면 정렬 시 패닉한다.
#[must_use]
pub fn compute_stats(values: &[f64]) -> Stats {
    if values.is_empty() {
        return Stats {
            mean: 0.0,
            median: 0.0,
            stddev: 0.0,
            p25: 0.0,
            p75: 0.0,
        };
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in values"));

    let median = percentile(&sorted, 0.5);
    let p25 = percentile(&sorted, 0.25);
    let p75 = percentile(&sorted, 0.75);

    let variance = if values.len() > 1 {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };

    Stats {
        mean,
        median,
        stddev: variance.sqrt(),
        p25,
        p75,
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (sorted.len() as f64 * p).floor() as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx]
}

/// 세션 메트릭 목록에서 리포트 문자열을 생성한다.
#[must_use]
pub fn format_report(
    metrics: &[SessionMetrics],
    from: &str,
    to: &str,
    project: Option<&str>,
) -> String {
    if metrics.is_empty() {
        return "해당 기간에 데이터가 없습니다.\n".to_string();
    }

    let n = metrics.len();
    let project_label = project.unwrap_or("전체");

    let mut out = String::new();
    let _ = writeln!(out, "기간: {from} ~ {to} (n={n} 세션)");
    let _ = writeln!(out, "프로젝트: {project_label}");
    out.push('\n');

    let _ = writeln!(
        out,
        "{:<24} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "", "평균", "중앙값", "σ", "P25", "P75"
    );

    format_stat_row(
        &mut out,
        "read_before_edit",
        &extract(metrics, |m| f64::from(m.read_before_edit_ratio())),
    );
    format_stat_row(
        &mut out,
        "doom_loop_count",
        &extract(metrics, |m| f64::from(m.doom_loop_count())),
    );
    format_stat_row(
        &mut out,
        "tool_call_count",
        &extract(metrics, |m| f64::from(m.tool_call_count())),
    );
    format_stat_row(
        &mut out,
        "session_duration_sec",
        &extract(metrics, |m| m.session_duration_ms() as f64 / 1000.0),
    );
    format_stat_row_pct(
        &mut out,
        "bash_error_rate",
        &extract(metrics, SessionMetrics::bash_error_rate),
    );
    format_stat_row(
        &mut out,
        "edit_files_count",
        &extract(metrics, |m| m.edit_files().len() as f64),
    );

    out.push('\n');

    format_bool_row(
        &mut out,
        "test_invoked",
        metrics,
        SessionMetrics::test_invoked,
    );
    format_bool_row(
        &mut out,
        "build_invoked",
        metrics,
        SessionMetrics::build_invoked,
    );
    format_bool_row(
        &mut out,
        "lint_invoked",
        metrics,
        SessionMetrics::lint_invoked,
    );
    format_bool_row(
        &mut out,
        "typecheck_invoked",
        metrics,
        SessionMetrics::typecheck_invoked,
    );

    out
}

fn extract(metrics: &[SessionMetrics], f: impl Fn(&SessionMetrics) -> f64) -> Vec<f64> {
    metrics.iter().map(f).collect()
}

fn format_stat_row(out: &mut String, label: &str, values: &[f64]) {
    let s = compute_stats(values);
    let _ = writeln!(
        out,
        "{label:<24} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1}",
        s.mean, s.median, s.stddev, s.p25, s.p75
    );
}

fn format_stat_row_pct(out: &mut String, label: &str, values: &[f64]) {
    let s = compute_stats(values);
    let _ = writeln!(
        out,
        "{label:<24} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}%",
        s.mean * 100.0,
        s.median * 100.0,
        s.stddev * 100.0,
        s.p25 * 100.0,
        s.p75 * 100.0
    );
}

fn format_bool_row(
    out: &mut String,
    label: &str,
    metrics: &[SessionMetrics],
    f: impl Fn(&SessionMetrics) -> bool,
) {
    let n = metrics.len() as f64;
    let pct = metrics.iter().filter(|m| f(m)).count() as f64 / n * 100.0;
    let _ = writeln!(out, "{label:<24} {pct:>7.0}%");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::log::ToolUse;
    use crate::domain::metrics;
    use crate::domain::value::{Ms, SessionId, Timestamp};

    #[test]
    fn test_compute_stats_basic() {
        let s = compute_stats(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((s.mean - 3.0).abs() < f64::EPSILON);
        assert!((s.median - 3.0).abs() < f64::EPSILON);
        assert!((s.p25 - 2.0).abs() < f64::EPSILON);
        assert!((s.p75 - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_stats_stddev() {
        let s = compute_stats(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let expected = (2.5_f64).sqrt();
        assert!((s.stddev - expected).abs() < 0.001);
    }

    #[test]
    fn test_compute_stats_single() {
        let s = compute_stats(&[42.0]);
        assert!((s.mean - 42.0).abs() < f64::EPSILON);
        assert!((s.stddev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_stats_empty() {
        let s = compute_stats(&[]);
        assert!((s.mean - 0.0).abs() < f64::EPSILON);
    }

    fn make_metrics(session_id: &str, test_invoked: bool) -> SessionMetrics {
        let tu = ToolUse::new(
            "id".to_string(),
            SessionId::new(session_id),
            "proj".to_string(),
            "/proj".to_string(),
            if test_invoked { "Bash" } else { "Read" }.to_string(),
            if test_invoked {
                r#"{"command":"cargo test"}"#.to_string()
            } else {
                "{}".to_string()
            },
            Ms::zero(),
            Timestamp::new(1000),
        );
        metrics::calculate(session_id, &[tu], &[])
    }

    #[test]
    fn test_format_report_with_data() {
        let m = vec![
            make_metrics("s1", true),
            make_metrics("s2", true),
            make_metrics("s3", false),
        ];
        let out = format_report(&m, "2026-04-01", "2026-04-15", None);
        assert!(out.contains("n=3"), "should contain n=3: {out}");
        assert!(out.contains("67%"), "test_invoked 2/3=67%: {out}");
        assert!(out.contains("전체"), "project label: {out}");
    }

    #[test]
    fn test_format_report_empty() {
        let out = format_report(&[], "2026-04-01", "2026-04-15", None);
        assert!(out.contains("데이터가 없습니다"));
    }
}
