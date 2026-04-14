use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::models::ChangelogEntry;

/// `seogi changelog add <description>` — 하니스 변경 이력 기록
///
/// # Errors
///
/// Returns an error if the changelog file cannot be written.
pub fn add(config: &Config, description: &str) -> Result<()> {
    let log_dir = config.log_dir_expanded();
    let changelog_path = log_dir.join("harness-changelog.jsonl");

    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    let entry = ChangelogEntry {
        timestamp: timestamp.clone(),
        description: description.to_string(),
    };

    let line = serde_json::to_string(&entry)?;
    append_line(&changelog_path, &line)?;

    println!("Recorded at {timestamp}");
    Ok(())
}

fn append_line(path: &Path, line: &str) -> Result<()> {
    use std::io::Write;

    // 부모 디렉토리 생성
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changelog_creates_file() {
        let dir = std::env::temp_dir().join("seogi_test_changelog_create");
        std::fs::create_dir_all(&dir).unwrap();

        let config = Config {
            log_dir: dir.to_str().unwrap().to_string(),
            max_file_size_mb: 10,
        };

        add(&config, "test change").unwrap();

        let path = dir.join("harness-changelog.jsonl");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let entry: ChangelogEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry.description, "test change");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changelog_appends_to_existing() {
        let dir = std::env::temp_dir().join("seogi_test_changelog_append");
        std::fs::create_dir_all(&dir).unwrap();

        let config = Config {
            log_dir: dir.to_str().unwrap().to_string(),
            max_file_size_mb: 10,
        };

        add(&config, "first change").unwrap();
        add(&config, "second change").unwrap();

        let path = dir.join("harness-changelog.jsonl");
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 2);

        let first: ChangelogEntry = serde_json::from_str(lines[0]).unwrap();
        let second: ChangelogEntry = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first.description, "first change");
        assert_eq!(second.description, "second change");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changelog_timestamp_format() {
        let dir = std::env::temp_dir().join("seogi_test_changelog_ts");
        std::fs::create_dir_all(&dir).unwrap();

        let config = Config {
            log_dir: dir.to_str().unwrap().to_string(),
            max_file_size_mb: 10,
        };

        add(&config, "check timestamp").unwrap();

        let path = dir.join("harness-changelog.jsonl");
        let content = std::fs::read_to_string(&path).unwrap();
        let entry: ChangelogEntry = serde_json::from_str(content.trim()).unwrap();
        // ISO 8601 형식 검증
        assert!(entry.timestamp.ends_with('Z'));
        assert!(entry.timestamp.contains('T'));

        std::fs::remove_dir_all(&dir).ok();
    }
}
