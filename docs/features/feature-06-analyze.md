# Feature 06: 세션 분석 (`seogi analyze`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

세션 동안 수집된 도구 사용/실패 데이터를 SQLite에서 읽어 프록시 지표 10개를 계산하고 stdout에 JSON으로 출력한다.

**Ground Truth 연결:**
- **정량 측정**: 도구 사용 패턴(read-before-edit ratio, doom loop), 품질 관행(test/lint/typecheck 호출), 세션 특성(도구 수, 소요 시간, 실패율)을 자동 집계하여 하니스 성능의 핵심 프록시 지표를 산출
- **동치 보장**: 하니스 변경 전후의 10개 지표를 통계적으로 비교하여 동등한 업무 효율이 유지되는지 검증

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | `seogi analyze <session_id>` |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용) |
| DB 상태 | `tool_uses`, `tool_failures` 테이블에 해당 세션의 데이터가 존재 |

### 데이터 소스 매핑

기존 `analyzers/session_summary.rs`는 `LogEntry`(JSONL)에서 계산했다. 새 구현은 SQLite의 `ToolUse`/`ToolFailure`에서 동일 지표를 계산한다.

| LogEntry 필드 | ToolUse 필드 | 용도 |
|---|---|---|
| `tool.name` | `tool_name` | 도구 종류 판별 |
| `tool.input.command` | `tool_input` (JSON 내 `command` 키) | Bash 명령어 추출 |
| `tool.input.file_path` | `tool_input` (JSON 내 `file_path` 키) | 편집 파일 경로 추출 |
| `tool.failed` | `tool_failures` 테이블 존재 여부 | 실패 판별 |
| `timestamp` (RFC3339) | `timestamp` (밀리초 Unix) | 세션 기간 계산 |

---

## 출력

| 항목 | 설명 |
|------|------|
| stdout | 10개 지표를 JSON 형식으로 출력 |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 |

DB에 저장하지 않는다. 원시 데이터(`tool_uses`, `tool_failures`)가 있으면 언제든 재계산 가능하므로, 결과 캐싱은 실제 성능 필요가 발생할 때 추가한다.

### stdout JSON 형식

```json
{
  "session_id": "sess-1",
  "read_before_edit_ratio": 3,
  "doom_loop_count": 0,
  "test_invoked": true,
  "build_invoked": false,
  "lint_invoked": true,
  "typecheck_invoked": false,
  "tool_call_count": 42,
  "session_duration_ms": 300000,
  "edit_files": ["src/main.rs", "src/lib.rs"],
  "bash_error_rate": 0.1
}
```

---

## 10개 지표 계산 규칙

기존 `analyzers/session_summary.rs`의 로직을 이식한다. 데이터 소스만 JSONL → SQLite로 변경.

### 1. read_before_edit_ratio (u32)
첫 Edit/Write `tool_uses` 행 이전의 Read/Grep/Glob 행 수. Edit이 없으면 전체 Read/Grep/Glob 수.

### 2. doom_loop_count (u32)
`tool_input` JSON에서 `file_path`를 추출하여 동일 파일에 대한 Edit 5회 이상인 파일 수.

### 3. test_invoked (bool)
`tool_name`=="Bash"인 행의 `tool_input` JSON에서 `command`를 추출, 정규식 `(?i)\b(test|vitest|playwright|jest|pytest|mocha|karma)\b` 매칭.

### 4. build_invoked (bool)
정규식 `(?i)\b(build|tsc|webpack|vite build|esbuild|rollup)\b`.

### 5. lint_invoked (bool)
정규식 `(?i)\b(lint|eslint|prettier|ruff|biome)\b`.

### 6. typecheck_invoked (bool)
정규식 `(?i)\b(tsc\s+--noEmit|mypy|pyright)\b`.

### 7. tool_call_count (u32)
해당 세션의 `tool_uses` 행 수.

### 8. session_duration_ms (i64)
해당 세션의 `tool_uses` 중 최소 timestamp와 최대 timestamp의 차이. 행이 1개 이하이면 0.

### 9. edit_files (Vec<String>)
Edit/Write `tool_uses`의 `tool_input` JSON에서 `file_path` 추출, 중복 제거 후 알파벳 정렬.

### 10. bash_error_rate (f64)
해당 세션의 Bash `tool_failures` 수 / 해당 세션의 Bash `tool_uses` 수. Bash 호출이 없으면 0.0.

---

## 성공 시나리오

1. `seogi analyze sess-1`이 실행된다.
2. DB에서 `tool_uses` 행을 `session_id`로 조회한다 (timestamp 순 정렬).
3. DB에서 `tool_failures` 행을 `session_id`로 조회한다 (bash_error_rate 계산용).
4. 10개 지표를 계산한다 (순수 함수).
5. 결과를 JSON으로 stdout에 출력한다.
6. exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| 해당 세션의 tool_uses가 0건 | 지표를 모두 기본값(0/false/0.0/[])으로 출력. 에러가 아님 |
| `tool_input`이 유효하지 않은 JSON이거나 `command`/`file_path` 키가 없음 | 해당 행은 해당 지표 계산에서 제외. 모든 행이 제외되면 빈 세션과 동일한 기본값 출력. 에러가 아님 |
| DB 접근 불가 | exit 1 + stderr에 에러 메시지 |
| CLI 인자 누락 (session_id) | clap이 자동 에러 처리 |

---

## 제약 조건

- **성능**: 합리적인 시간(< 1초) 내 완료 목표
- **호환성**: 기존 `analyzers/session_summary.rs`와 동일한 지표 계산 결과를 산출해야 함
- **순수성**: `domain/metrics.rs`의 `calculate` 함수는 순수 함수. I/O 없음

---

## 의존 Feature

- **Feature 01: DB 초기화** — `initialize_db` 함수
- **Feature 02: 도구 사용 로깅** — `tool_uses` 데이터, `list_by_session` 함수
- **Feature 03: 도구 실패 로깅** — `tool_failures` 데이터, `list_failures_by_session` 함수

---

## 구현 범위

### 수직 슬라이스

기존 `analyzers/session_summary.rs`의 순수 계산 함수를 `domain/metrics.rs`로 이식한다. 데이터 소스를 `ToolUse`/`ToolFailure`로 변경한다.

```
domain/metrics.rs           calculate 순수 함수 + SessionMetrics 타입 [신규]
    ↓
workflow/analyze.rs          Impureim Sandwich: load → calculate → output [신규]
    ↓
entrypoint/app/analyze.rs    seogi analyze <session_id> [신규]
    ↓
main.rs                      Analyze 서브커맨드 연결 변경 [수정]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `domain/metrics.rs` | `SessionMetrics` 타입 + `calculate(tool_uses, tool_failures) -> SessionMetrics` |
| `workflow/analyze.rs` (신규) | `run(conn, session_id) -> Result<SessionMetrics>` |
| `entrypoint/app/analyze.rs` | CLI 진입점 |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `domain/mod.rs` | `pub mod metrics;` 추가 |
| `workflow/mod.rs` | `pub mod analyze;` 추가 |
| `main.rs` | `Analyze` 서브커맨드를 새 workflow로 연결 |

### 재사용 모듈 (변경 없음)

| 파일 | 재사용 함수 |
|------|-----------|
| `adapter/log_repo.rs` | `list_by_session(conn, session_id)` — tool_uses 조회 |
| `adapter/log_repo.rs` | `list_failures_by_session(conn, session_id)` — tool_failures 조회 |
| `entrypoint/hooks/mod.rs` | `db_path()` — DB 경로 결정 |

### 기존 코드 공존

`analyzers/session_summary.rs`, `commands/analyze.rs` 등 JSONL 기반 기존 코드는 이번 Feature에서 제거하지 않는다. Phase 1 완료 후 별도 정리.

---

## QA 목록

### 지표 계산 (순수 함수)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | Read 3회 → Edit 1회 순서의 tool_uses에서 `read_before_edit_ratio`가 3이다 | 단위 |
| Q2 | Edit이 없는 tool_uses에서 `read_before_edit_ratio`가 전체 Read/Grep/Glob 수이다 | 단위 |
| Q3 | 같은 파일 Edit 6회의 tool_uses에서 `doom_loop_count`가 1이다 | 단위 |
| Q4 | 같은 파일 Edit 4회의 tool_uses에서 `doom_loop_count`가 0이다 | 단위 |
| Q5 | Bash command에 "pytest" 포함 시 `test_invoked`가 true이다 | 단위 |
| Q6 | Bash command에 "ls" 포함 시 `test_invoked`가 false이다 | 단위 |
| Q7 | Bash command에 "webpack" 포함 시 `build_invoked`가 true이다 | 단위 |
| Q8 | Bash command에 "eslint" 포함 시 `lint_invoked`가 true이다 | 단위 |
| Q9 | Bash command에 "tsc --noEmit" 포함 시 `typecheck_invoked`가 true이다 | 단위 |
| Q10 | tool_uses 5건의 세션에서 `tool_call_count`가 5이다 | 단위 |
| Q11 | 첫 timestamp 1000, 마지막 5000인 tool_uses에서 `session_duration_ms`가 4000이다 | 단위 |
| Q12 | tool_uses 1건의 세션에서 `session_duration_ms`가 0이다 | 단위 |
| Q13 | Edit("a.rs") 2회 + Edit("b.rs") 1회에서 `edit_files`가 `["a.rs", "b.rs"]`이다 | 단위 |
| Q14 | Bash tool_uses 10건 중 tool_failures에 Bash 2건 있으면 `bash_error_rate`가 0.2이다 | 단위 |
| Q15 | Bash 호출이 없으면 `bash_error_rate`가 0.0이다 | 단위 |
| Q16 | 빈 tool_uses에서 모든 지표가 기본값(0/false/0.0/[])이다 | 단위 |
| Q17 | `tool_input`에 `command` 키가 없는 Bash 행은 test/build/lint/typecheck 판별에서 제외된다 | 단위 |

### workflow

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q18 | `tool_uses` 3건(Read, Read, Edit) + `tool_failures` 0건 삽입 후 `workflow::analyze::run` 호출 시, `read_before_edit_ratio`가 2이고 `tool_call_count`가 3인 `SessionMetrics`가 반환된다 | 통합 |
| Q19 | tool_uses가 비어있는 세션에 대해 `workflow::analyze::run`을 호출하면 기본값 `SessionMetrics`가 반환된다 | 통합 |

### E2E

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q20 | `seogi analyze sess-1` 실행 시 stdout에 유효한 JSON이 출력되고 exit 0이다 | E2E |
| Q21 | stdout JSON의 `tool_call_count`가 DB에 삽입한 tool_uses 수와 일치한다 | E2E |
| Q22 | 인자 없이 `seogi analyze` 실행 시 exit code != 0이다 | E2E |
| Q23 | 존재하지 않는 DB 경로로 `seogi analyze sess-1` 실행 시 exit 1이고 stderr에 에러 메시지가 출력된다 | E2E |

---

## Test Pyramid

### Unit Tests (domain/metrics.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_read_before_edit_ratio` | Q1 | Read 3 → Edit 1 → 3 |
| `test_read_before_edit_no_edit` | Q2 | Edit 없음 → 전체 Read 수 |
| `test_doom_loop_above_threshold` | Q3 | 같은 파일 6회 → 1 |
| `test_doom_loop_below_threshold` | Q4 | 같은 파일 4회 → 0 |
| `test_test_invoked_true` | Q5 | pytest → true |
| `test_test_invoked_false` | Q6 | ls → false |
| `test_build_invoked` | Q7 | webpack → true |
| `test_lint_invoked` | Q8 | eslint → true |
| `test_typecheck_invoked` | Q9 | tsc --noEmit → true |
| `test_tool_call_count` | Q10 | 5건 → 5 |
| `test_session_duration` | Q11 | 1000~5000 → 4000 |
| `test_session_duration_single` | Q12 | 1건 → 0 |
| `test_edit_files` | Q13 | 중복 제거 + 정렬 |
| `test_bash_error_rate` | Q14 | 10건 중 2실패 → 0.2 |
| `test_bash_error_rate_no_bash` | Q15 | 없음 → 0.0 |
| `test_empty_session` | Q16 | 모든 기본값 |
| `test_bash_missing_command_key` | Q17 | command 키 없음 → 제외 |

### Integration Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_workflow_analyze_run` | Q18 | tool_uses 삽입 → workflow run → 지표 검증 |
| `test_workflow_analyze_empty_session` | Q19 | 빈 세션 → 기본값 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_analyze_command_outputs_json` | Q20, Q21 | 바이너리 호출 → stdout JSON 확인 |
| `test_analyze_command_no_args` | Q22 | 인자 없음 → exit != 0 |
| `test_analyze_command_bad_db` | Q23 | 잘못된 DB 경로 → exit 1 + stderr |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과 (FAIL 3건 수정 완료)
- [ ] 사용자 승인 완료
