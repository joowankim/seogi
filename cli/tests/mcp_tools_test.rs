use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn send_jsonrpc(stdin: &mut impl Write, msg: &serde_json::Value) {
    let line = serde_json::to_string(msg).unwrap();
    writeln!(stdin, "{line}").unwrap();
    stdin.flush().unwrap();
}

fn recv_jsonrpc(reader: &mut BufReader<impl std::io::Read>) -> serde_json::Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(line.trim()).unwrap()
}

fn spawn_mcp_server(db_path: &std::path::Path) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["mcp-server"])
        .env("SEOGI_DB_PATH", db_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn initialize(
    stdin: &mut impl Write,
    reader: &mut BufReader<impl std::io::Read>,
) -> serde_json::Value {
    send_jsonrpc(
        stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }),
    );
    let response = recv_jsonrpc(reader);

    send_jsonrpc(
        stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    );

    response
}

#[allow(clippy::needless_pass_by_value)]
fn call_tool(
    stdin: &mut impl Write,
    reader: &mut BufReader<impl std::io::Read>,
    id: u64,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    send_jsonrpc(
        stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        }),
    );
    recv_jsonrpc(reader)
}

fn list_tools(
    stdin: &mut impl Write,
    reader: &mut BufReader<impl std::io::Read>,
    id: u64,
) -> serde_json::Value {
    send_jsonrpc(
        stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/list",
            "params": {}
        }),
    );
    recv_jsonrpc(reader)
}

fn extract_text(response: &serde_json::Value) -> &str {
    response["result"]["content"][0]["text"].as_str().unwrap()
}

fn is_error(response: &serde_json::Value) -> bool {
    response["result"]["isError"].as_bool().unwrap_or(false)
}

struct McpSession {
    child: std::process::Child,
    stdin: Box<dyn Write>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpSession {
    fn new(db_path: &std::path::Path) -> Self {
        let mut child = spawn_mcp_server(db_path);
        let stdin = Box::new(child.stdin.take().unwrap());
        let reader = BufReader::new(child.stdout.take().unwrap());
        let mut session = Self {
            child,
            stdin,
            reader,
            next_id: 1,
        };
        initialize(&mut session.stdin, &mut session.reader);
        session.next_id = 2;
        session
    }

    #[allow(clippy::needless_pass_by_value)]
    fn call(&mut self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        call_tool(&mut self.stdin, &mut self.reader, id, name, arguments)
    }

    fn list_tools(&mut self) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        list_tools(&mut self.stdin, &mut self.reader, id)
    }

    fn shutdown(mut self) {
        drop(self.stdin);
        let status = self.child.wait().unwrap();
        assert!(status.success());
    }
}

// ── QA 1: tools/list 요청에 10개 도구가 응답된다 ──

#[test]
fn tools_list_returns_ten_tools() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 13);

    session.shutdown();
}

// ── QA 2: 각 도구의 inputSchema에서 필수 파라미터가 required에 포함 ──

#[test]
fn tools_list_schema_has_correct_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.list_tools();
    let tools = response["result"]["tools"].as_array().unwrap();

    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let required = tool["inputSchema"]["required"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

        match name {
            "project_create" => {
                assert!(required_strs.contains(&"name"));
                assert!(required_strs.contains(&"goal"));
                assert!(!required_strs.contains(&"prefix"));
            }
            "project_list" | "status_list" | "task_list" => {
                assert!(required.is_empty());
            }
            "status_create" => {
                assert!(required_strs.contains(&"category"));
                assert!(required_strs.contains(&"name"));
            }
            "status_update" => {
                assert!(required_strs.contains(&"id"));
                assert!(required_strs.contains(&"name"));
            }
            "status_delete" => {
                assert!(required_strs.contains(&"id"));
            }
            "task_create" => {
                assert!(required_strs.contains(&"project"));
                assert!(required_strs.contains(&"title"));
                assert!(required_strs.contains(&"description"));
                assert!(required_strs.contains(&"label"));
            }
            "task_get" => {
                assert!(required_strs.contains(&"task_id"));
            }
            "task_update" => {
                assert!(required_strs.contains(&"task_id"));
                assert!(!required_strs.contains(&"title"));
                assert!(!required_strs.contains(&"description"));
                assert!(!required_strs.contains(&"label"));
            }
            "task_move" => {
                assert!(required_strs.contains(&"task_id"));
                assert!(required_strs.contains(&"status"));
            }
            "task_depend" | "task_undepend" => {
                assert!(required_strs.contains(&"task_id"));
                assert!(required_strs.contains(&"depends_on"));
            }
            _ => panic!("unexpected tool: {name}"),
        }
    }

    session.shutdown();
}

// ── QA 3: project_create 성공 ──

#[test]
fn project_create_returns_created_project() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "project_create",
        serde_json::json!({"name": "Seogi", "prefix": "SEO", "goal": "harness measurement"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["name"], "Seogi");
    assert_eq!(data["prefix"], "SEO");
    assert_eq!(data["goal"], "harness measurement");

    session.shutdown();
}

// ── QA 4: project_create prefix 자동 생성 ──

#[test]
fn project_create_auto_generates_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "project_create",
        serde_json::json!({"name": "Seogi", "goal": "harness measurement"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["prefix"], "SEO");

    session.shutdown();
}

// ── QA 5: project_create 중복 prefix ──

#[test]
fn project_create_duplicate_prefix_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Seogi", "prefix": "SEO", "goal": "first"}),
    );

    let response = session.call(
        "project_create",
        serde_json::json!({"name": "Other", "prefix": "SEO", "goal": "second"}),
    );

    assert!(is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("already exists"));

    session.shutdown();
}

// ── QA 6: project_list ──

#[test]
fn project_list_returns_all_projects() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Alpha", "prefix": "ALP", "goal": "goal1"}),
    );
    session.call(
        "project_create",
        serde_json::json!({"name": "Beta", "prefix": "BET", "goal": "goal2"}),
    );

    let response = session.call("project_list", serde_json::json!({}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(data.len(), 2);

    session.shutdown();
}

// ── QA 7: status_create 성공 ──

#[test]
fn status_create_returns_created_status() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "status_create",
        serde_json::json!({"category": "started", "name": "coding"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["name"], "coding");
    assert_eq!(data["category"], "started");
    assert!(data["id"].is_string());
    assert!(data["position"].is_number());

    session.shutdown();
}

// ── QA 8: status_create 잘못된 category ──

#[test]
fn status_create_invalid_category_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "status_create",
        serde_json::json!({"category": "invalid_cat", "name": "test"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 9: status_list ──

#[test]
fn status_list_returns_all_statuses() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    // DB 초기화 시 기본 7개 상태가 시딩됨
    let response = session.call("status_list", serde_json::json!({}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert!(data.len() >= 7);

    session.shutdown();
}

// ── QA 10: status_update 성공 ──

#[test]
fn status_update_changes_name() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    // 기존 상태 목록에서 하나의 id를 가져옴
    let list_response = session.call("status_list", serde_json::json!({}));
    let text = extract_text(&list_response);
    let statuses: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    let status_id = statuses[0]["id"].as_str().unwrap();

    let response = session.call(
        "status_update",
        serde_json::json!({"id": status_id, "name": "renamed"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("Updated status"));

    session.shutdown();
}

// ── QA 11: status_update 미존재 id ──

#[test]
fn status_update_nonexistent_id_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "status_update",
        serde_json::json!({"id": "nonexistent_id", "name": "test"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 12: status_delete 성공 ──

#[test]
fn status_delete_removes_status() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    // 커스텀 상태를 만들어서 삭제
    let create_response = session.call(
        "status_create",
        serde_json::json!({"category": "started", "name": "temp_status"}),
    );
    let text = extract_text(&create_response);
    let created: serde_json::Value = serde_json::from_str(text).unwrap();
    let status_id = created["id"].as_str().unwrap();

    let response = session.call("status_delete", serde_json::json!({"id": status_id}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("Deleted status"));

    session.shutdown();
}

// ── QA 13: status_delete 참조 중 ──

#[test]
fn status_delete_referenced_by_task_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    // 프로젝트 + 태스크 생성 (태스크는 backlog 상태를 참조)
    session.call(
        "project_create",
        serde_json::json!({"name": "Test", "prefix": "TST", "goal": "test"}),
    );
    session.call(
        "task_create",
        serde_json::json!({
            "project": "Test",
            "title": "task1",
            "description": "desc",
            "label": "feature"
        }),
    );

    // backlog 상태의 id를 가져옴
    let list_response = session.call("status_list", serde_json::json!({}));
    let text = extract_text(&list_response);
    let statuses: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    let backlog_id = statuses
        .iter()
        .find(|s| s["name"].as_str() == Some("backlog"))
        .unwrap()["id"]
        .as_str()
        .unwrap();

    let response = session.call("status_delete", serde_json::json!({"id": backlog_id}));

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 14: task_create 성공 ──

#[test]
fn task_create_returns_created_task() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "goal"}),
    );

    let response = session.call(
        "task_create",
        serde_json::json!({
            "project": "Proj",
            "title": "My Task",
            "description": "A description",
            "label": "feature"
        }),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["id"], "PRJ-1");
    assert_eq!(data["title"], "My Task");
    assert_eq!(data["description"], "A description");
    assert_eq!(data["label"], "feature");

    session.shutdown();
}

// ── QA 15: task_create 미존재 프로젝트 ──

#[test]
fn task_create_nonexistent_project_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "task_create",
        serde_json::json!({
            "project": "NonExistent",
            "title": "task",
            "description": "desc",
            "label": "feature"
        }),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 16: task_create 잘못된 label ──

#[test]
fn task_create_invalid_label_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "goal"}),
    );

    let response = session.call(
        "task_create",
        serde_json::json!({
            "project": "Proj",
            "title": "task",
            "description": "desc",
            "label": "invalid_label"
        }),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 17: task_list ──

#[test]
fn task_list_returns_all_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "goal"}),
    );
    session.call(
        "task_create",
        serde_json::json!({
            "project": "Proj",
            "title": "task1",
            "description": "d1",
            "label": "feature"
        }),
    );
    session.call(
        "task_create",
        serde_json::json!({
            "project": "Proj",
            "title": "task2",
            "description": "d2",
            "label": "bug"
        }),
    );

    let response = session.call("task_list", serde_json::json!({}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(data.len(), 2);

    session.shutdown();
}

// ── QA 18: task_list project 필터 ──

#[test]
fn task_list_filters_by_project() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Alpha", "prefix": "ALP", "goal": "g1"}),
    );
    session.call(
        "project_create",
        serde_json::json!({"name": "Beta", "prefix": "BET", "goal": "g2"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Alpha", "title": "t1", "description": "d", "label": "feature"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Beta", "title": "t2", "description": "d", "label": "bug"}),
    );

    let response = session.call("task_list", serde_json::json!({"project": "Alpha"}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["workspace_name"], "Alpha");

    session.shutdown();
}

// ── QA 19: task_list status 필터 ──

#[test]
fn task_list_filters_by_status() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "feature"}),
    );
    // t1을 todo로 이동
    session.call(
        "task_move",
        serde_json::json!({"task_id": "PRJ-1", "status": "todo"}),
    );

    let response = session.call("task_list", serde_json::json!({"status": "todo"}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["id"], "PRJ-1");

    session.shutdown();
}

// ── QA 20: task_list label 필터 ──

#[test]
fn task_list_filters_by_label() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "bug"}),
    );

    let response = session.call("task_list", serde_json::json!({"label": "bug"}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["label"], "bug");

    session.shutdown();
}

// ── Q8: task_get 성공 ──

#[test]
fn task_get_returns_task_detail() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "goal"}),
    );
    session.call(
        "task_create",
        serde_json::json!({
            "project": "Proj",
            "title": "My Task",
            "description": "Task description here",
            "label": "feature"
        }),
    );

    let response = session.call("task_get", serde_json::json!({"task_id": "PRJ-1"}));

    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["id"], "PRJ-1");
    assert_eq!(data["title"], "My Task");
    assert_eq!(data["description"], "Task description here");
    assert_eq!(data["label"], "feature");
    assert_eq!(data["status_name"], "backlog");
    assert_eq!(data["workspace_name"], "Proj");
    assert!(data["created_at"].is_string());
    assert!(data["updated_at"].is_string());

    session.shutdown();
}

// ── Q9: task_get 미존재 태스크 ──

#[test]
fn task_get_nonexistent_id_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call("task_get", serde_json::json!({"task_id": "XXX-999"}));

    assert!(is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("XXX-999"));

    session.shutdown();
}

// ── Q27: task_depend 성공 ──

#[test]
fn task_depend_success() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}));
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "feature"}));

    let response = session.call(
        "task_depend",
        serde_json::json!({"task_id": "PRJ-2", "depends_on": "PRJ-1"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("PRJ-2"));
    assert!(text.contains("PRJ-1"));

    session.shutdown();
}

// ── Q28: task_depend 순환 → 에러 ──

#[test]
fn task_depend_circular_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}));
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "feature"}));

    session.call(
        "task_depend",
        serde_json::json!({"task_id": "PRJ-2", "depends_on": "PRJ-1"}),
    );
    let response = session.call(
        "task_depend",
        serde_json::json!({"task_id": "PRJ-1", "depends_on": "PRJ-2"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── Q29: task_undepend 성공 ──

#[test]
fn task_undepend_success() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}));
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "feature"}));

    session.call(
        "task_depend",
        serde_json::json!({"task_id": "PRJ-2", "depends_on": "PRJ-1"}),
    );
    let response = session.call(
        "task_undepend",
        serde_json::json!({"task_id": "PRJ-2", "depends_on": "PRJ-1"}),
    );

    assert!(!is_error(&response));

    session.shutdown();
}

// ── Q30: task_get 의존성 포함 응답 ──

#[test]
fn task_get_includes_depends_on() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}));
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t2", "description": "d", "label": "feature"}));

    session.call(
        "task_depend",
        serde_json::json!({"task_id": "PRJ-2", "depends_on": "PRJ-1"}),
    );

    let response = session.call("task_get", serde_json::json!({"task_id": "PRJ-2"}));
    assert!(!is_error(&response));
    let text = extract_text(&response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    let deps = data["depends_on"].as_array().unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "PRJ-1");

    session.shutdown();
}

// ── Q30a: task_create with depends_on ──

#[test]
fn task_create_with_depends_on() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call("task_create", serde_json::json!({"project": "Proj", "title": "t1", "description": "d", "label": "feature"}));

    let response = session.call("task_create", serde_json::json!({
        "project": "Proj", "title": "t2", "description": "d", "label": "feature", "depends_on": "PRJ-1"
    }));
    assert!(!is_error(&response));

    let get_response = session.call("task_get", serde_json::json!({"task_id": "PRJ-2"}));
    let text = extract_text(&get_response);
    let data: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["depends_on"][0], "PRJ-1");

    session.shutdown();
}

// ── task_create with invalid depends_on → 에러 ──

#[test]
fn task_create_with_invalid_depends_on_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );

    let response = session.call("task_create", serde_json::json!({
        "project": "Proj", "title": "t1", "description": "d", "label": "feature", "depends_on": "PRJ-99"
    }));
    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 21: task_update 성공 ──

#[test]
fn task_update_changes_title() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "old", "description": "d", "label": "feature"}),
    );

    let response = session.call(
        "task_update",
        serde_json::json!({"task_id": "PRJ-1", "title": "new title"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("Updated task"));

    session.shutdown();
}

// ── QA 22: task_update 미존재 ID ──

#[test]
fn task_update_nonexistent_id_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "task_update",
        serde_json::json!({"task_id": "XXX-999", "title": "new"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 23: task_update 옵션 미지정 ──

#[test]
fn task_update_no_options_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t", "description": "d", "label": "feature"}),
    );

    let response = session.call("task_update", serde_json::json!({"task_id": "PRJ-1"}));

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 24: task_move 성공 ──

#[test]
fn task_move_transitions_status() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t", "description": "d", "label": "feature"}),
    );

    let response = session.call(
        "task_move",
        serde_json::json!({"task_id": "PRJ-1", "status": "todo"}),
    );

    assert!(!is_error(&response));
    let text = extract_text(&response);
    assert!(text.contains("Moved task PRJ-1"));

    session.shutdown();
}

// ── QA 25: task_move 미존재 ID ──

#[test]
fn task_move_nonexistent_id_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    let response = session.call(
        "task_move",
        serde_json::json!({"task_id": "XXX-999", "status": "todo"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}

// ── QA 26: task_move FSM 위반 ──

#[test]
fn task_move_fsm_violation_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let mut session = McpSession::new(&db_path);

    session.call(
        "project_create",
        serde_json::json!({"name": "Proj", "prefix": "PRJ", "goal": "g"}),
    );
    session.call(
        "task_create",
        serde_json::json!({"project": "Proj", "title": "t", "description": "d", "label": "feature"}),
    );

    // backlog → done 은 FSM 위반 (backlog → unstarted/canceled만 허용)
    let response = session.call(
        "task_move",
        serde_json::json!({"task_id": "PRJ-1", "status": "done"}),
    );

    assert!(is_error(&response));

    session.shutdown();
}
