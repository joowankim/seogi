# Phase 1 구현 계획

상위 문서: [measurement-framework.md](./2026-04-08-measurement-framework.md)

---

## 목표

Phase 1은 두 가지를 달성한다:
1. 로그 보강 (PostToolUseFailure 훅 추가)
2. 지표 확장 (기존 session-summary.sh에 신규 지표 추가)

Python 전환(패키지 구조, 분석기, CLI, pytest)은 Phase 2에서 일괄 수행한다.

---

## Step 1: PostToolUseFailure 훅 추가

### hooks/post-tool-failure.sh

PostToolUseFailure 훅의 stdin 필드:
- `session_id`, `tool_name`, `tool_input`, `error`, `is_interrupt`, `cwd`

```bash
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
```

### ~/.claude/settings.json 등록 형태

```json
{
  "PostToolUseFailure": [{
    "matcher": "*",
    "hooks": [{"type": "command", "command": "~/.seogi/hooks/post-tool-failure.sh"}]
  }]
}
```

---

## Step 2: session-summary.sh 지표 확장

기존 bash/jq 분석기에 4개 지표를 추가한다.

### 신규 지표

| # | 지표 | 타입 | 계산 로직 |
|---|---|---|---|
| 8 | `lint_invoked` | bool | Bash command에 lint/eslint/prettier/ruff/biome 포함 |
| 9 | `typecheck_invoked` | bool | Bash command에 tsc --noEmit/mypy/pyright 포함 |
| 10 | `bash_error_rate` | float | Bash 실패 수 / Bash 전체 수 (tool.failed 기반) |

### 기존 지표 (변경 없음)

| # | 지표 | 타입 |
|---|---|---|
| 1 | `read_before_edit_ratio` | int |
| 2 | `doom_loop_count` | int |
| 3 | `test_invoked` | bool |
| 4 | `build_invoked` | bool |
| 5 | `tool_call_count` | int |
| 6 | `session_duration_ms` | int |
| 7 | `edit_files` | list[str] |

### bash_error_rate 계산 로직

- PostToolUseFailure 훅이 `tool.failed = true`로 기록한 Bash 엔트리를 실패로 카운트
- PostToolUse 훅이 기록한 Bash 엔트리를 성공으로 카운트
- `bash_error_rate = 실패 / (성공 + 실패)`
- Bash 호출이 0이면 `0.0`

### jq 추가 로직 (session-summary.sh에 삽입)

```jq
# 8. lint_invoked
(
  [$tool_calls[] | select(.tool.name == "Bash") | .tool.input.command // ""] |
  any(test("\\b(lint|eslint|prettier|ruff|biome)\\b"; "i"))
) as $lint_invoked |

# 9. typecheck_invoked
(
  [$tool_calls[] | select(.tool.name == "Bash") | .tool.input.command // ""] |
  any(test("\\b(tsc\\s+--noEmit|mypy|pyright)\\b"; "i"))
) as $typecheck_invoked |

# 10. bash_error_rate
(
  [$sorted[] | select(.tool != null and .tool.name == "Bash")] as $all_bash |
  [$all_bash[] | select(.tool.failed == true)] as $failed_bash |
  if ($all_bash | length) > 0 then
    (($failed_bash | length) / ($all_bash | length) * 100 | round / 100)
  else 0 end
) as $bash_error_rate |
```

---

## Step 3: install.sh / uninstall.sh 업데이트

### install.sh 변경

1. `post-tool-failure.sh` 복사 (기존 hooks 복사와 동일)
2. PostToolUseFailure 훅 등록 추가

```bash
# 기존 훅 등록 jq에 추가:
.hooks.PostToolUseFailure = (.hooks.PostToolUseFailure // []) + [{
  "matcher": "*",
  "hooks": [($seogi_dir + "/hooks/post-tool-failure.sh")]
}]
```

### uninstall.sh 변경

PostToolUseFailure 훅 제거 추가:

```bash
# 기존 제거 jq에 추가:
.hooks.PostToolUseFailure = [.hooks.PostToolUseFailure[]? | select(.hooks[]? | (type == "string" and contains($seogi_dir)) | not)] |
if .hooks.PostToolUseFailure == [] then del(.hooks.PostToolUseFailure) else . end |
```

---

## 실행 순서 요약

```
Step 1: post-tool-failure.sh 훅 추가 + ~/.seogi/ 배포 + settings.json 등록
Step 2: session-summary.sh에 4개 지표 추가 + ~/.seogi/ 배포
Step 3: install.sh / uninstall.sh 업데이트
```

각 Step 완료 후 실제 로그 데이터로 동작을 확인한다.

---

## 미결 사항

없음.
