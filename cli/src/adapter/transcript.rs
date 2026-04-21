use std::io::BufRead;
use std::path::PathBuf;

use crate::adapter::error::AdapterError;
use crate::domain::token_usage::TokenUsage;

/// Parse JSONL transcript lines and sum token usage from assistant messages.
///
/// Each line is a JSON object. Only lines with `type: "assistant"` and a
/// `message.usage` field contribute to the total. Malformed lines and
/// non-assistant records are silently skipped.
///
/// # Errors
///
/// Returns `AdapterError::Io` if reading from the underlying reader fails.
pub fn parse_token_usage(reader: impl BufRead) -> Result<TokenUsage, AdapterError> {
    let mut total = TokenUsage::zero();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };

        if value.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
            continue;
        }

        let Some(usage) = value.get("message").and_then(|m| m.get("usage")) else {
            continue;
        };

        let input_tokens = usage
            .get("input_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let output_tokens = usage
            .get("output_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let cache_creation_input_tokens = usage
            .get("cache_creation_input_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let cache_read_input_tokens = usage
            .get("cache_read_input_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        total = total
            + TokenUsage {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            };
    }

    Ok(total)
}

/// Derive the transcript file path from a project path and session ID.
///
/// The project hash replaces path separators (`/`) with hyphens and strips
/// the leading hyphen. For example:
/// `/Users/kim/projects/seogi` becomes `Users-kim-projects-seogi`.
///
/// Returns `~/.claude/projects/<project-hash>/<session_id>.jsonl`.
#[must_use]
pub fn transcript_path(project_path: &str, session_id: &str) -> PathBuf {
    let project_hash = project_path.replace('/', "-");
    let project_hash = project_hash.trim_start_matches('-');

    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
    PathBuf::from(home)
        .join(".claude")
        .join("projects")
        .join(project_hash)
        .join(format!("{session_id}.jsonl"))
}

/// Read token usage from a transcript file.
///
/// Returns `TokenUsage::zero()` if the file does not exist (graceful skip).
///
/// # Errors
///
/// Returns `AdapterError::Io` if the file exists but cannot be read.
pub fn read_token_usage(
    project_path: &str,
    session_id: &str,
) -> Result<TokenUsage, AdapterError> {
    let path = transcript_path(project_path, session_id);

    if !path.exists() {
        return Ok(TokenUsage::zero());
    }

    let file = std::fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    parse_token_usage(reader)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn make_reader(content: &str) -> Cursor<Vec<u8>> {
        Cursor::new(content.as_bytes().to_vec())
    }

    #[test]
    fn parse_single_assistant_record() {
        let jsonl = r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":20}}}"#;

        let result = parse_token_usage(make_reader(jsonl)).unwrap();

        assert_eq!(
            result,
            TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: 10,
                cache_read_input_tokens: 20,
            }
        );
    }

    #[test]
    fn parse_sums_multiple_assistant_records() {
        let jsonl = concat!(
            r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":20}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"usage":{"input_tokens":200,"output_tokens":100,"cache_creation_input_tokens":30,"cache_read_input_tokens":40}}}"#,
        );

        let result = parse_token_usage(make_reader(jsonl)).unwrap();

        assert_eq!(
            result,
            TokenUsage {
                input_tokens: 300,
                output_tokens: 150,
                cache_creation_input_tokens: 40,
                cache_read_input_tokens: 60,
            }
        );
    }

    #[test]
    fn parse_ignores_non_assistant_records() {
        let jsonl = concat!(
            r#"{"type":"user","message":{"content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#,
            "\n",
            r#"{"type":"tool_result","content":"ok"}"#,
        );

        let result = parse_token_usage(make_reader(jsonl)).unwrap();

        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.output_tokens, 50);
    }

    #[test]
    fn parse_ignores_assistant_without_usage() {
        let jsonl = r#"{"type":"assistant","message":{"content":"hello"}}"#;

        let result = parse_token_usage(make_reader(jsonl)).unwrap();

        assert_eq!(result, TokenUsage::zero());
    }

    #[test]
    fn parse_empty_input_returns_zero() {
        let result = parse_token_usage(make_reader("")).unwrap();

        assert_eq!(result, TokenUsage::zero());
    }

    #[test]
    fn parse_skips_malformed_json_lines() {
        let jsonl = concat!(
            "not valid json\n",
            r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#,
            "\n",
            "{broken\n",
        );

        let result = parse_token_usage(make_reader(jsonl)).unwrap();

        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.output_tokens, 50);
    }

    #[test]
    fn transcript_path_converts_separators() {
        let path = transcript_path("/Users/kim/projects/seogi", "abc-123");

        let path_str = path.to_string_lossy();
        assert!(path_str.contains("Users-kim-projects-seogi"));
        assert!(path_str.ends_with("abc-123.jsonl"));
        assert!(path_str.contains(".claude/projects/"));
    }

    #[test]
    fn read_token_usage_returns_zero_for_missing_file() {
        let result = read_token_usage("/nonexistent/path", "no-session").unwrap();

        assert_eq!(result, TokenUsage::zero());
    }

    #[test]
    fn read_token_usage_reads_actual_file() {
        let dir = tempfile::tempdir().unwrap();
        let projects_dir = dir.path().join(".claude").join("projects");

        // project path: /test/project → hash: test-project
        let project_hash = "test-project";
        let session_dir = projects_dir.join(project_hash);
        std::fs::create_dir_all(&session_dir).unwrap();

        let jsonl_path = session_dir.join("sess-1.jsonl");
        std::fs::write(
            &jsonl_path,
            r#"{"type":"assistant","message":{"usage":{"input_tokens":500,"output_tokens":200,"cache_creation_input_tokens":50,"cache_read_input_tokens":100}}}"#,
        )
        .unwrap();

        // Override HOME to use temp dir
        let original_home = std::env::var("HOME").unwrap();
        // SAFETY: This test runs single-threaded and restores the original value.
        unsafe { std::env::set_var("HOME", dir.path().to_str().unwrap()) };

        let result = read_token_usage("/test/project", "sess-1").unwrap();

        // SAFETY: Restoring original HOME value.
        unsafe { std::env::set_var("HOME", &original_home) };

        assert_eq!(
            result,
            TokenUsage {
                input_tokens: 500,
                output_tokens: 200,
                cache_creation_input_tokens: 50,
                cache_read_input_tokens: 100,
            }
        );
    }
}
