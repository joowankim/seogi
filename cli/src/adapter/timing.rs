use std::fs;
use std::path::{Path, PathBuf};

use super::error::AdapterError;
use crate::domain::value::Timestamp;

/// 타이밍 디렉토리 경로를 결정한다.
///
/// `SEOGI_TIMING_DIR` 환경변수가 설정되어 있으면 그 값을 사용하고,
/// 없으면 `${TMPDIR:-/tmp}/seogi`를 기본값으로 사용한다.
#[must_use]
pub fn timing_dir() -> PathBuf {
    if let Ok(path) = std::env::var("SEOGI_TIMING_DIR") {
        return PathBuf::from(path);
    }
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(tmp).join("seogi")
}

/// 도구 호출 시작 시간을 파일에 기록한다.
///
/// `{dir}/{tool_use_id}_start` 파일에 밀리초 Unix timestamp를 저장한다.
/// 디렉토리가 없으면 자동 생성한다.
///
/// # Errors
///
/// 디렉토리 생성 또는 파일 쓰기 실패 시 `AdapterError::Io`.
pub fn save_start_time(dir: &Path, tool_use_id: &str) -> Result<(), AdapterError> {
    fs::create_dir_all(dir)?;
    let file_path = dir.join(format!("{tool_use_id}_start"));
    let now = Timestamp::now();
    fs::write(file_path, now.value().to_string())?;
    Ok(())
}

/// 도구 호출 시작 시간을 읽고 파일을 삭제한다.
///
/// 파일이 없거나 내용이 유효하지 않으면 `None`을 반환한다.
/// 파일 삭제 실패는 무시한다 (best-effort cleanup).
#[must_use]
pub fn read_and_remove_start_time(dir: &Path, tool_use_id: &str) -> Option<Timestamp> {
    let file_path = dir.join(format!("{tool_use_id}_start"));
    let content = fs::read_to_string(&file_path).ok()?;
    let ts: i64 = content.trim().parse().ok()?;
    let _ = fs::remove_file(&file_path);
    Some(Timestamp::new(ts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_start_time_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        save_start_time(dir.path(), "toolu_01").unwrap();

        let file = dir.path().join("toolu_01_start");
        assert!(file.exists());

        let content = fs::read_to_string(&file).unwrap();
        let ts: i64 = content.trim().parse().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        assert!((now - ts).abs() < 1000);
    }

    #[test]
    fn test_read_and_remove_start_time_returns_value() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("toolu_02_start");
        fs::write(&file, "1713000000123").unwrap();

        let result = read_and_remove_start_time(dir.path(), "toolu_02");
        assert_eq!(result, Some(Timestamp::new(1_713_000_000_123)));
        assert!(!file.exists(), "file should be deleted");
    }

    #[test]
    fn test_read_and_remove_start_time_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_and_remove_start_time(dir.path(), "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_read_and_remove_start_time_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("toolu_03_start");
        fs::write(&file, "not_a_number").unwrap();

        let result = read_and_remove_start_time(dir.path(), "toolu_03");
        assert!(result.is_none());
    }

    #[test]
    fn test_save_start_time_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        save_start_time(&nested, "toolu_04").unwrap();

        let file = nested.join("toolu_04_start");
        assert!(file.exists());
    }

    // timing_dir()의 환경변수 분기는 E2E 테스트에서 커버됨
    // (SEOGI_TIMING_DIR 설정: feature_05_test, 미설정: 기본값 fallback)
}
