use std::sync::{Arc, Mutex};

use anyhow::Result;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServiceExt, tool, tool_handler, tool_router};
use rusqlite::Connection;
use serde::Deserialize;

use crate::adapter::db;
use crate::entrypoint::hooks::db_path;
use crate::workflow;

#[derive(Clone)]
struct SeogiMcpServer {
    conn: Arc<Mutex<Connection>>,
}

// ── Parameter types ──

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WorkspaceCreateParams {
    name: String,
    #[serde(default)]
    prefix: Option<String>,
    goal: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StatusCreateParams {
    category: String,
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StatusUpdateParams {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StatusDeleteParams {
    id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskCreateParams {
    workspace: String,
    title: String,
    description: String,
    label: String,
    #[serde(default)]
    depends_on: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskListParams {
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskGetParams {
    task_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskUpdateParams {
    task_id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskMoveParams {
    task_id: String,
    status: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskDependParams {
    task_id: String,
    depends_on: String,
}

// ── Tool implementations ──

fn success_text(text: String) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn error_text(text: String) -> CallToolResult {
    CallToolResult::error(vec![Content::text(text)])
}

#[tool_router]
impl SeogiMcpServer {
    #[tool(name = "workspace_create", description = "Create a new workspace")]
    async fn workspace_create(
        &self,
        Parameters(params): Parameters<WorkspaceCreateParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::workspace::create(
                &conn,
                &params.name,
                params.prefix.as_deref(),
                &params.goal,
            ) {
                Ok(workspace) => {
                    let json = serde_json::json!({
                        "name": workspace.name(),
                        "prefix": workspace.prefix().as_str(),
                        "goal": workspace.goal(),
                    });
                    success_text(
                        serde_json::to_string_pretty(&json)
                            .expect("JSON Value serialization is infallible"),
                    )
                }
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "workspace_list", description = "List all workspaces")]
    async fn workspace_list(&self) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::workspace::list(&conn) {
                Ok(workspaces) => {
                    let json: Vec<serde_json::Value> = workspaces
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "name": p.name(),
                                "prefix": p.prefix().as_str(),
                                "goal": p.goal(),
                            })
                        })
                        .collect();
                    success_text(
                        serde_json::to_string_pretty(&json)
                            .expect("JSON Value serialization is infallible"),
                    )
                }
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "status_create", description = "Create a new status")]
    async fn status_create(
        &self,
        Parameters(params): Parameters<StatusCreateParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::status::create(&conn, &params.category, &params.name) {
                Ok(status) => {
                    let json = serde_json::json!({
                        "id": status.id(),
                        "name": status.name(),
                        "category": status.category().as_str(),
                        "position": status.position(),
                    });
                    success_text(
                        serde_json::to_string_pretty(&json)
                            .expect("JSON Value serialization is infallible"),
                    )
                }
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "status_list", description = "List all statuses")]
    async fn status_list(&self) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::status::list(&conn) {
                Ok(statuses) => {
                    let json: Vec<serde_json::Value> = statuses
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "id": s.id(),
                                "name": s.name(),
                                "category": s.category().as_str(),
                                "position": s.position(),
                            })
                        })
                        .collect();
                    success_text(
                        serde_json::to_string_pretty(&json)
                            .expect("JSON Value serialization is infallible"),
                    )
                }
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "status_update", description = "Update a status name")]
    async fn status_update(
        &self,
        Parameters(params): Parameters<StatusUpdateParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::status::update(&conn, &params.id, &params.name) {
                Ok(()) => success_text(format!("Updated status {}", params.id)),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "status_delete", description = "Delete a status")]
    async fn status_delete(
        &self,
        Parameters(params): Parameters<StatusDeleteParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::status::delete(&conn, &params.id) {
                Ok(()) => success_text(format!("Deleted status {}", params.id)),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_create", description = "Create a new task")]
    async fn task_create(
        &self,
        Parameters(params): Parameters<TaskCreateParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            let task = match workflow::task::create(
                &conn,
                &params.workspace,
                &params.title,
                &params.description,
                &params.label,
            ) {
                Ok(t) => t,
                Err(e) => return error_text(format!("{e}")),
            };
            if let Some(dep) = &params.depends_on
                && let Err(e) = workflow::task::depend(&conn, task.id(), dep)
            {
                return error_text(format!("{e}"));
            }
            let json = serde_json::json!({
                "id": task.id(),
                "title": task.title(),
                "description": task.description(),
                "label": task.label().as_str(),
            });
            success_text(
                serde_json::to_string_pretty(&json)
                    .expect("JSON Value serialization is infallible"),
            )
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_list", description = "List tasks with optional filters")]
    async fn task_list(&self, Parameters(params): Parameters<TaskListParams>) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::task::list(
                &conn,
                params.workspace.as_deref(),
                params.status.as_deref(),
                params.label.as_deref(),
            ) {
                Ok(rows) => success_text(
                    serde_json::to_string_pretty(&rows)
                        .expect("TaskListRow serialization is infallible"),
                ),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_get", description = "Get a single task by ID")]
    async fn task_get(&self, Parameters(params): Parameters<TaskGetParams>) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            let row = match workflow::task::get(&conn, &params.task_id) {
                Ok(r) => r,
                Err(e) => return error_text(format!("{e}")),
            };
            let deps = match workflow::task::list_dependencies(&conn, &params.task_id) {
                Ok(d) => d,
                Err(e) => return error_text(format!("{e}")),
            };
            let mut value =
                serde_json::to_value(&row).expect("TaskListRow serialization is infallible");
            value["depends_on"] = serde_json::json!(deps);
            success_text(
                serde_json::to_string_pretty(&value)
                    .expect("JSON Value serialization is infallible"),
            )
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_update", description = "Update a task")]
    async fn task_update(
        &self,
        Parameters(params): Parameters<TaskUpdateParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::task::update(
                &conn,
                &params.task_id,
                params.title.as_deref(),
                params.description.as_deref(),
                params.label.as_deref(),
            ) {
                Ok(()) => success_text(format!("Updated task {}", params.task_id)),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_move", description = "Move a task to a different status")]
    async fn task_move(&self, Parameters(params): Parameters<TaskMoveParams>) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::task::move_task(&conn, &params.task_id, &params.status) {
                Ok((from, to)) => {
                    success_text(format!("Moved task {}: {from} → {to}", params.task_id))
                }
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(name = "task_depend", description = "Add a dependency between tasks")]
    async fn task_depend(
        &self,
        Parameters(params): Parameters<TaskDependParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::task::depend(&conn, &params.task_id, &params.depends_on) {
                Ok(()) => success_text(format!(
                    "Added dependency: {} depends on {}",
                    params.task_id, params.depends_on
                )),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }

    #[tool(
        name = "task_undepend",
        description = "Remove a dependency between tasks"
    )]
    async fn task_undepend(
        &self,
        Parameters(params): Parameters<TaskDependParams>,
    ) -> CallToolResult {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("db lock poisoned");
            match workflow::task::undepend(&conn, &params.task_id, &params.depends_on) {
                Ok(()) => success_text(format!(
                    "Removed dependency: {} no longer depends on {}",
                    params.task_id, params.depends_on
                )),
                Err(e) => error_text(format!("{e}")),
            }
        })
        .await
        .expect("spawn_blocking panicked")
    }
}

#[tool_handler]
impl ServerHandler for SeogiMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("seogi", env!("CARGO_PKG_VERSION")))
    }
}

/// MCP 서버를 stdio transport로 구동한다.
///
/// # Errors
///
/// tokio 런타임 초기화 실패, DB 초기화 실패, MCP 핸드셰이크 실패 시 `anyhow::Error`.
pub fn run() -> Result<()> {
    let path = db_path();
    let conn = db::initialize_db(&path)
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {e}"))?;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = SeogiMcpServer {
                conn: Arc::new(Mutex::new(conn)),
            };
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
            Ok(())
        })
}
