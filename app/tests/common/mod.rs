use std::process::{Command, Stdio};

pub fn run_seogi(args: &[&str], db_path: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(args)
        .env("SEOGI_DB_PATH", db_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}
