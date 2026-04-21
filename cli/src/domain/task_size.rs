/// 태스크 브랜치의 변경량을 나타내는 도메인 타입.
///
/// `git diff --stat` 출력에서 파싱한 추가/삭제 라인 수와 변경 파일 수.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSize {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

/// `git diff --stat` 출력의 summary line을 파싱하여 `TaskSize`를 반환한다.
///
/// summary line 형식:
/// - `" 5 files changed, 120 insertions(+), 30 deletions(-)"`
/// - `" 1 file changed, 10 insertions(+)"`  (deletions 없음)
/// - `" 3 files changed, 50 deletions(-)"` (insertions 없음)
///
/// 빈 출력이거나 summary line을 파싱할 수 없으면 `None`을 반환한다.
#[must_use]
pub fn parse_diff_stat(output: &str) -> Option<TaskSize> {
    let summary = output.lines().last()?.trim();

    if summary.is_empty() {
        return None;
    }

    let files_changed = extract_number(summary, "file")?;
    let additions = extract_number(summary, "insertion").unwrap_or(0);
    let deletions = extract_number(summary, "deletion").unwrap_or(0);

    Some(TaskSize {
        additions,
        deletions,
        files_changed,
    })
}

/// summary line에서 특정 키워드 앞의 숫자를 추출한다.
///
/// 예: `"5 files changed"` + keyword `"file"` → `Some(5)`
fn extract_number(summary: &str, keyword: &str) -> Option<u32> {
    let parts: Vec<&str> = summary.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if part.starts_with(keyword) && i > 0 {
            return parts[i - 1].parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_summary_with_insertions_and_deletions() {
        let output = " 5 files changed, 120 insertions(+), 30 deletions(-)";

        let result = parse_diff_stat(output);

        assert_eq!(
            result,
            Some(TaskSize {
                additions: 120,
                deletions: 30,
                files_changed: 5,
            })
        );
    }

    #[test]
    fn parse_summary_with_insertions_only() {
        let output = " 1 file changed, 10 insertions(+)";

        let result = parse_diff_stat(output);

        assert_eq!(
            result,
            Some(TaskSize {
                additions: 10,
                deletions: 0,
                files_changed: 1,
            })
        );
    }

    #[test]
    fn parse_summary_with_deletions_only() {
        let output = " 3 files changed, 50 deletions(-)";

        let result = parse_diff_stat(output);

        assert_eq!(
            result,
            Some(TaskSize {
                additions: 0,
                deletions: 50,
                files_changed: 3,
            })
        );
    }

    #[test]
    fn parse_multiline_stat_output_uses_last_line() {
        let output = " src/main.rs | 10 +++++++---\n src/lib.rs  |  5 +++++\n 2 files changed, 12 insertions(+), 3 deletions(-)";

        let result = parse_diff_stat(output);

        assert_eq!(
            result,
            Some(TaskSize {
                additions: 12,
                deletions: 3,
                files_changed: 2,
            })
        );
    }

    #[test]
    fn parse_empty_output_returns_none() {
        assert_eq!(parse_diff_stat(""), None);
    }

    #[test]
    fn parse_whitespace_only_returns_none() {
        assert_eq!(parse_diff_stat("   \n   "), None);
    }

    #[test]
    fn parse_invalid_summary_returns_none() {
        assert_eq!(parse_diff_stat("no matching pattern here"), None);
    }

    #[test]
    fn parse_singular_file_changed() {
        let output = " 1 file changed, 1 insertion(+), 1 deletion(-)";

        let result = parse_diff_stat(output);

        assert_eq!(
            result,
            Some(TaskSize {
                additions: 1,
                deletions: 1,
                files_changed: 1,
            })
        );
    }
}
