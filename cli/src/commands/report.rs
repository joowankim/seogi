use anyhow::Result;

use crate::config::Config;
use crate::metrics_reader::read_metrics;
use crate::models::SessionMetricsEntry;

/// `seogi report --from <date> --to <date> [--project <name>]`
///
/// # Errors
///
/// Returns an error if metrics files cannot be read.
pub fn run(config: &Config, from: &str, to: &str, project: Option<&str>) -> Result<()> {
    let log_dir = config.log_dir_expanded();
    let entries = read_metrics(&log_dir, project, from, to)?;

    if entries.is_empty() {
        println!("해당 기간에 데이터가 없습니다.");
        return Ok(());
    }

    let n = entries.len();
    let project_label = project.unwrap_or("전체");

    println!("기간: {from} ~ {to} (n={n} 세션)");
    println!("프로젝트: {project_label}");
    println!();

    print_numeric_stats(&entries);
    println!();
    print_boolean_stats(&entries);

    Ok(())
}

fn print_numeric_stats(entries: &[SessionMetricsEntry]) {
    println!(
        "{:<24} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "", "평균", "중앙값", "σ", "P25", "P75"
    );

    let rbe: Vec<f64> = entries
        .iter()
        .map(|e| f64::from(e.metrics.read_before_edit_ratio))
        .collect();
    print_stat_row("read_before_edit", &rbe);

    let doom: Vec<f64> = entries
        .iter()
        .map(|e| f64::from(e.metrics.doom_loop_count))
        .collect();
    print_stat_row("doom_loop_count", &doom);

    let tools: Vec<f64> = entries
        .iter()
        .map(|e| f64::from(e.metrics.tool_call_count))
        .collect();
    print_stat_row("tool_call_count", &tools);

    let duration: Vec<f64> = entries
        .iter()
        .map(|e| e.metrics.session_duration_ms as f64 / 1000.0)
        .collect();
    print_stat_row("session_duration_sec", &duration);

    let bash_err: Vec<f64> = entries
        .iter()
        .filter_map(|e| e.metrics.bash_error_rate)
        .collect();
    if !bash_err.is_empty() {
        print_stat_row_pct("bash_error_rate", &bash_err);
    }

    let edit_files: Vec<f64> = entries
        .iter()
        .map(|e| e.metrics.edit_files.len() as f64)
        .collect();
    print_stat_row("edit_files_count", &edit_files);
}

fn print_boolean_stats(entries: &[SessionMetricsEntry]) {
    let n = entries.len() as f64;

    let test_pct = entries.iter().filter(|e| e.metrics.test_invoked).count() as f64 / n * 100.0;
    println!("{:<24} {:>7.0}%", "test_invoked", test_pct);

    let build_pct = entries.iter().filter(|e| e.metrics.build_invoked).count() as f64 / n * 100.0;
    println!("{:<24} {:>7.0}%", "build_invoked", build_pct);

    let lint_entries: Vec<_> = entries
        .iter()
        .filter_map(|e| e.metrics.lint_invoked)
        .collect();
    if !lint_entries.is_empty() {
        let lint_pct =
            lint_entries.iter().filter(|&&v| v).count() as f64 / lint_entries.len() as f64 * 100.0;
        println!("{:<24} {:>7.0}%", "lint_invoked", lint_pct);
    }

    let tc_entries: Vec<_> = entries
        .iter()
        .filter_map(|e| e.metrics.typecheck_invoked)
        .collect();
    if !tc_entries.is_empty() {
        let tc_pct =
            tc_entries.iter().filter(|&&v| v).count() as f64 / tc_entries.len() as f64 * 100.0;
        println!("{:<24} {:>7.0}%", "typecheck_invoked", tc_pct);
    }
}

fn print_stat_row(label: &str, values: &[f64]) {
    let stats = compute_stats(values);
    println!(
        "{:<24} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1}",
        label, stats.mean, stats.median, stats.stddev, stats.p25, stats.p75
    );
}

fn print_stat_row_pct(label: &str, values: &[f64]) {
    let stats = compute_stats(values);
    println!(
        "{:<24} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}%",
        label,
        stats.mean * 100.0,
        stats.median * 100.0,
        stats.stddev * 100.0,
        stats.p25 * 100.0,
        stats.p75 * 100.0
    );
}

#[derive(Debug)]
struct Stats {
    mean: f64,
    median: f64,
    stddev: f64,
    p25: f64,
    p75: f64,
}

fn compute_stats(values: &[f64]) -> Stats {
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
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median = percentile(&sorted, 0.5);
    let p25 = percentile(&sorted, 0.25);
    let p75 = percentile(&sorted, 0.75);

    let variance = if values.len() > 1 {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    let stddev = variance.sqrt();

    Stats {
        mean,
        median,
        stddev,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_stats_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = compute_stats(&values);
        assert!((stats.mean - 3.0).abs() < f64::EPSILON);
        assert!((stats.median - 3.0).abs() < f64::EPSILON);
        assert!((stats.p25 - 2.0).abs() < f64::EPSILON);
        assert!((stats.p75 - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_stats_single_value() {
        let values = vec![42.0];
        let stats = compute_stats(&values);
        assert!((stats.mean - 42.0).abs() < f64::EPSILON);
        assert!((stats.stddev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_stats_empty() {
        let stats = compute_stats(&[]);
        assert!((stats.mean - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentile_basic() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((percentile(&sorted, 0.5) - 6.0).abs() < f64::EPSILON);
        assert!((percentile(&sorted, 0.25) - 3.0).abs() < f64::EPSILON);
    }
}
