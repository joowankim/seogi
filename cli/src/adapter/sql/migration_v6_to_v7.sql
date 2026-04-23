-- Phase 6 Step 1: project → workspace 리네이밍

-- 1) projects → workspaces
ALTER TABLE projects RENAME TO workspaces;

-- 2) tasks.project_id → workspace_id
ALTER TABLE tasks RENAME COLUMN project_id TO workspace_id;

-- 3) tool_uses
ALTER TABLE tool_uses RENAME COLUMN project TO workspace;
ALTER TABLE tool_uses RENAME COLUMN project_path TO workspace_path;

-- 4) tool_failures
ALTER TABLE tool_failures RENAME COLUMN project TO workspace;
ALTER TABLE tool_failures RENAME COLUMN project_path TO workspace_path;

-- 5) system_events
ALTER TABLE system_events RENAME COLUMN project TO workspace;
ALTER TABLE system_events RENAME COLUMN project_path TO workspace_path;
