mod common;

use common::run_seogi;
use rusqlite::Connection;

// Q17: 명시적 prefix로 create → 성공 메시지, exit 0
#[test]
fn test_project_create_with_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(
        &[
            "project",
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
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SEO"), "stdout: {stdout}");

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let (name, prefix, goal, next_seq): (String, String, String, i64) = conn
        .query_row(
            "SELECT name, prefix, goal, next_seq FROM projects LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(name, "Seogi");
    assert_eq!(prefix, "SEO");
    assert_eq!(goal, "하니스 계측");
    assert_eq!(next_seq, 1);
}

// Q18: 중복 prefix → stderr 에러, exit != 0
#[test]
fn test_project_create_duplicate_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // 첫 번째 생성: 성공
    let output1 = run_seogi(
        &[
            "project",
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
    assert!(output1.status.success());

    // 두 번째 생성: 중복 prefix
    let output2 = run_seogi(
        &[
            "project",
            "create",
            "--name",
            "Other",
            "--prefix",
            "SEO",
            "--goal",
            "다른 프로젝트",
        ],
        db,
    );
    assert!(!output2.status.success());

    let stderr = String::from_utf8(output2.stderr).unwrap();
    assert!(
        stderr.contains("SEO"),
        "에러 메시지에 중복 prefix가 포함되어야 함: {stderr}"
    );

    // DB에 여전히 1개만 존재
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// Q19: list → 테이블 출력
#[test]
fn test_project_list_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // 프로젝트 2개 생성
    run_seogi(
        &[
            "project",
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
    run_seogi(
        &[
            "project",
            "create",
            "--name",
            "Local",
            "--prefix",
            "LOC",
            "--goal",
            "로컬 개발",
        ],
        db,
    );

    let output = run_seogi(&["project", "list"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SEO"), "stdout: {stdout}");
    assert!(stdout.contains("LOC"), "stdout: {stdout}");
    assert!(stdout.contains("Seogi"), "stdout: {stdout}");
    assert!(stdout.contains("Local"), "stdout: {stdout}");
}

// Q20: list --json → JSON 배열
#[test]
fn test_project_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    run_seogi(
        &[
            "project",
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

    let output = run_seogi(&["project", "list", "--json"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("유효한 JSON이어야 함");
    let arr = parsed.as_array().expect("JSON 배열이어야 함");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["prefix"], "SEO");
    assert_eq!(arr[0]["name"], "Seogi");
}

// Q21: 빈 목록 → 빈 결과
#[test]
fn test_project_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(&["project", "list", "--json"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("유효한 JSON이어야 함");
    let arr = parsed.as_array().expect("JSON 배열이어야 함");
    assert!(arr.is_empty());
}
