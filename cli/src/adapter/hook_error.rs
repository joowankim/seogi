use std::fs;
use std::path::Path;

use crate::entrypoint::hooks;

/// 훅 에러를 `~/.seogi/hook-errors.log`에 기록하고, 5분 쿨다운 후 macOS 알림을 보낸다.
pub fn handle_hook_error(error: &dyn std::fmt::Display) {
    let dir = hooks::seogi_dir();
    let _ = fs::create_dir_all(&dir);

    log_error(&dir, error);
    if update_marker_if_cooldown_elapsed(&dir) && !notification_suppressed() {
        send_notification();
    }
}

fn log_error(seogi_dir: &Path, error: &dyn std::fmt::Display) {
    let log_path = seogi_dir.join("hook-errors.log");
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let line = format!("[{timestamp}] {error}\n");
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(line.as_bytes())
        });
}

fn notification_suppressed() -> bool {
    std::env::var("SEOGI_NO_NOTIFY").is_ok()
}

const NOTIFICATION_COOLDOWN_SECS: i64 = 300;

/// 쿨다운이 경과했으면 마커를 갱신하고 `true`를 반환한다.
fn update_marker_if_cooldown_elapsed(seogi_dir: &Path) -> bool {
    let marker = seogi_dir.join("last-notification");

    let should_notify = match fs::read_to_string(&marker) {
        Ok(content) => {
            let last: i64 = content.trim().parse().unwrap_or(0);
            let now = chrono::Utc::now().timestamp();
            now - last >= NOTIFICATION_COOLDOWN_SECS
        }
        Err(_) => true,
    };

    if should_notify {
        let now = chrono::Utc::now().timestamp();
        let _ = fs::write(&marker, now.to_string());
    }

    should_notify
}

#[cfg(target_os = "macos")]
fn send_notification() {
    let _ = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"display notification "훅 에러 발생. ~/.seogi/hook-errors.log 확인 필요" with title "seogi""#,
        ])
        .output();
}

#[cfg(not(target_os = "macos"))]
fn send_notification() {
    // macOS 이외 플랫폼에서는 알림 건너뜀
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_error_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        log_error(dir.path(), &"test error");

        let log = dir.path().join("hook-errors.log");
        assert!(log.exists());
        let content = fs::read_to_string(&log).unwrap();
        assert!(content.contains("test error"));
        assert!(content.contains('T'));
    }

    #[test]
    fn test_notification_cooldown_first_time() {
        let dir = tempfile::tempdir().unwrap();
        let should_notify = update_marker_if_cooldown_elapsed(dir.path());

        assert!(should_notify);
        let marker = dir.path().join("last-notification");
        assert!(marker.exists());
    }

    #[test]
    fn test_notification_cooldown_within_5min() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("last-notification");
        let now = chrono::Utc::now().timestamp();
        fs::write(&marker, now.to_string()).unwrap();

        let should_notify = update_marker_if_cooldown_elapsed(dir.path());

        assert!(!should_notify);
    }
}
