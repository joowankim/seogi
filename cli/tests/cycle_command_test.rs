mod common;

use common::run_seogi;
use rusqlite::Connection;

fn setup_workspace(db: &str) {
    let output = run_seogi(
        &[
            "workspace",
            "create",
            "--name",
            "Seogi",
            "--prefix",
            "SEO",
            "--goal",
            "하니스 계측",
        ],
        db,
    );
    assert!(
        output.status.success(),
        "workspace setup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Q34: seogi cycle create 성공 메시지에 cycle ID 포함
#[test]
fn test_cycle_create_cli() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    let output = run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Sprint 1"),
        "stdout should contain cycle name: {stdout}"
    );

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM cycles", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// Q35: seogi cycle create 존재하지 않는 워크스페이스 → stderr 에러 출력, exit 1
#[test]
fn test_cycle_create_cli_unknown_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // 워크스페이스 미생성 상태에서 cycle create
    let output = run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "NonExistent",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("NonExistent"),
        "stderr should contain workspace name: {stderr}"
    );
}

// Q36: seogi cycle list 테이블 형식 출력
#[test]
fn test_cycle_list_cli_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    let output = run_seogi(&["cycle", "list"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Sprint 1"), "stdout: {stdout}");
    assert!(stdout.contains("planned"), "stdout: {stdout}");
    assert!(stdout.contains("2026-05-01"), "stdout: {stdout}");
}

// Q37: seogi cycle list --json JSON 형식 출력
#[test]
fn test_cycle_list_cli_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    let output = run_seogi(&["cycle", "list", "--json"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let arr = parsed.as_array().expect("JSON array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "Sprint 1");
    assert_eq!(arr[0]["status"], "planned");
    assert_eq!(arr[0]["start_date"], "2026-05-01");
    assert_eq!(arr[0]["end_date"], "2026-05-14");
}

// Q38: seogi cycle list --workspace "..." 필터링
#[test]
fn test_cycle_list_cli_workspace_filter() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);
    run_seogi(
        &[
            "workspace",
            "create",
            "--name",
            "Other",
            "--prefix",
            "OTH",
            "--goal",
            "other",
        ],
        db,
    );

    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );
    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Other",
            "--name",
            "Sprint A",
            "--start",
            "2026-06-01",
            "--end",
            "2026-06-14",
        ],
        db,
    );

    let output = run_seogi(&["cycle", "list", "--workspace", "Seogi", "--json"], db);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "Sprint 1");
}

// Q39: seogi cycle update 성공 메시지 출력
#[test]
fn test_cycle_update_cli() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    // cycle ID를 가져오기
    let conn = Connection::open(&db_path).unwrap();
    let cycle_id: String = conn
        .query_row("SELECT id FROM cycles LIMIT 1", [], |r| r.get(0))
        .unwrap();

    let output = run_seogi(
        &["cycle", "update", &cycle_id, "--name", "Sprint 1 (updated)"],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&cycle_id),
        "stdout should contain cycle id: {stdout}"
    );

    // DB에서 이름 변경 확인
    let conn = Connection::open(&db_path).unwrap();
    let name: String = conn
        .query_row("SELECT name FROM cycles WHERE id = ?1", [&cycle_id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(name, "Sprint 1 (updated)");
}

// Q23: seogi cycle create 겹침 시 에러 출력, exit 1
#[test]
fn test_cycle_create_overlap_cli() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 1",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-14",
        ],
        db,
    );

    let output = run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Sprint 2",
            "--start",
            "2026-05-10",
            "--end",
            "2026-05-24",
        ],
        db,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("overlaps"),
        "stderr should contain overlap error: {stderr}"
    );
}

// Q24: seogi cycle list --json 파생 status 포함
#[test]
fn test_cycle_list_derived_status_cli() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    setup_workspace(db);

    // 미래 날짜로 생성 → planned
    run_seogi(
        &[
            "cycle",
            "create",
            "--workspace",
            "Seogi",
            "--name",
            "Future Sprint",
            "--start",
            "2099-01-01",
            "--end",
            "2099-01-14",
        ],
        db,
    );

    let output = run_seogi(&["cycle", "list", "--json"], db);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr[0]["status"], "planned");
}
