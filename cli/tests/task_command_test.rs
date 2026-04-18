mod common;

use common::run_seogi;
use rusqlite::Connection;

fn create_project(db: &str) {
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
        "project create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Q24: task create 성공 메시지에 task ID 포함
#[test]
fn test_task_create_success() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);

    let output = run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "첫 번째 태스크",
            "--description",
            "태스크 설명입니다",
            "--label",
            "feature",
        ],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SEO-1"), "stdout: {stdout}");

    // DB 검증: task 1건, task_events 1건
    let conn = Connection::open(&db_path).unwrap();
    let task_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(task_count, 1);

    let (task_id, title, label, status_id): (String, String, String, String) = conn
        .query_row(
            "SELECT id, title, label, status_id FROM tasks LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(task_id, "SEO-1");
    assert_eq!(title, "첫 번째 태스크");
    assert_eq!(label, "feature");

    // status_id는 backlog 카테고리의 상태를 가리켜야 함
    let category: String = conn
        .query_row(
            "SELECT category FROM statuses WHERE id = ?1",
            [&status_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(category, "backlog");

    // task_events 검증
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(event_count, 1);

    let (from_status, to_status, session_id): (Option<String>, String, String) = conn
        .query_row(
            "SELECT from_status, to_status, session_id FROM task_events LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert!(from_status.is_none());
    assert_eq!(to_status, "backlog");
    assert_eq!(session_id, "CLI");

    // next_seq 증가 검증
    let next_seq: i64 = conn
        .query_row("SELECT next_seq FROM projects LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(next_seq, 2);
}

// Q25: 존재하지 않는 프로젝트 → 에러 출력
#[test]
fn test_task_create_unknown_project() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    // DB 초기화 (run_seogi가 DB를 생성하도록 빈 list 실행)
    run_seogi(&["project", "list"], db);

    let output = run_seogi(
        &[
            "task",
            "create",
            "--project",
            "NonExistent",
            "--title",
            "태스크",
            "--description",
            "설명",
            "--label",
            "feature",
        ],
        db,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("NonExistent"),
        "에러에 프로젝트 이름 포함: {stderr}"
    );
}

// Q26: task list 테이블 형식 출력
#[test]
fn test_task_list_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);

    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "첫 번째",
            "--description",
            "설명1",
            "--label",
            "feature",
        ],
        db,
    );
    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "두 번째",
            "--description",
            "설명2",
            "--label",
            "bug",
        ],
        db,
    );

    let output = run_seogi(&["task", "list"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SEO-1"), "stdout: {stdout}");
    assert!(stdout.contains("SEO-2"), "stdout: {stdout}");
    assert!(stdout.contains("첫 번째"), "stdout: {stdout}");
    assert!(stdout.contains("두 번째"), "stdout: {stdout}");
    assert!(stdout.contains("feature"), "stdout: {stdout}");
    assert!(stdout.contains("bug"), "stdout: {stdout}");
}

// Q27: task list --json → JSON 배열
#[test]
fn test_task_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);

    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "JSON 테스트",
            "--description",
            "설명",
            "--label",
            "chore",
        ],
        db,
    );

    let output = run_seogi(&["task", "list", "--json"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("유효한 JSON이어야 함");
    let arr = parsed.as_array().expect("JSON 배열이어야 함");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "SEO-1");
    assert_eq!(arr[0]["title"], "JSON 테스트");
    assert_eq!(arr[0]["label"], "chore");
    assert_eq!(arr[0]["status_name"], "backlog");
    assert_eq!(arr[0]["project_name"], "Seogi");
}

// Q28: task list 필터링 (project + label)
#[test]
fn test_task_list_filtered() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);

    // 두 번째 프로젝트
    run_seogi(
        &[
            "project", "create", "--name", "Local", "--prefix", "LOC", "--goal", "로컬",
        ],
        db,
    );

    // Seogi 프로젝트 태스크
    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "Seogi feature",
            "--description",
            "설명",
            "--label",
            "feature",
        ],
        db,
    );
    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            "Seogi bug",
            "--description",
            "설명",
            "--label",
            "bug",
        ],
        db,
    );

    // Local 프로젝트 태스크
    run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Local",
            "--title",
            "Local feature",
            "--description",
            "설명",
            "--label",
            "feature",
        ],
        db,
    );

    // --project Seogi --label feature → 1건만
    let output = run_seogi(
        &[
            "task",
            "list",
            "--project",
            "Seogi",
            "--label",
            "feature",
            "--json",
        ],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("유효한 JSON이어야 함");
    let arr = parsed.as_array().expect("JSON 배열이어야 함");
    assert_eq!(arr.len(), 1, "Seogi+feature 필터 결과 1건: {stdout}");
    assert_eq!(arr[0]["id"], "SEO-1");
    assert_eq!(arr[0]["title"], "Seogi feature");
}

fn create_task(db: &str, title: &str, label: &str) {
    let output = run_seogi(
        &[
            "task",
            "create",
            "--project",
            "Seogi",
            "--title",
            title,
            "--description",
            "설명",
            "--label",
            label,
        ],
        db,
    );
    assert!(
        output.status.success(),
        "task create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Q11: task update --title 성공, DB 반영
#[test]
fn test_task_update_title() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);
    create_task(db, "원래 제목", "feature");

    let output = run_seogi(&["task", "update", "SEO-1", "--title", "변경된 제목"], db);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("SEO-1"), "stdout: {stdout}");

    // DB 검증
    let conn = Connection::open(&db_path).unwrap();
    let title: String = conn
        .query_row("SELECT title FROM tasks WHERE id = 'SEO-1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(title, "변경된 제목");
}

// Q12: 존재하지 않는 태스크 → 에러
#[test]
fn test_task_update_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);

    let output = run_seogi(&["task", "update", "SEO-99", "--title", "new"], db);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("SEO-99"), "stderr: {stderr}");
}

// Q13: 옵션 없음 → 에러
#[test]
fn test_task_update_no_options() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);
    create_task(db, "제목", "feature");

    let output = run_seogi(&["task", "update", "SEO-1"], db);

    assert!(!output.status.success());
}

// Q14: 복합 수정 (title + label)
#[test]
fn test_task_update_combined() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_project(db);
    create_task(db, "원래", "feature");

    let output = run_seogi(
        &[
            "task",
            "update",
            "SEO-1",
            "--title",
            "새 제목",
            "--label",
            "bug",
        ],
        db,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conn = Connection::open(&db_path).unwrap();
    let (title, label): (String, String) = conn
        .query_row(
            "SELECT title, label FROM tasks WHERE id = 'SEO-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(title, "새 제목");
    assert_eq!(label, "bug");
}
