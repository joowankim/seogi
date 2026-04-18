use std::path::PathBuf;

pub mod notification;
pub mod post_tool;
pub mod post_tool_failure;
pub mod pre_tool;
pub mod stop;

/// DB 경로를 결정한다.
///
/// `SEOGI_DB_PATH` 환경변수가 설정되어 있으면 그 값을 사용하고,
/// 없으면 `$HOME/.seogi/seogi.db`를 기본값으로 사용한다.
pub(crate) fn db_path() -> PathBuf {
    if let Ok(path) = std::env::var("SEOGI_DB_PATH") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".seogi").join("seogi.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_path_uses_env_var_when_set() {
        unsafe {
            std::env::set_var("SEOGI_DB_PATH", "/tmp/test.db");
        }
        let path = db_path();
        unsafe {
            std::env::remove_var("SEOGI_DB_PATH");
        }

        assert_eq!(path, PathBuf::from("/tmp/test.db"));
    }

    #[test]
    fn db_path_falls_back_to_home() {
        // db_path_uses_env_var_when_set 에서 이미 SEOGI_DB_PATH를 remove 했으므로
        // 여기서는 미설정 상태에서 호출
        unsafe {
            std::env::remove_var("SEOGI_DB_PATH");
        }

        let path = db_path();

        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(path, PathBuf::from(home).join(".seogi").join("seogi.db"));
    }
}
