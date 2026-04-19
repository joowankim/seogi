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

fn spawn_mcp_server() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_seogi"))
        .args(["mcp-server"])
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

#[test]
fn mcp_server_handles_initialize_and_responds_with_server_info() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    let response = initialize(&mut stdin, &mut reader);
    let result = &response["result"];
    assert_eq!(result["serverInfo"]["name"], "seogi");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn mcp_server_returns_empty_tools_list() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    initialize(&mut stdin, &mut reader);

    send_jsonrpc(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    );

    let response = recv_jsonrpc(&mut reader);
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools.is_empty());

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn mcp_server_exits_cleanly_on_stdin_eof() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    initialize(&mut stdin, &mut reader);

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}
