#!/usr/bin/env bash
set -euo pipefail

# 세션 요약 분석기
# 사용법: session-summary.sh <project_name> <session_id>
# 현재 세션의 raw JSONL 로그에서 프록시 지표 10개를 계산하여 metrics JSONL에 저장

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/logger.sh"

PROJECT_NAME="${1:?project_name required}"
SESSION_ID="${2:?session_id required}"

load_config

PROJECT_LOG_DIR="$LOG_DIR/$PROJECT_NAME"

# 해당 세션의 로그 엔트리 추출
# jq 스트림 파싱으로 pretty-printed/compact 모두 처리
LOG_FILES=$(find "$PROJECT_LOG_DIR" -name '*.jsonl' -not -path '*/metrics/*' 2>/dev/null)

if [[ -z "$LOG_FILES" ]]; then
  exit 0
fi

# 모든 로그 파일에서 해당 세션 엔트리만 추출 (compact 한 줄로)
SESSION_LOGS=$(echo "$LOG_FILES" | while IFS= read -r f; do
  jq -c "select(.sessionId == \"$SESSION_ID\")" "$f" 2>/dev/null
done)

if [[ -z "$SESSION_LOGS" ]]; then
  exit 0
fi

# jq로 모든 지표를 한 번에 계산
METRICS=$(echo "$SESSION_LOGS" | jq -s --arg sid "$SESSION_ID" --arg proj "$PROJECT_NAME" '

  # 타임스탬프 정렬 후 도구 호출만 필터
  (sort_by(.timestamp)) as $sorted |
  [$sorted[] | select(.tool != null)] as $tool_calls |

  # 1. read_before_edit_ratio: 첫 Edit/Write 전 Read/Grep/Glob 호출 수
  (
    [$tool_calls[].tool.name] as $names |
    ([$names | to_entries[] | select(.value == "Edit" or .value == "Write") | .key] | if length > 0 then min else ($names | length) end) as $first_edit_idx |
    [$names[:$first_edit_idx][] | select(. == "Read" or . == "Grep" or . == "Glob")] | length
  ) as $read_before_edit |

  # 2. doom_loop_count: 동일 파일 Edit 5회 이상 발생 횟수
  (
    [
      $tool_calls[] |
      select(.tool.name == "Edit") |
      .tool.input.file_path // "unknown"
    ] |
    group_by(.) |
    [.[] | select(length >= 5)] |
    length
  ) as $doom_loop_count |

  # 3. test_invoked: Bash에서 test/vitest/playwright/jest/pytest 호출 여부
  (
    [
      $tool_calls[] |
      select(.tool.name == "Bash") |
      .tool.input.command // ""
    ] |
    any(test("\\b(test|vitest|playwright|jest|pytest|mocha|karma)\\b"; "i"))
  ) as $test_invoked |

  # 4. build_invoked: Bash에서 build/tsc 호출 여부
  (
    [
      $tool_calls[] |
      select(.tool.name == "Bash") |
      .tool.input.command // ""
    ] |
    any(test("\\b(build|tsc|webpack|vite build|esbuild|rollup)\\b"; "i"))
  ) as $build_invoked |

  # 5. tool_call_count: 총 도구 호출 수
  ($tool_calls | length) as $tool_call_count |

  # 6. session_duration_ms: 첫 도구 ~ 마지막 도구 시간 차
  (
    if ($tool_calls | length) > 1 then
      (($tool_calls | last | .timestamp) as $end |
       ($tool_calls | first | .timestamp) as $start |
       (($end | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601) - ($start | sub("\\.[0-9]+Z$"; "Z") | fromdateiso8601)) * 1000)
    else 0 end
  ) as $session_duration_ms |

  # 7. edit_files: Edit/Write한 고유 파일 목록
  (
    [
      $tool_calls[] |
      select(.tool.name == "Edit" or .tool.name == "Write") |
      .tool.input.file_path // "unknown"
    ] | unique | [.[] | select(. != "unknown")]
  ) as $edit_files |

  # 8. lint_invoked: Bash에서 lint/eslint/prettier/ruff/biome 호출 여부
  (
    [$tool_calls[] | select(.tool.name == "Bash") | .tool.input.command // ""] |
    any(test("\\b(lint|eslint|prettier|ruff|biome)\\b"; "i"))
  ) as $lint_invoked |

  # 9. typecheck_invoked: Bash에서 tsc --noEmit/mypy/pyright 호출 여부
  (
    [$tool_calls[] | select(.tool.name == "Bash") | .tool.input.command // ""] |
    any(test("\\b(tsc\\s+--noEmit|mypy|pyright)\\b"; "i"))
  ) as $typecheck_invoked |

  # 10. bash_error_rate: Bash 실패 비율
  (
    [$sorted[] | select(.tool != null and .tool.name == "Bash")] as $all_bash |
    [$all_bash[] | select(.tool.failed == true)] as $failed_bash |
    if ($all_bash | length) > 0 then
      (($failed_bash | length) / ($all_bash | length) * 100 | round / 100)
    else 0 end
  ) as $bash_error_rate |

  # 결과 조립
  {
    timestamp: (now | strftime("%Y-%m-%dT%H:%M:%S.000Z")),
    sessionId: $sid,
    project: $proj,
    metrics: {
      read_before_edit_ratio: $read_before_edit,
      doom_loop_count: $doom_loop_count,
      test_invoked: $test_invoked,
      build_invoked: $build_invoked,
      tool_call_count: $tool_call_count,
      session_duration_ms: $session_duration_ms,
      edit_files: $edit_files,
      lint_invoked: $lint_invoked,
      typecheck_invoked: $typecheck_invoked,
      bash_error_rate: $bash_error_rate
    }
  }
')

# metrics 디렉토리에 저장
METRICS_DIR="$PROJECT_LOG_DIR/metrics"
mkdir -p "$METRICS_DIR"

DATE_STR=$(date +%Y-%m-%d)
METRICS_FILE="$METRICS_DIR/$DATE_STR.jsonl"

echo "$METRICS" | jq -c '.' >> "$METRICS_FILE"
