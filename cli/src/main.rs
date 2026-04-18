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
    /// 세션 로그에서 메트릭을 계산하여 저장
    Analyze {
        /// 프로젝트 이름
        project: String,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            project,
            session_id,
        } => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            seogi::commands::analyze::run(&config, &project, &session_id)?;
        }
        Commands::Report { from, to, project } => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            seogi::commands::report::run(&config, &from, &to, project.as_deref())?;
        }
        Commands::Changelog { action } => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            match action {
                ChangelogAction::Add { description } => {
                    seogi::commands::changelog::add(&config, &description)?;
                }
            }
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
        },
    }

    Ok(())
}
