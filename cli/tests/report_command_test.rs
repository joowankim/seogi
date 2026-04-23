mod common;

use common::run_seogi;

fn create_workspace(db: &str) {
    let output = run_seogi(
        &[
            "workspace",
            "create",
            "--name",
            "Seogi",
            "--prefix",
            "SEO",
            "--goal",
            "test goal",
        ],
        db,
    );
    assert!(
        output.status.success(),
        "workspace create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_task(db: &str) {
    let output = run_seogi(
        &[
            "task",
            "create",
            "--workspace",
            "Seogi",
            "--title",
            "Test task",
            "--description",
            "A test task",
            "--label",
            "feature",
        ],
        db,
    );
    assert!(
        output.status.success(),
        "task create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn move_task(db: &str, task_id: &str, status: &str) {
    let output = run_seogi(&["task", "move", task_id, status], db);
    assert!(
        output.status.success(),
        "task move failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// E2E: basic report with no completed tasks
#[test]
fn test_report_no_completed_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_workspace(db);

    let output = run_seogi(
        &["report", "--from", "2020-01-01", "--to", "2030-12-31"],
        db,
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No completed tasks"),
        "Expected 'No completed tasks' but got: {stdout}"
    );
}

// E2E: report with completed task
#[test]
fn test_report_with_completed_task() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_workspace(db);
    create_task(db);

    // Move task through workflow: backlog -> todo -> in_progress -> done
    move_task(db, "SEO-1", "todo");
    move_task(db, "SEO-1", "in_progress");
    move_task(db, "SEO-1", "done");

    let output = run_seogi(
        &["report", "--from", "2020-01-01", "--to", "2030-12-31"],
        db,
    );
    assert!(
        output.status.success(),
        "report failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SEO-1"),
        "Expected task ID in output: {stdout}"
    );
    assert!(
        stdout.contains("throughput"),
        "Expected throughput in output: {stdout}"
    );
}

// E2E: report with --detail flag
#[test]
fn test_report_detail_flag() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_workspace(db);
    create_task(db);

    move_task(db, "SEO-1", "todo");
    move_task(db, "SEO-1", "in_progress");
    move_task(db, "SEO-1", "done");

    let output = run_seogi(
        &[
            "report",
            "--from",
            "2020-01-01",
            "--to",
            "2030-12-31",
            "--detail",
        ],
        db,
    );
    assert!(
        output.status.success(),
        "report --detail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== SEO-1:"),
        "Expected detail header in output: {stdout}"
    );
    assert!(
        stdout.contains("cycle_time:"),
        "Expected cycle_time in output: {stdout}"
    );
}

// E2E: report with --workspace filter
#[test]
fn test_report_workspace_filter() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    create_workspace(db);
    create_task(db);

    move_task(db, "SEO-1", "todo");
    move_task(db, "SEO-1", "in_progress");
    move_task(db, "SEO-1", "done");

    // Filter by matching workspace
    let output = run_seogi(
        &[
            "report",
            "--from",
            "2020-01-01",
            "--to",
            "2030-12-31",
            "--workspace",
            "Seogi",
        ],
        db,
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SEO-1"),
        "Expected task with matching workspace: {stdout}"
    );

    // Filter by non-matching workspace
    let output = run_seogi(
        &[
            "report",
            "--from",
            "2020-01-01",
            "--to",
            "2030-12-31",
            "--workspace",
            "OtherProject",
        ],
        db,
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No completed tasks"),
        "Expected no tasks for non-matching workspace: {stdout}"
    );
}

// E2E: report with invalid date range (from > to)
#[test]
fn test_report_invalid_date_range() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let db = db_path.to_str().unwrap();

    let output = run_seogi(
        &["report", "--from", "2026-02-01", "--to", "2026-01-01"],
        db,
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid date range") || stderr.contains("Failed to generate report"),
        "Expected error message: {stderr}"
    );
}
