use std::path::Path;
use std::process::Command;

use super::error::AdapterError;
use crate::domain::task_size::{TaskSize, parse_diff_stat};

/// 태스크 브랜치의 변경량을 계산한다.
///
/// `git diff main...<task_id> --stat`을 실행하여 추가/삭제 라인 수와
/// 변경 파일 수를 반환한다.
///
/// 브랜치가 존재하지 않거나 이미 삭제된 경우, 또는 git 저장소가 아닌
/// 경로인 경우 `Ok(None)`을 반환한다 (graceful skip).
///
/// # Errors
///
/// git 명령 실행 자체가 실패하면 (예: git 미설치) `AdapterError::Io`.
pub fn diff_stat(repo_path: &Path, task_id: &str) -> Result<Option<TaskSize>, AdapterError> {
    let output = Command::new("git")
        .args(["diff", &format!("main...{task_id}"), "--stat"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_stat(&stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-b", "main"]);
        git(
            dir,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=t@t",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
    }

    #[test]
    fn diff_stat_returns_none_for_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = diff_stat(tmp.path(), "SEO-1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn diff_stat_returns_none_for_missing_branch() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());

        let result = diff_stat(tmp.path(), "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn diff_stat_returns_task_size_for_existing_branch() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());

        git(tmp.path(), &["checkout", "-b", "SEO-99"]);
        std::fs::write(tmp.path().join("new_file.rs"), "fn main() {}").unwrap();
        git(tmp.path(), &["add", "."]);
        git(
            tmp.path(),
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=t@t",
                "commit",
                "-m",
                "add file",
            ],
        );

        let result = diff_stat(tmp.path(), "SEO-99").unwrap();
        assert!(result.is_some());
        let size = result.unwrap();
        assert_eq!(size.files_changed, 1);
        assert!(size.additions > 0);
    }
}
