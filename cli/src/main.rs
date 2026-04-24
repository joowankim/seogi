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
    /// 하니스 변경 이력 관리
    Changelog {
        #[command(subcommand)]
        action: ChangelogAction,
    },
    /// JSONL 로그를 `SQLite`로 마이그레이션
    Migrate,
    /// 워크스페이스 관리
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// 상태 관리
    Status {
        #[command(subcommand)]
        action: StatusAction,
    },
    /// 사이클 관리
    Cycle {
        #[command(subcommand)]
        action: CycleAction,
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
    /// 태스크 기반 성과 리포트
    Report {
        /// 시작 날짜 (YYYY-MM-DD)
        #[arg(long)]
        from: String,
        /// 종료 날짜 (YYYY-MM-DD)
        #[arg(long)]
        to: String,
        /// 워크스페이스 이름 필터
        #[arg(long)]
        workspace: Option<String>,
        /// 상세 출력
        #[arg(long)]
        detail: bool,
    },
    /// MCP 서버 (stdio transport)
    McpServer,
}

#[derive(Subcommand)]
enum WorkspaceAction {
    /// 워크스페이스 생성
    Create {
        /// 워크스페이스 이름
        #[arg(long)]
        name: String,
        /// 대문자 알파벳 3글자 (미지정 시 이름 앞 3글자 대문자)
        #[arg(long)]
        prefix: Option<String>,
        /// 워크스페이스 목표
        #[arg(long)]
        goal: String,
    },
    /// 워크스페이스 목록 조회
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
        /// 워크스페이스 이름
        #[arg(long)]
        workspace: String,
        /// 태스크 제목
        #[arg(long)]
        title: String,
        /// 태스크 설명
        #[arg(long)]
        description: String,
        /// 라벨 (feature, bug, refactor, chore, docs)
        #[arg(long)]
        label: String,
        /// 의존 대상 태스크 ID
        #[arg(long)]
        depends_on: Option<String>,
    },
    /// 태스크 목록 조회
    List {
        /// 워크스페이스 이름 필터
        #[arg(long)]
        workspace: Option<String>,
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
    /// 태스크 단일 조회
    Get {
        /// 태스크 ID (e.g., SEO-1)
        task_id: String,
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
    /// 태스크 의존 관계 추가
    Depend {
        /// 태스크 ID (e.g., SEO-2)
        task_id: String,
        /// 의존 대상 태스크 ID (e.g., SEO-1)
        #[arg(long)]
        on: String,
    },
    /// 태스크 의존 관계 제거
    Undepend {
        /// 태스크 ID (e.g., SEO-2)
        task_id: String,
        /// 제거할 의존 대상 태스크 ID (e.g., SEO-1)
        #[arg(long)]
        on: String,
    },
    /// 태스크 상태 전환
    Move {
        /// 태스크 ID (e.g., SEO-1)
        task_id: String,
        /// 이동할 상태 이름 (e.g., `in_progress`, `done`)
        status: String,
    },
}

#[derive(Subcommand)]
enum CycleAction {
    /// 사이클 생성
    Create {
        /// 워크스페이스 이름
        #[arg(long)]
        workspace: String,
        /// 사이클 이름
        #[arg(long)]
        name: String,
        /// 시작일 (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// 종료일 (YYYY-MM-DD)
        #[arg(long)]
        end: String,
    },
    /// 사이클 목록 조회
    List {
        /// 워크스페이스 이름 필터
        #[arg(long)]
        workspace: Option<String>,
        /// JSON 형식으로 출력
        #[arg(long)]
        json: bool,
    },
    /// 사이클 수정
    Update {
        /// 사이클 ID
        cycle_id: String,
        /// 변경할 이름
        #[arg(long)]
        name: Option<String>,
        /// 변경할 시작일
        #[arg(long)]
        start: Option<String>,
        /// 변경할 종료일
        #[arg(long)]
        end: Option<String>,
    },
    /// 사이클에 태스크 배정
    Assign {
        /// 사이클 ID
        cycle_id: String,
        /// 태스크 ID
        task_id: String,
    },
    /// 사이클에서 태스크 배정 해제
    Unassign {
        /// 사이클 ID
        cycle_id: String,
        /// 태스크 ID
        task_id: String,
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
        Commands::Workspace { action } => {
            let conn = open_db()?;
            match action {
                WorkspaceAction::Create { name, prefix, goal } => {
                    seogi::entrypoint::workspace::create(&conn, &name, prefix.as_deref(), &goal)?;
                }
                WorkspaceAction::List { json } => {
                    seogi::entrypoint::workspace::list(&conn, json)?;
                }
            }
        }
        Commands::Cycle { action } => {
            let conn = open_db()?;
            match action {
                CycleAction::Create {
                    workspace,
                    name,
                    start,
                    end,
                } => {
                    seogi::entrypoint::cycle::create(&conn, &workspace, &name, &start, &end)?;
                }
                CycleAction::List { workspace, json } => {
                    seogi::entrypoint::cycle::list(&conn, workspace.as_deref(), json)?;
                }
                CycleAction::Update {
                    cycle_id,
                    name,
                    start,
                    end,
                } => {
                    seogi::entrypoint::cycle::update(
                        &conn,
                        &cycle_id,
                        name.as_deref(),
                        start.as_deref(),
                        end.as_deref(),
                    )?;
                }
                CycleAction::Assign { cycle_id, task_id } => {
                    seogi::entrypoint::cycle::assign(&conn, &cycle_id, &task_id)?;
                }
                CycleAction::Unassign { cycle_id, task_id } => {
                    seogi::entrypoint::cycle::unassign(&conn, &cycle_id, &task_id)?;
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
                    workspace,
                    title,
                    description,
                    label,
                    depends_on,
                } => {
                    seogi::entrypoint::task::create(
                        &conn,
                        &workspace,
                        &title,
                        &description,
                        &label,
                        depends_on.as_deref(),
                    )?;
                }
                TaskAction::List {
                    workspace,
                    status,
                    label,
                    json,
                } => {
                    seogi::entrypoint::task::list(
                        &conn,
                        workspace.as_deref(),
                        status.as_deref(),
                        label.as_deref(),
                        json,
                    )?;
                }
                TaskAction::Get { task_id, json } => {
                    seogi::entrypoint::task::get(&conn, &task_id, json)?;
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
                TaskAction::Depend { task_id, on } => {
                    seogi::entrypoint::task::depend(&conn, &task_id, &on)?;
                }
                TaskAction::Undepend { task_id, on } => {
                    seogi::entrypoint::task::undepend(&conn, &task_id, &on)?;
                }
                TaskAction::Move { task_id, status } => {
                    seogi::entrypoint::task::move_task(&conn, &task_id, &status)?;
                }
            }
        }
        Commands::Report {
            from,
            to,
            workspace,
            detail,
        } => {
            let conn = open_db()?;
            let output =
                seogi::workflow::report::run(&conn, &from, &to, workspace.as_deref(), detail)
                    .map_err(|e| anyhow::anyhow!("Failed to generate report: {e}"))?;
            print!("{output}");
        }
        Commands::McpServer => {
            seogi::entrypoint::mcp::run()?;
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
