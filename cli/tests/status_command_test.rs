mod common;

use common::run_seogi;
use rusqlite::Connection;

// Q20: create → 성공 메시지, exit 0
#[test]
fn test_status_create() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(
        &[
            "status",
            "create",
            "--category",
            "started",
            "--name",
            "testing",
        ],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("testing"), "stdout: {stdout}");
    assert!(stdout.contains("started"), "stdout: {stdout}");

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM statuses", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 8); // 7 seeded + 1 created
}

// Q21: list → 테이블 (7개 시딩 포함)
#[test]
fn test_status_list_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(&["status", "list"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("backlog"), "stdout: {stdout}");
    assert!(stdout.contains("todo"), "stdout: {stdout}");
    assert!(stdout.contains("in_progress"), "stdout: {stdout}");
    assert!(stdout.contains("done"), "stdout: {stdout}");
    assert!(stdout.contains("canceled"), "stdout: {stdout}");
}

// Q22: list --json → JSON 배열
#[test]
fn test_status_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(&["status", "list", "--json"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("유효한 JSON이어야 함");
    let arr = parsed.as_array().expect("JSON 배열이어야 함");
    assert_eq!(arr.len(), 7); // 7 seeded
    assert_eq!(arr[0]["name"], "backlog");
}

// Q23: update → 성공 메시지
#[test]
fn test_status_update() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // 시딩된 "backlog" 상태(id: 00000000000000000000000000000001) 이름 변경
    let id = "00000000000000000000000000000001";
    let output = run_seogi(&["status", "update", id, "--name", "renamed"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(id), "stdout: {stdout}");

    // DB에서 이름 변경 확인
    let conn = Connection::open(&db_path).unwrap();
    let name: String = conn
        .query_row("SELECT name FROM statuses WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(name, "renamed");
}

// Q24: delete → 성공 메시지
#[test]
fn test_status_delete() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // 시딩된 "blocked" 상태(id: 00000000000000000000000000000005) 삭제
    let id = "00000000000000000000000000000005";
    let output = run_seogi(&["status", "delete", id], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(id), "stdout: {stdout}");

    // DB에서 삭제 확인
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM statuses", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 6); // 7 seeded - 1 deleted
}
