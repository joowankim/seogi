use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rusqlite::Connection;

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
    /// 상태 관리
    Status {
        #[command(subcommand)]
        action: StatusAction,
    },
    /// 태스크 관리
    Task {
        #[command(subcommand)]
        action: TaskAction,
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
enum StatusAction {
    /// 상태 생성
    Create {
        /// 카테고리 (backlog, unstarted, started, completed, canceled)
        #[arg(long)]
        category: String,
        /// 상태 이름
        #[arg(long)]
        name: String,
    },
    /// 상태 목록 조회
    List {
        /// JSON 형식으로 출력
        #[arg(long)]
        json: bool,
    },
    /// 상태 이름 수정
    Update {
        /// 수정할 Status ID
        id: String,
        /// 변경할 이름
        #[arg(long)]
        name: String,
    },
    /// 상태 삭제
    Delete {
        /// 삭제할 Status ID
        id: String,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// 태스크 생성
    Create {
        /// 프로젝트 이름
        #[arg(long)]
        project: String,
        /// 태스크 제목
        #[arg(long)]
        title: String,
        /// 태스크 설명
        #[arg(long)]
        description: String,
        /// 라벨 (feature, bug, refactor, chore, docs)
        #[arg(long)]
        label: String,
    },
    /// 태스크 목록 조회
    List {
        /// 프로젝트 이름 필터
        #[arg(long)]
        project: Option<String>,
        /// 상태 이름 필터
        #[arg(long)]
        status: Option<String>,
        /// 라벨 필터
        #[arg(long)]
        label: Option<String>,
        /// JSON 형식으로 출력
        #[arg(long)]
        json: bool,
    },
    /// 태스크 수정
    Update {
        /// 태스크 ID (e.g., SEO-1)
        task_id: String,
        /// 변경할 제목
        #[arg(long)]
        title: Option<String>,
        /// 변경할 설명
        #[arg(long)]
        description: Option<String>,
        /// 변경할 라벨
        #[arg(long)]
        label: Option<String>,
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

fn open_db() -> Result<Connection> {
    let db_path = seogi::entrypoint::hooks::db_path();
    seogi::adapter::db::initialize_db(&db_path)
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { session_id } => {
            let conn = open_db()?;
            let metrics = seogi::workflow::analyze::run(&conn, &session_id)
                .map_err(|e| anyhow::anyhow!("Failed to analyze session: {e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&metrics)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize metrics: {e}"))?
            );
        }
        Commands::Report { from, to, project } => {
            let conn = open_db()?;
            let output = seogi::workflow::report::run(&conn, &from, &to, project.as_deref())
                .map_err(|e| anyhow::anyhow!("Failed to generate report: {e}"))?;
            print!("{output}");
        }
        Commands::Changelog { action } => match action {
            ChangelogAction::Add { description } => {
                let conn = open_db()?;
                let ts = seogi::workflow::changelog::run(&conn, &description)
                    .map_err(|e| anyhow::anyhow!("Failed to save changelog: {e}"))?;
                println!("Recorded at {ts}");
            }
        },
        Commands::Migrate => {
            let config = seogi::config::Config::load(cli.config.as_deref())?;
            let log_dir = config.log_dir_expanded();
            let conn = open_db()?;
            let summary = seogi::workflow::migrate::run(&conn, &log_dir)
                .map_err(|e| anyhow::anyhow!("Failed to migrate: {e}"))?;
            println!(
                "Migrated: {} tool_uses, {} tool_failures, {} skipped, {} files",
                summary.tool_uses, summary.tool_failures, summary.skipped, summary.files
            );
        }
        Commands::Project { action } => {
            let conn = open_db()?;
            match action {
                ProjectAction::Create { name, prefix, goal } => {
                    seogi::entrypoint::project::create(&conn, &name, prefix.as_deref(), &goal)?;
                }
                ProjectAction::List { json } => {
                    seogi::entrypoint::project::list(&conn, json)?;
                }
            }
        }
        Commands::Status { action } => {
            let conn = open_db()?;
            match action {
                StatusAction::Create { category, name } => {
                    seogi::entrypoint::status::create(&conn, &category, &name)?;
                }
                StatusAction::List { json } => {
                    seogi::entrypoint::status::list(&conn, json)?;
                }
                StatusAction::Update { id, name } => {
                    seogi::entrypoint::status::update(&conn, &id, &name)?;
                }
                StatusAction::Delete { id } => {
                    seogi::entrypoint::status::delete(&conn, &id)?;
                }
            }
        }
        Commands::Task { action } => {
            let conn = open_db()?;
            match action {
                TaskAction::Create {
                    project,
                    title,
                    description,
                    label,
                } => {
                    seogi::entrypoint::task::create(&conn, &project, &title, &description, &label)?;
                }
                TaskAction::List {
                    project,
                    status,
                    label,
                    json,
                } => {
                    seogi::entrypoint::task::list(
                        &conn,
                        project.as_deref(),
                        status.as_deref(),
                        label.as_deref(),
                        json,
                    )?;
                }
                TaskAction::Update {
                    task_id,
                    title,
                    description,
                    label,
                } => {
                    seogi::entrypoint::task::update(
                        &conn,
                        &task_id,
                        title.as_deref(),
                        description.as_deref(),
                        label.as_deref(),
                    )?;
                }
            }
        }
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
