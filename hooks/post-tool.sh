#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/logger.sh"

# stdin에서 hook 데이터 읽기
INPUT=$(cat)

# 필드 추출
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // null')
TOOL_INPUT=$(echo "$INPUT" | jq -r '.tool_input // null')
TOOL_OUTPUT=$(echo "$INPUT" | jq -r '.tool_output // null')
PROJECT_PATH=$(echo "$INPUT" | jq -r '.cwd // "unknown"')
ROLE=$(echo "$INPUT" | jq -r '.role // "assistant"')
CONTENT=$(echo "$INPUT" | jq -r '.content // .tool_output // ""')

# 프로젝트 이름 추출
PROJECT_NAME=$(get_project_name "$PROJECT_PATH")

# 타임스탬프 생성
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)

# 소요 시간 계산
TEMP_DIR="${TMPDIR:-/tmp}/seogi"
DURATION_MS=0
START_FILE="$TEMP_DIR/${SESSION_ID}_${TOOL_NAME}_start"

if [[ -f "$START_FILE" ]]; then
  START_TIME=$(cat "$START_FILE")
  END_TIME=$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')
  DURATION_MS=$((END_TIME - START_TIME))
  rm -f "$START_FILE"
fi

# tool 객체 생성
if [[ "$TOOL_NAME" != "null" && -n "$TOOL_NAME" ]]; then
  TOOL_JSON=$(jq -n \
    --arg name "$TOOL_NAME" \
    --argjson duration "$DURATION_MS" \
    '{name: $name, duration_ms: $duration}')
else
  TOOL_JSON="null"
fi

# 로그 엔트리 생성
LOG_ENTRY=$(jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg sessionId "$SESSION_ID" \
  --arg project "$PROJECT_NAME" \
  --arg projectPath "$PROJECT_PATH" \
  --argjson tool "$TOOL_JSON" \
  --arg role "$ROLE" \
  --arg content "$CONTENT" \
  '{
    timestamp: $timestamp,
    sessionId: $sessionId,
    project: $project,
    projectPath: $projectPath,
    role: $role,
    content: $content,
    tool: $tool
  }')

# 로그 작성
write_log_entry "$PROJECT_NAME" "$LOG_ENTRY"
