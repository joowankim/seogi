#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/logger.sh"

# stdin에서 hook 데이터 읽기
INPUT=$(cat)

# 필드 추출
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
PROJECT_PATH=$(echo "$INPUT" | jq -r '.cwd // "unknown"')
STOP_REASON=$(echo "$INPUT" | jq -r '.stop_reason // "unknown"')

# 프로젝트 이름 추출
PROJECT_NAME=$(get_project_name "$PROJECT_PATH")

# 세션 종료 로그 기록
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)
LOG_ENTRY=$(jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg sessionId "$SESSION_ID" \
  --arg project "$PROJECT_NAME" \
  --arg projectPath "$PROJECT_PATH" \
  --arg role "system" \
  --arg content "[stop] $STOP_REASON" \
  '{
    timestamp: $timestamp,
    sessionId: $sessionId,
    project: $project,
    projectPath: $projectPath,
    role: $role,
    content: $content,
    tool: null
  }')

write_log_entry "$PROJECT_NAME" "$LOG_ENTRY"

# 세션 요약 분석기를 백그라운드로 실행 (세션 종료 지연 방지)
SEOGI_BIN="$SCRIPT_DIR/../bin/seogi"
if [[ -x "$SEOGI_BIN" ]]; then
  "$SEOGI_BIN" analyze "$PROJECT_NAME" "$SESSION_ID" &
fi
