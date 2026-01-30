# Seogi Hook Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Claude Code에서 LLM 대화를 실시간으로 JSONL 파일에 기록하는 Hook 시스템 구현

**Architecture:** Bash + jq 기반 Hook 스크립트. PreToolUse에서 시작 시간 기록, PostToolUse에서 로그 작성, 공통 logger.sh가 파일 롤오버 처리.

**Tech Stack:** Bash, jq, Claude Code Hooks

---

### Task 1: 프로젝트 디렉토리 구조 생성

**Files:**
- Create: `hooks/pre-tool.sh`
- Create: `hooks/post-tool.sh`
- Create: `hooks/notification.sh`
- Create: `lib/logger.sh`
- Create: `config.json`

**Step 1: 디렉토리 구조 생성**

```bash
mkdir -p hooks lib
touch hooks/pre-tool.sh hooks/post-tool.sh hooks/notification.sh
touch lib/logger.sh config.json
chmod +x hooks/*.sh lib/*.sh
```

**Step 2: 구조 확인**

Run: `find . -type f | grep -E '\.(sh|json)$' | sort`
Expected:
```
./config.json
./hooks/notification.sh
./hooks/post-tool.sh
./hooks/pre-tool.sh
./lib/logger.sh
```

**Step 3: Commit**

```bash
git add -A
git commit -m "chore: create project directory structure"
```

---

### Task 2: 설정 파일 구현

**Files:**
- Modify: `config.json`

**Step 1: 기본 설정 작성**

```json
{
  "logDir": "~/seogi-logs",
  "maxFileSizeMB": 10
}
```

**Step 2: JSON 유효성 검증**

Run: `jq . config.json`
Expected: 동일한 JSON 출력 (파싱 성공)

**Step 3: Commit**

```bash
git add config.json
git commit -m "feat: add default configuration file"
```

---

### Task 3: logger.sh 공통 함수 구현

**Files:**
- Modify: `lib/logger.sh`

**Step 1: 설정 로드 함수 작성**

```bash
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
```

**Step 2: 로그 파일 경로 결정 함수 작성**

```bash
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
```

**Step 3: 로그 작성 함수 작성**

```bash
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
```

**Step 4: 함수 테스트**

Run: `bash -n lib/logger.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 5: Commit**

```bash
git add lib/logger.sh
git commit -m "feat: implement logger with file rollover support"
```

---

### Task 4: pre-tool.sh Hook 구현

**Files:**
- Modify: `hooks/pre-tool.sh`

**Step 1: 시작 시간 기록 스크립트 작성**

```bash
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
```

**Step 2: 문법 검증**

Run: `bash -n hooks/pre-tool.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 3: Commit**

```bash
git add hooks/pre-tool.sh
git commit -m "feat: implement pre-tool hook for timing"
```

---

### Task 5: post-tool.sh Hook 구현

**Files:**
- Modify: `hooks/post-tool.sh`

**Step 1: 로그 기록 스크립트 작성**

```bash
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
  --arg role "$ROLE" \
  --arg content "$CONTENT" \
  --argjson tool "$TOOL_JSON" \
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
```

**Step 2: 문법 검증**

Run: `bash -n hooks/post-tool.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 3: Commit**

```bash
git add hooks/post-tool.sh
git commit -m "feat: implement post-tool hook for logging"
```

---

### Task 6: notification.sh Hook 구현

**Files:**
- Modify: `hooks/notification.sh`

**Step 1: 알림 로깅 스크립트 작성**

```bash
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
```

**Step 2: 문법 검증**

Run: `bash -n hooks/notification.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 3: Commit**

```bash
git add hooks/notification.sh
git commit -m "feat: implement notification hook for session events"
```

---

### Task 7: install.sh 설치 스크립트 구현

**Files:**
- Create: `install.sh`

**Step 1: 설치 스크립트 작성**

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Installing seogi..."

SEOGI_DIR="$HOME/.seogi"
SEOGI_LOGS_DIR="$HOME/seogi-logs"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. seogi 디렉토리 생성 및 파일 복사
echo "Creating $SEOGI_DIR..."
mkdir -p "$SEOGI_DIR/hooks" "$SEOGI_DIR/lib"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/config.json" "$SEOGI_DIR/"
cp "$SCRIPT_DIR/hooks/"*.sh "$SEOGI_DIR/hooks/"
cp "$SCRIPT_DIR/lib/"*.sh "$SEOGI_DIR/lib/"
chmod +x "$SEOGI_DIR/hooks/"*.sh "$SEOGI_DIR/lib/"*.sh

# 2. 기본 로그 디렉토리 생성
echo "Creating log directory at $SEOGI_LOGS_DIR..."
mkdir -p "$SEOGI_LOGS_DIR"

# 3. Claude Code 설정에 hook 추가
echo "Configuring Claude Code hooks..."
mkdir -p "$(dirname "$CLAUDE_SETTINGS")"

if [[ -f "$CLAUDE_SETTINGS" ]]; then
  # 기존 설정이 있으면 백업
  cp "$CLAUDE_SETTINGS" "$CLAUDE_SETTINGS.backup"

  # hooks 섹션 추가/병합
  UPDATED_SETTINGS=$(jq --arg seogi_dir "$SEOGI_DIR" '
    .hooks = (.hooks // {}) |
    .hooks.PreToolUse = (.hooks.PreToolUse // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/pre-tool.sh")]
    }] |
    .hooks.PostToolUse = (.hooks.PostToolUse // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/post-tool.sh")]
    }] |
    .hooks.Notification = (.hooks.Notification // []) + [{
      "matcher": "*",
      "hooks": [($seogi_dir + "/hooks/notification.sh")]
    }]
  ' "$CLAUDE_SETTINGS")

  echo "$UPDATED_SETTINGS" > "$CLAUDE_SETTINGS"
else
  # 새 설정 파일 생성
  jq -n --arg seogi_dir "$SEOGI_DIR" '{
    hooks: {
      PreToolUse: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/pre-tool.sh")]
      }],
      PostToolUse: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/post-tool.sh")]
      }],
      Notification: [{
        matcher: "*",
        hooks: [($seogi_dir + "/hooks/notification.sh")]
      }]
    }
  }' > "$CLAUDE_SETTINGS"
fi

echo ""
echo "✓ seogi installed successfully!"
echo ""
echo "Configuration: $SEOGI_DIR/config.json"
echo "Logs will be saved to: $SEOGI_LOGS_DIR"
echo ""
echo "To customize, edit $SEOGI_DIR/config.json:"
echo '  {"logDir": "~/seogi-logs", "maxFileSizeMB": 10}'
```

**Step 2: 실행 권한 부여 및 문법 검증**

Run: `chmod +x install.sh && bash -n install.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 3: Commit**

```bash
git add install.sh
git commit -m "feat: add installation script"
```

---

### Task 8: uninstall.sh 제거 스크립트 구현

**Files:**
- Create: `uninstall.sh`

**Step 1: 제거 스크립트 작성**

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Uninstalling seogi..."

SEOGI_DIR="$HOME/.seogi"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# 1. Claude Code 설정에서 hook 제거
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  echo "Removing hooks from Claude Code settings..."

  UPDATED_SETTINGS=$(jq --arg seogi_dir "$SEOGI_DIR" '
    if .hooks then
      .hooks.PreToolUse = [.hooks.PreToolUse[]? | select(.hooks[]? | contains($seogi_dir) | not)] |
      .hooks.PostToolUse = [.hooks.PostToolUse[]? | select(.hooks[]? | contains($seogi_dir) | not)] |
      .hooks.Notification = [.hooks.Notification[]? | select(.hooks[]? | contains($seogi_dir) | not)] |
      if .hooks.PreToolUse == [] then del(.hooks.PreToolUse) else . end |
      if .hooks.PostToolUse == [] then del(.hooks.PostToolUse) else . end |
      if .hooks.Notification == [] then del(.hooks.Notification) else . end |
      if .hooks == {} then del(.hooks) else . end
    else . end
  ' "$CLAUDE_SETTINGS")

  echo "$UPDATED_SETTINGS" > "$CLAUDE_SETTINGS"
fi

# 2. seogi 디렉토리 제거
if [[ -d "$SEOGI_DIR" ]]; then
  echo "Removing $SEOGI_DIR..."
  rm -rf "$SEOGI_DIR"
fi

echo ""
echo "✓ seogi uninstalled successfully!"
echo ""
echo "Note: Log files in ~/seogi-logs were preserved."
echo "To remove logs: rm -rf ~/seogi-logs"
```

**Step 2: 실행 권한 부여 및 문법 검증**

Run: `chmod +x uninstall.sh && bash -n uninstall.sh && echo "Syntax OK"`
Expected: `Syntax OK`

**Step 3: Commit**

```bash
git add uninstall.sh
git commit -m "feat: add uninstallation script"
```

---

### Task 9: README.md 문서 작성

**Files:**
- Create: `README.md`

**Step 1: README 작성**

```markdown
# Seogi (서기)

Claude Code에서 LLM과의 대화를 실시간으로 로깅하는 Hook 플러그인.

## 기능

- 실시간 대화 로깅 (JSONL 형식)
- 프로젝트별 로그 파일 분리
- 자동 파일 롤오버 (기본 10MB)
- 도구 사용 시간 측정

## 설치

```bash
git clone git@github.com:joowankim/seogi.git
cd seogi
./install.sh
```

## 설정

`~/.seogi/config.json` 파일을 편집하세요:

```json
{
  "logDir": "~/seogi-logs",
  "maxFileSizeMB": 10
}
```

| 설정 | 설명 | 기본값 |
|------|------|--------|
| `logDir` | 로그 저장 디렉토리 | `~/seogi-logs` |
| `maxFileSizeMB` | 파일 롤오버 크기 (MB) | `10` |

## 로그 형식

로그는 JSONL (JSON Lines) 형식으로 저장됩니다:

```
~/seogi-logs/
  └── {프로젝트명}/
      ├── 2026-01-30.jsonl
      ├── 2026-01-30_001.jsonl  (롤오버)
      └── ...
```

각 로그 엔트리:

```json
{
  "timestamp": "2026-01-30T14:23:45.000Z",
  "sessionId": "abc123",
  "project": "my-project",
  "projectPath": "/path/to/my-project",
  "role": "assistant",
  "content": "메시지 내용...",
  "tool": {
    "name": "Edit",
    "duration_ms": 1523
  }
}
```

## 제거

```bash
cd seogi
./uninstall.sh
```

로그 파일은 보존됩니다. 완전 삭제:

```bash
rm -rf ~/seogi-logs
```

## 의존성

- `jq` - JSON 처리

macOS: `brew install jq`
Ubuntu: `apt install jq`

## 라이선스

MIT
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with installation and usage guide"
```

---

### Task 10: LICENSE 파일 추가

**Files:**
- Create: `LICENSE`

**Step 1: MIT 라이선스 추가**

```
MIT License

Copyright (c) 2026 Joowan Kim

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

**Step 2: Commit**

```bash
git add LICENSE
git commit -m "chore: add MIT license"
```

---

### Task 11: 수동 테스트

**Step 1: 설치 테스트**

```bash
./install.sh
```

Expected: 설치 성공 메시지

**Step 2: 설정 확인**

```bash
cat ~/.seogi/config.json
cat ~/.claude/settings.json | jq '.hooks'
```

Expected: 설정 파일과 hook 등록 확인

**Step 3: Claude Code에서 테스트**

새 터미널에서 Claude Code 실행 후 간단한 작업 수행.

**Step 4: 로그 확인**

```bash
ls ~/seogi-logs/
cat ~/seogi-logs/{project-name}/*.jsonl | jq .
```

Expected: JSONL 로그 엔트리 확인

**Step 5: 제거 테스트**

```bash
./uninstall.sh
```

Expected: 제거 성공 메시지, ~/.seogi 삭제됨

---

### Task 12: GitHub에 푸시

**Step 1: 원격 저장소 설정**

```bash
git remote add origin git@github.com:joowankim/seogi.git 2>/dev/null || true
```

**Step 2: 브랜치 푸시**

```bash
git push -u origin feature/hook-impl
```

**Step 3: PR 생성**

```bash
gh pr create --title "feat: implement seogi logging hooks" --body "## Summary
- Real-time JSONL logging via Claude Code hooks
- Project+date based file organization with configurable rollover
- Install/uninstall scripts for easy setup

## Test Plan
- [ ] Run install.sh and verify ~/.seogi created
- [ ] Run Claude Code and verify logs in ~/seogi-logs
- [ ] Run uninstall.sh and verify cleanup"
```

---

## 완료 후 체크리스트

- [ ] 모든 hook 스크립트 문법 검증 통과
- [ ] install.sh 실행 성공
- [ ] Claude Code에서 로그 생성 확인
- [ ] 파일 롤오버 동작 확인 (선택)
- [ ] uninstall.sh 실행 성공
- [ ] GitHub PR 생성
