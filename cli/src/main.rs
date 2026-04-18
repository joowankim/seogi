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
    /// 프로젝트 관리
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Claude Code 훅
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// 프로젝트 생성
    Create {
        /// 프로젝트 이름
        #[arg(long)]
        name: String,
        /// 대문자 알파벳 3글자 (미지정 시 이름 앞 3글자 대문자)
        #[arg(long)]
        prefix: Option<String>,
        /// 프로젝트 목표
        #[arg(long)]
        goal: String,
    },
    /// 프로젝트 목록 조회
    List {
        /// JSON 형식으로 출력
        #[arg(long)]
        json: bool,
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
        Commands::Changelog { action } => match action {
            ChangelogAction::Add { description } => {
                let db_path = seogi::entrypoint::hooks::db_path();
                let conn = seogi::adapter::db::initialize_db(&db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
                let ts = seogi::workflow::changelog::run(&conn, &description)
                    .map_err(|e| anyhow::anyhow!("Failed to save changelog: {e}"))?;
                println!("Recorded at {ts}");
            }
        },
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
        Commands::Project { action } => match action {
            ProjectAction::Create { name, prefix, goal } => {
                let db_path = seogi::entrypoint::hooks::db_path();
                let conn = seogi::adapter::db::initialize_db(&db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
                let project =
                    seogi::workflow::project::create(&conn, &name, prefix.as_deref(), &goal)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                println!(
                    "Created project \"{}\" ({})",
                    project.name(),
                    project.prefix()
                );
            }
            ProjectAction::List { json } => {
                let db_path = seogi::entrypoint::hooks::db_path();
                let conn = seogi::adapter::db::initialize_db(&db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;
                let projects =
                    seogi::workflow::project::list(&conn).map_err(|e| anyhow::anyhow!("{e}"))?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&projects)
                            .map_err(|e| anyhow::anyhow!("Failed to serialize: {e}"))?
                    );
                } else {
                    println!("{:<8} {:<20} GOAL", "PREFIX", "NAME");
                    for p in &projects {
                        println!("{:<8} {:<20} {}", p.prefix(), p.name(), p.goal());
                    }
                }
            }
        },
        Commands::Hook { action } => {
            use seogi::entrypoint::hooks::{
                notification, post_tool, post_tool_failure, pre_tool, run_safely, stop,
            };
            match action {
                HookAction::PostTool => run_safely(post_tool::run),
                HookAction::PostToolFailure => run_safely(post_tool_failure::run),
                HookAction::Notification => run_safely(notification::run),
                HookAction::Stop => run_safely(stop::run),
                HookAction::PreTool => run_safely(pre_tool::run),
            }
        }
    }

    Ok(())
}
