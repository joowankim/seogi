CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,
    goal        TEXT NOT NULL,
    next_seq    INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    label       TEXT NOT NULL,
    status_id   TEXT NOT NULL REFERENCES statuses(id),
    project_id  TEXT NOT NULL REFERENCES projects(id),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_events (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(id),
    from_status TEXT,
    to_status   TEXT NOT NULL,
    session_id  TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_uses (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    tool_input      TEXT NOT NULL,
    duration_ms     INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_failures (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    error           TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS system_events (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    content         TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS task_dependencies (
    task_id             TEXT NOT NULL REFERENCES tasks(id),
    depends_on_task_id  TEXT NOT NULL REFERENCES tasks(id),
    PRIMARY KEY (task_id, depends_on_task_id)
);

CREATE INDEX IF NOT EXISTS idx_tool_uses_timestamp ON tool_uses(timestamp);
CREATE INDEX IF NOT EXISTS idx_tool_failures_timestamp ON tool_failures(timestamp);

DROP TABLE IF EXISTS session_metrics;

CREATE TABLE IF NOT EXISTS changelog (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);
