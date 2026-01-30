#!/usr/bin/env bash
set -euo pipefail

SEOGI_DIR="${SEOGI_DIR:-$HOME/.seogi}"
CONFIG_FILE="$SEOGI_DIR/config.json"

# 설정 로드
load_config() {
  if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: Config file not found at $CONFIG_FILE" >&2
    exit 1
  fi

  LOG_DIR=$(jq -r '.logDir' "$CONFIG_FILE" | sed "s|^~|$HOME|")
  MAX_FILE_SIZE_MB=$(jq -r '.maxFileSizeMB // 10' "$CONFIG_FILE")
  MAX_FILE_SIZE_BYTES=$((MAX_FILE_SIZE_MB * 1024 * 1024))
}

# 로그 파일 경로 결정 (롤오버 포함)
get_log_file_path() {
  local project_name="$1"
  local date_str
  date_str=$(date +%Y-%m-%d)

  local project_log_dir="$LOG_DIR/$project_name"
  mkdir -p "$project_log_dir"

  local base_file="$project_log_dir/$date_str.jsonl"

  # 파일이 없거나 크기가 제한 미만이면 기본 파일 사용
  if [[ ! -f "$base_file" ]] || [[ $(stat -f%z "$base_file" 2>/dev/null || stat -c%s "$base_file" 2>/dev/null || echo 0) -lt $MAX_FILE_SIZE_BYTES ]]; then
    echo "$base_file"
    return
  fi

  # 롤오버 파일 찾기
  local counter=1
  while true; do
    local rollover_file
    rollover_file=$(printf "%s/%s_%03d.jsonl" "$project_log_dir" "$date_str" "$counter")

    if [[ ! -f "$rollover_file" ]] || [[ $(stat -f%z "$rollover_file" 2>/dev/null || stat -c%s "$rollover_file" 2>/dev/null || echo 0) -lt $MAX_FILE_SIZE_BYTES ]]; then
      echo "$rollover_file"
      return
    fi

    ((counter++))
  done
}

# JSON 로그 엔트리 작성
write_log_entry() {
  local project_name="$1"
  local json_entry="$2"

  load_config

  local log_file
  log_file=$(get_log_file_path "$project_name")

  echo "$json_entry" >> "$log_file"
}

# 프로젝트 이름 추출 (경로에서)
get_project_name() {
  local project_path="$1"
  basename "$project_path"
}
