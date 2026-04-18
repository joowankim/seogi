use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::log::{ToolFailure, ToolUse};
use super::value::{Ms, SessionId};

static TEST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(test|vitest|playwright|jest|pytest|mocha|karma)\b")
        .expect("static regex is valid")
});
static BUILD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(build|tsc|webpack|vite build|esbuild|rollup)\b")
        .expect("static regex is valid")
});
static LINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(lint|eslint|prettier|ruff|biome)\b").expect("static regex is valid")
});
static TYPECHECK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(tsc\s+--noEmit|mypy|pyright)\b").expect("static regex is valid")
});

/// 세션 프록시 지표 10개.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SessionMetrics {
    session_id: SessionId,
    read_before_edit_ratio: u32,
    doom_loop_count: u32,
    test_invoked: bool,
    build_invoked: bool,
    lint_invoked: bool,
    typecheck_invoked: bool,
    tool_call_count: u32,
    #[serde(rename = "session_duration_ms")]
    session_duration: Ms,
    edit_files: Vec<String>,
    bash_error_rate: f64,
}

impl SessionMetrics {
    #[must_use]
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    #[must_use]
    pub fn read_before_edit_ratio(&self) -> u32 {
        self.read_before_edit_ratio
    }

    #[must_use]
    pub fn doom_loop_count(&self) -> u32 {
        self.doom_loop_count
    }

    #[must_use]
    pub fn test_invoked(&self) -> bool {
        self.test_invoked
    }

    #[must_use]
    pub fn build_invoked(&self) -> bool {
        self.build_invoked
    }

    #[must_use]
    pub fn lint_invoked(&self) -> bool {
        self.lint_invoked
    }

    #[must_use]
    pub fn typecheck_invoked(&self) -> bool {
        self.typecheck_invoked
    }

    #[must_use]
    pub fn tool_call_count(&self) -> u32 {
        self.tool_call_count
    }

    #[must_use]
    pub fn session_duration(&self) -> Ms {
        self.session_duration
    }

    #[must_use]
    pub fn edit_files(&self) -> &[String] {
        &self.edit_files
    }

    #[must_use]
    pub fn bash_error_rate(&self) -> f64 {
        self.bash_error_rate
    }
}

/// `ToolUse`와 `ToolFailure` 목록에서 10개 프록시 지표를 계산한다.
///
/// 순수 함수. I/O 없음.
#[must_use]
pub fn calculate(
    session_id: SessionId,
    tool_uses: &[ToolUse],
    tool_failures: &[ToolFailure],
) -> SessionMetrics {
    SessionMetrics {
        session_id,
        read_before_edit_ratio: calc_read_before_edit(tool_uses),
        doom_loop_count: calc_doom_loop_count(tool_uses),
        test_invoked: calc_invoked(tool_uses, &TEST_PATTERN),
        build_invoked: calc_invoked(tool_uses, &BUILD_PATTERN),
        lint_invoked: calc_invoked(tool_uses, &LINT_PATTERN),
        typecheck_invoked: calc_invoked(tool_uses, &TYPECHECK_PATTERN),
        tool_call_count: tool_uses.len() as u32,
        session_duration: calc_session_duration(tool_uses),
        edit_files: calc_edit_files(tool_uses),
        bash_error_rate: calc_bash_error_rate(tool_uses, tool_failures),
    }
}

fn bash_command(tool_use: &ToolUse) -> String {
    serde_json::from_str::<serde_json::Value>(tool_use.tool_input())
        .ok()
        .and_then(|v| v.get("command")?.as_str().map(String::from))
        .unwrap_or_default()
}

fn file_path(tool_use: &ToolUse) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(tool_use.tool_input())
        .ok()
        .and_then(|v| v.get("file_path")?.as_str().map(String::from))
}

/// 1. 첫 Edit/Write 전 Read/Grep/Glob 호출 수
fn calc_read_before_edit(tool_uses: &[ToolUse]) -> u32 {
    let read_tools = ["Read", "Grep", "Glob"];
    let edit_tools = ["Edit", "Write"];

    let first_edit_idx = tool_uses
        .iter()
        .position(|tu| edit_tools.contains(&tu.tool_name()));

    let end = first_edit_idx.unwrap_or(tool_uses.len());

    tool_uses[..end]
        .iter()
        .filter(|tu| read_tools.contains(&tu.tool_name()))
        .count() as u32
}

/// 2. 동일 파일 Edit 5회 이상인 파일 수
fn calc_doom_loop_count(tool_uses: &[ToolUse]) -> u32 {
    let mut file_counts: HashMap<String, u32> = HashMap::new();

    for tu in tool_uses {
        if tu.tool_name() == "Edit"
            && let Some(path) = file_path(tu)
        {
            *file_counts.entry(path).or_insert(0) += 1;
        }
    }

    file_counts.values().filter(|&&count| count >= 5).count() as u32
}

/// 3-6. Bash command 패턴 매칭
fn calc_invoked(tool_uses: &[ToolUse], pattern: &Regex) -> bool {
    tool_uses
        .iter()
        .filter(|tu| tu.tool_name() == "Bash")
        .any(|tu| pattern.is_match(&bash_command(tu)))
}

/// 8. 첫 도구 호출 ~ 마지막 도구 호출 시간차
fn calc_session_duration(tool_uses: &[ToolUse]) -> Ms {
    if tool_uses.len() <= 1 {
        return Ms::zero();
    }

    let first = tool_uses.first().unwrap().timestamp().value();
    let last = tool_uses.last().unwrap().timestamp().value();
    Ms::new(last - first)
}

/// 9. Edit/Write한 고유 파일 목록
fn calc_edit_files(tool_uses: &[ToolUse]) -> Vec<String> {
    let mut files: Vec<String> = tool_uses
        .iter()
        .filter(|tu| tu.tool_name() == "Edit" || tu.tool_name() == "Write")
        .filter_map(file_path)
        .collect();

    files.sort();
    files.dedup();
    files
}

/// 10. Bash 실패 비율
fn calc_bash_error_rate(tool_uses: &[ToolUse], tool_failures: &[ToolFailure]) -> f64 {
    let bash_count = tool_uses
        .iter()
        .filter(|tu| tu.tool_name() == "Bash")
        .count();

    if bash_count == 0 {
        return 0.0;
    }

    let bash_failures = tool_failures
        .iter()
        .filter(|tf| tf.tool_name() == "Bash")
        .count();

    bash_failures as f64 / bash_count as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value::{Ms, SessionId, Timestamp};

    fn make_tool_use(name: &str, input: &str, ts: i64) -> ToolUse {
        ToolUse::new(
            format!("id-{ts}"),
            SessionId::new("sess-1"),
            "test".to_string(),
            "/test".to_string(),
            name.to_string(),
            input.to_string(),
            Ms::zero(),
            Timestamp::new(ts),
        )
    }

    fn make_failure(name: &str, ts: i64) -> ToolFailure {
        ToolFailure::new(
            format!("fail-{ts}"),
            SessionId::new("sess-1"),
            "test".to_string(),
            "/test".to_string(),
            name.to_string(),
            "error".to_string(),
            Timestamp::new(ts),
        )
    }

    #[test]
    fn test_read_before_edit_ratio() {
        let uses = vec![
            make_tool_use("Read", "{}", 1000),
            make_tool_use("Grep", "{}", 2000),
            make_tool_use("Read", "{}", 3000),
            make_tool_use("Edit", r#"{"file_path":"a.rs"}"#, 4000),
        ];
        assert_eq!(calc_read_before_edit(&uses), 3);
    }

    #[test]
    fn test_read_before_edit_no_edit() {
        let uses = vec![
            make_tool_use("Read", "{}", 1000),
            make_tool_use("Glob", "{}", 2000),
            make_tool_use("Read", "{}", 3000),
        ];
        assert_eq!(calc_read_before_edit(&uses), 3);
    }

    #[test]
    fn test_doom_loop_above_threshold() {
        let uses: Vec<_> = (0..6)
            .map(|i| make_tool_use("Edit", r#"{"file_path":"same.rs"}"#, 1000 + i))
            .collect();
        assert_eq!(calc_doom_loop_count(&uses), 1);
    }

    #[test]
    fn test_doom_loop_below_threshold() {
        let uses: Vec<_> = (0..4)
            .map(|i| make_tool_use("Edit", r#"{"file_path":"same.rs"}"#, 1000 + i))
            .collect();
        assert_eq!(calc_doom_loop_count(&uses), 0);
    }

    #[test]
    fn test_test_invoked_true() {
        let uses = vec![make_tool_use("Bash", r#"{"command":"cargo pytest"}"#, 1000)];
        assert!(calc_invoked(&uses, &TEST_PATTERN));
    }

    #[test]
    fn test_test_invoked_false() {
        let uses = vec![make_tool_use("Bash", r#"{"command":"ls -la"}"#, 1000)];
        assert!(!calc_invoked(&uses, &TEST_PATTERN));
    }

    #[test]
    fn test_build_invoked() {
        let uses = vec![make_tool_use("Bash", r#"{"command":"npx webpack"}"#, 1000)];
        assert!(calc_invoked(&uses, &BUILD_PATTERN));
    }

    #[test]
    fn test_lint_invoked() {
        let uses = vec![make_tool_use("Bash", r#"{"command":"npx eslint ."}"#, 1000)];
        assert!(calc_invoked(&uses, &LINT_PATTERN));
    }

    #[test]
    fn test_typecheck_invoked() {
        let uses = vec![make_tool_use(
            "Bash",
            r#"{"command":"npx tsc --noEmit"}"#,
            1000,
        )];
        assert!(calc_invoked(&uses, &TYPECHECK_PATTERN));
    }

    #[test]
    fn test_tool_call_count() {
        let uses: Vec<_> = (0..5)
            .map(|i| make_tool_use("Read", "{}", 1000 + i))
            .collect();
        let m = calculate(SessionId::new("s1"), &uses, &[]);
        assert_eq!(m.tool_call_count(), 5);
    }

    #[test]
    fn test_session_duration() {
        let uses = vec![
            make_tool_use("Read", "{}", 1000),
            make_tool_use("Edit", r#"{"file_path":"a.rs"}"#, 5000),
        ];
        assert_eq!(calc_session_duration(&uses), Ms::new(4000));
    }

    #[test]
    fn test_session_duration_single() {
        let uses = vec![make_tool_use("Read", "{}", 1000)];
        assert_eq!(calc_session_duration(&uses), Ms::zero());
    }

    #[test]
    fn test_edit_files() {
        let uses = vec![
            make_tool_use("Edit", r#"{"file_path":"a.rs"}"#, 1000),
            make_tool_use("Edit", r#"{"file_path":"b.rs"}"#, 2000),
            make_tool_use("Edit", r#"{"file_path":"a.rs"}"#, 3000),
        ];
        assert_eq!(calc_edit_files(&uses), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn test_bash_error_rate() {
        let uses: Vec<_> = (0..10)
            .map(|i| make_tool_use("Bash", r#"{"command":"ls"}"#, 1000 + i))
            .collect();
        let failures = vec![make_failure("Bash", 1000), make_failure("Bash", 1001)];
        let rate = calc_bash_error_rate(&uses, &failures);
        assert!((rate - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bash_error_rate_no_bash() {
        let uses = vec![make_tool_use("Read", "{}", 1000)];
        assert!((calc_bash_error_rate(&uses, &[]) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_session() {
        let m = calculate(SessionId::new("s1"), &[], &[]);
        assert_eq!(m.read_before_edit_ratio(), 0);
        assert_eq!(m.doom_loop_count(), 0);
        assert!(!m.test_invoked());
        assert!(!m.build_invoked());
        assert!(!m.lint_invoked());
        assert!(!m.typecheck_invoked());
        assert_eq!(m.tool_call_count(), 0);
        assert_eq!(m.session_duration().value(), 0);
        assert!(m.edit_files().is_empty());
        assert!((m.bash_error_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bash_missing_command_key() {
        let uses = vec![
            make_tool_use("Bash", r#"{"not_command":"ls"}"#, 1000),
            make_tool_use("Bash", r#"{"command":"cargo test"}"#, 2000),
        ];
        // command 키 없는 행은 제외, cargo test만 매칭
        assert!(calc_invoked(&uses, &TEST_PATTERN));
        // not_command 행은 빈 문자열로 fallback → test 패턴 미매칭
        let uses_only_missing = vec![make_tool_use("Bash", r#"{"not_command":"ls"}"#, 1000)];
        assert!(!calc_invoked(&uses_only_missing, &TEST_PATTERN));
    }
}
