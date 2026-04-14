use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub log_dir: String,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u32,
}

fn default_max_file_size_mb() -> u32 {
    10
}

impl Config {
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let path = config_path.map_or_else(
            || {
                let home = std::env::var("HOME").unwrap_or_default();
                PathBuf::from(home).join(".seogi").join("config.json")
            },
            PathBuf::from,
        );

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config from {}", path.display()))?;

        let config: Config = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse config from {}", path.display()))?;

        Ok(config)
    }

    /// logDir의 `~`를 홈 디렉토리로 확장한 절대경로를 반환
    #[must_use]
    pub fn log_dir_expanded(&self) -> PathBuf {
        if self.log_dir.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(self.log_dir.replacen('~', &home, 1))
        } else {
            PathBuf::from(&self.log_dir)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_valid_config() {
        let dir = std::env::temp_dir().join("seogi_test_config_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"logDir": "~/seogi-logs", "maxFileSizeMB": 10}}"#).unwrap();

        let config = Config::load(Some(&path)).unwrap();
        assert_eq!(config.log_dir, "~/seogi-logs");
        assert_eq!(config.max_file_size_mb, 10);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_config_default_max_file_size() {
        let dir = std::env::temp_dir().join("seogi_test_config_default");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"logDir": "/tmp/logs"}}"#).unwrap();

        let config = Config::load(Some(&path)).unwrap();
        assert_eq!(config.max_file_size_mb, 10);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_config_file_not_found() {
        let result = Config::load(Some(Path::new("/nonexistent/config.json")));
        assert!(result.is_err());
    }

    #[test]
    fn load_config_invalid_json() {
        let dir = std::env::temp_dir().join("seogi_test_config_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "not json").unwrap();

        let result = Config::load(Some(&path));
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn log_dir_tilde_expansion() {
        let config = Config {
            log_dir: "~/seogi-logs".to_string(),
            max_file_size_mb: 10,
        };
        let expanded = config.log_dir_expanded();
        assert!(!expanded.to_str().unwrap().starts_with('~'));
        assert!(expanded.to_str().unwrap().ends_with("/seogi-logs"));
    }

    #[test]
    fn log_dir_absolute_path() {
        let config = Config {
            log_dir: "/var/logs/seogi".to_string(),
            max_file_size_mb: 10,
        };
        let expanded = config.log_dir_expanded();
        assert_eq!(expanded, PathBuf::from("/var/logs/seogi"));
    }
}
