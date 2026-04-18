use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "seogi", version, about = "하니스 엔지니어링을 위한 계측 도구")]
struct Cli {
    /// config.json 경로 (기본값: ~/.seogi/config.json)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 세션 로그에서 메트릭을 계산하여 JSON 출력
    Analyze {
        /// 세션 ID
        session_id: String,
    },
    /// 기간별 메트릭 집계 리포트
    Report {
        /// 시작 날짜 (YYYY-MM-DD)
        #[arg(long)]
        from: String,
        /// 종료 날짜 (YYYY-MM-DD)
        #[arg(long)]
        to: String,
        /// 프로젝트 이름 (생략 시 전체)
        #[arg(long)]
        project: Option<String>,
    },
    /// 하니스 변경 이력 관리
    Changelog {
        #[command(subcommand)]
        action: ChangelogAction,
    },
    /// JSONL 로그를 `SQLite`로 마이그레이션
    Migrate,
    /// Claude Code 훅
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand)]
enum ChangelogAction {
    /// 변경 이력 추가
    Add {
        /// 변경 설명
        description: String,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// 도구 사용 성공 기록 (`PostToolUse`)
    PostTool,
    /// 도구 사용 실패 기록 (`PostToolUseFailure`)
    PostToolFailure,
    /// 알림 이벤트 기록 (`Notification`)
    Notification,
    /// 세션 종료 이벤트 기록 (`Stop`)
    Stop,
    /// 도구 호출 시작 시간 기록 (`PreToolUse`)
    PreTool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { session_id } => {
            let db_path = seogi::entrypoint::hooks::db_path();
            let conn = seogi::adapter::db::initialize_db(&db_path)
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
            let metrics = seogi::workflow::analyze::run(&conn, &session_id)
                .map_err(|e| anyhow::anyhow!("Failed to analyze session: {e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&metrics)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize metrics: {e}"))?
            );
        }
        Commands::Report { from, to, project } => {
            let db_path = seogi::entrypoint::hooks::db_path();
            let conn = seogi::adapter::db::initialize_db(&db_path)
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
            let output = seogi::workflow::report::run(&conn, &from, &to, project.as_deref())
                .map_err(|e| anyhow::anyhow!("Failed to generate report: {e}"))?;
            print!("{output}");
        }
        Commands::Changelog { action } => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            match action {
                ChangelogAction::Add { description } => {
                    seogi::commands::changelog::add(&config, &description)?;
                }
            }
        }
        Commands::Migrate => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            let log_dir = config.log_dir_expanded();
            let db_path = seogi::entrypoint::hooks::db_path();
            let conn = seogi::adapter::db::initialize_db(&db_path)
                .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
            let summary = seogi::workflow::migrate::run(&conn, &log_dir)
                .map_err(|e| anyhow::anyhow!("Failed to migrate: {e}"))?;
            println!(
                "Migrated: {} tool_uses, {} tool_failures, {} skipped, {} files",
                summary.tool_uses, summary.tool_failures, summary.skipped, summary.files
            );
        }
        Commands::Hook { action } => match action {
            HookAction::PostTool => {
                seogi::entrypoint::hooks::post_tool::run()?;
            }
            HookAction::PostToolFailure => {
                seogi::entrypoint::hooks::post_tool_failure::run()?;
            }
            HookAction::Notification => {
                seogi::entrypoint::hooks::notification::run()?;
            }
            HookAction::Stop => {
                seogi::entrypoint::hooks::stop::run()?;
            }
            HookAction::PreTool => {
                seogi::entrypoint::hooks::pre_tool::run()?;
            }
        },
    }

    Ok(())
}
