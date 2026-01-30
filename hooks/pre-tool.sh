#!/usr/bin/env bash
set -euo pipefail

# stdin에서 hook 데이터 읽기
INPUT=$(cat)

# 세션 ID와 도구명 추출
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')

# 임시 파일에 시작 시간 저장
TEMP_DIR="${TMPDIR:-/tmp}/seogi"
mkdir -p "$TEMP_DIR"

# 현재 시간을 밀리초로 저장
START_TIME=$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')
echo "$START_TIME" > "$TEMP_DIR/${SESSION_ID}_${TOOL_NAME}_start"
