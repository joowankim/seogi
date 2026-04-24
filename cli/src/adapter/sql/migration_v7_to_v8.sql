CREATE TABLE IF NOT EXISTS cycles (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name         TEXT NOT NULL,
    status       TEXT NOT NULL,
    start_date   TEXT NOT NULL,
    end_date     TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cycle_tasks (
    cycle_id    TEXT NOT NULL REFERENCES cycles(id),
    task_id     TEXT NOT NULL REFERENCES tasks(id),
    assigned    TEXT NOT NULL,
    PRIMARY KEY (cycle_id, task_id)
);
