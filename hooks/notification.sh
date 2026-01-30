#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/logger.sh"

# stdin에서 hook 데이터 읽기
INPUT=$(cat)

# 필드 추출
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
NOTIFICATION_TYPE=$(echo "$INPUT" | jq -r '.type // "unknown"')
MESSAGE=$(echo "$INPUT" | jq -r '.message // ""')
PROJECT_PATH=$(echo "$INPUT" | jq -r '.cwd // "unknown"')

# 프로젝트 이름 추출
PROJECT_NAME=$(get_project_name "$PROJECT_PATH")

# 타임스탬프 생성
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)

# 로그 엔트리 생성
LOG_ENTRY=$(jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg sessionId "$SESSION_ID" \
  --arg project "$PROJECT_NAME" \
  --arg projectPath "$PROJECT_PATH" \
  --arg role "system" \
  --arg content "[$NOTIFICATION_TYPE] $MESSAGE" \
  '{
    timestamp: $timestamp,
    sessionId: $sessionId,
    project: $project,
    projectPath: $projectPath,
    role: $role,
    content: $content,
    tool: null
  }')

# 로그 작성
write_log_entry "$PROJECT_NAME" "$LOG_ENTRY"
