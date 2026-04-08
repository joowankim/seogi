#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/logger.sh"

INPUT=$(cat)

SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')
ERROR_MSG=$(echo "$INPUT" | jq -r '.error // ""')
PROJECT_PATH=$(echo "$INPUT" | jq -r '.cwd // "unknown"')

PROJECT_NAME=$(get_project_name "$PROJECT_PATH")
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)

LOG_ENTRY=$(jq -cn \
  --arg timestamp "$TIMESTAMP" \
  --arg sessionId "$SESSION_ID" \
  --arg project "$PROJECT_NAME" \
  --arg projectPath "$PROJECT_PATH" \
  --arg role "assistant" \
  --arg toolName "$TOOL_NAME" \
  --arg error "$ERROR_MSG" \
  '{
    timestamp: $timestamp,
    sessionId: $sessionId,
    project: $project,
    projectPath: $projectPath,
    role: $role,
    tool: {name: $toolName, failed: true, error: $error}
  }')

write_log_entry "$PROJECT_NAME" "$LOG_ENTRY"
