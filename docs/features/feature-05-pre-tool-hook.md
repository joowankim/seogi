# Feature 05: 도구 호출 시작 시간 기록 (`seogi hook pre-tool`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

Claude Code가 도구를 호출하기 직전에 시작 시각을 임시 파일에 기록하고, 호출 완료 후 post-tool 훅이 이를 읽어 `duration_ms`를 계산한다.

**Ground Truth 연결:**
- **정량 측정**: 도구 호출 소요 시간(`duration_ms`)은 하니스 성능의 핵심 프록시 지표. 현재 post-tool에서 0으로 하드코딩되어 있어 실제 측정이 불가능하며, 이를 해소한다
- **동치 보장**: 하니스 변경 전후의 도구 호출 소요 시간 분포를 비교하여 성능 회귀를 감지할 수 있는 기준선 데이터를 축적한다

---

## 입력

### pre-tool 훅

| 항목 | 설명 |
|------|------|
| stdin | Claude Code PreToolUse 훅이 전달하는 JSON (아래 스키마 참조) |
| 환경변수 | `SEOGI_TIMING_DIR` (선택, 테스트용. 미설정 시 `${TMPDIR:-/tmp}/seogi`) |

### post-tool 훅 (기존 수정)

| 항목 | 설명 |
|------|------|
| stdin | 기존과 동일 (PostToolUse JSON) |
| 파일 시스템 | pre-tool이 저장한 시작 시간 파일 |

### PreToolUse stdin JSON 스키마

```json
{
  "session_id": "string (필수)",
  "tool_name": "string (필수)",
  "tool_input": { ... },
  "tool_use_id": "string (필수)",
  "cwd": "string (필수)",
  "transcript_path": "string (필수)",
  "permission_mode": "string (필수)",
  "hook_event_name": "PreToolUse (필수)"
}
```

---

## 출력

### pre-tool 훅

| 항목 | 설명 |
|------|------|
| 파일 쓰기 | `{timing_dir}/{tool_use_id}_start` 파일에 밀리초 Unix timestamp 문자열 저장 |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 출력 |

### post-tool 훅 (변경)

| 항목 | 변경 내용 |
|------|-----------|
| `duration_ms` | 시작 시간 파일이 존재하면 `현재시각 - 시작시각`(ms). 없으면 기존대로 0 |
| 파일 삭제 | 시작 시간 파일 읽기 후 삭제 (cleanup) |

### 타이밍 파일 경로

```
{SEOGI_TIMING_DIR 또는 ${TMPDIR:-/tmp}/seogi}/{tool_use_id}_start
```

파일 내용: 밀리초 Unix timestamp 문자열 (예: `1713000000123`)

---

## 성공 시나리오

### pre-tool 훅

1. Claude Code가 도구를 호출하기 전에 PreToolUse 훅이 실행된다.
2. `seogi hook pre-tool`이 stdin에서 JSON을 읽는다.
3. 타이밍 디렉토리가 없으면 자동 생성한다.
4. `tool_use_id`를 추출하고 현재 시각(밀리초)을 파일에 기록한다.
5. exit 0으로 종료한다.

### post-tool 훅 (수정된 동작)

1. 기존 동작과 동일하게 stdin에서 JSON을 읽는다.
2. `tool_use_id`로 시작 시간 파일을 조회한다.
3. 파일이 존재하면: 시작 시각을 읽고 `duration_ms = 현재시각 - 시작시각`을 계산한 뒤 파일을 삭제한다.
4. 파일이 없으면: `duration_ms = 0` (하위 호환).
5. `ToolUse`를 생성하여 DB에 저장한다.

---

## 실패 시나리오

### pre-tool 훅

| 조건 | 처리 |
|------|------|
| stdin이 유효하지 않은 JSON | exit 1 + stderr에 에러 메시지 |
| `tool_use_id` 필드 누락 | serde 역직렬화 실패로 일괄 처리 — exit 1 + stderr |
| 타이밍 디렉토리 쓰기 실패 | exit 1 + stderr에 에러 메시지 |

### post-tool 훅 (변경 관련)

| 조건 | 처리 |
|------|------|
| 시작 시간 파일 없음 | `duration_ms = 0`으로 fallback (에러가 아님) |
| 시작 시간 파일 내용 파싱 실패 | `duration_ms = 0`으로 fallback (에러가 아님) |
| 시작 시간 파일 삭제 실패 | 무시 (best-effort cleanup) |

---

## 제약 조건

- **성능**: 훅 실행 시간 < 50ms. 파일 I/O는 tmpfs/ramfs 위의 단일 파일 읽기/쓰기이므로 충분히 빠름
- **호환성**: pre-tool 없이 post-tool만 호출되는 기존 시나리오에서 `duration_ms = 0`으로 동작 유지 (하위 호환)
- **정리**: 시작 시간 파일은 post-tool에서 삭제. pre-tool만 호출되고 post-tool이 호출되지 않는 경우(도구 호출 취소 등) 파일이 남지만 문제없음 (OS tmpdir 정리에 의존)
- **동시성**: `tool_use_id`가 고유하므로 여러 도구 호출이 병렬로 실행되어도 파일 충돌 없음

---

## 의존 Feature

- **Feature 01: DB 초기화** — `initialize_db` 함수
- **Feature 02: 도구 사용 로깅** — `workflow/log_tool.rs` 수정 대상, `ToolUse.duration: Ms` 필드
- **값객체 리팩토링** — `Ms` 값객체 도입 완료 (`domain/value.rs`)

---

## 구현 범위

### 수직 슬라이스

pre-tool 훅은 "시작 시간을 임시 파일에 기록"하는 I/O 전용 동작이므로 순수 도메인 로직이 없다. domain 계층 없이 adapter → entrypoint로 구성한다. duration 계산은 기존 `workflow/log_tool.rs`의 Impureim Sandwich 내에서 수행한다.

기존 `PostToolUse` JSON에 `tool_use_id`가 이미 포함되어 있으므로 `HookInput`에 필드 추가 시 역호환 문제 없음.

```
adapter/timing.rs         시작 시간 저장/조회/삭제 [신규]
    ↓
workflow/log_tool.rs      duration_ms 계산 로직 추가 [수정]
    ↓
entrypoint/hooks/         pre_tool.rs [신규]
    mod.rs
    ↓
main.rs                   PreTool 서브커맨드 추가 [수정]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `adapter/timing.rs` | `save_start_time(dir, tool_use_id)`, `read_and_remove_start_time(dir, tool_use_id) -> Option<i64>`, `timing_dir() -> PathBuf` |
| `entrypoint/hooks/pre_tool.rs` | `run() -> Result<()>` |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `adapter/mod.rs` | `pub mod timing;` 추가 |
| `workflow/log_tool.rs` | `HookInput`에 `tool_use_id` 추가, timing 조회로 `duration_ms` 계산 |
| `entrypoint/hooks/mod.rs` | `pub mod pre_tool;` 추가 |
| `main.rs` | `HookAction::PreTool` 서브커맨드 추가 |

### Cargo.toml 변경

없음.

---

## QA 목록

### 기능 검증 — pre-tool

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | 유효한 PreToolUse JSON stdin 전달 시 타이밍 디렉토리에 `{tool_use_id}_start` 파일이 생성되고 exit 0으로 종료된다 | E2E: 바이너리 호출 → 파일 존재 확인 + exit 0 |
| Q2 | 생성된 파일의 내용이 밀리초 Unix timestamp 문자열이고 현재 시각 ±1초 이내이다 | E2E: 파일 읽기 → 파싱 → 범위 검증 |

### 기능 검증 — duration 계산

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q3 | pre-tool → post-tool 순서로 호출 시, DB에 저장된 `duration_ms`가 0보다 크다 | E2E: pre-tool 호출 → post-tool 호출 → `SELECT duration_ms` > 0 |
| Q4 | pre-tool 없이 post-tool만 호출 시, `duration_ms`가 0이다 (하위 호환) | E2E: post-tool만 호출 → `SELECT duration_ms` == 0 |
| Q5 | post-tool 호출 후 시작 시간 파일이 삭제된다 | E2E: pre-tool → post-tool → 파일 부재 확인 |

### 기능 검증 — adapter

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q6 | `save_start_time`이 지정 디렉토리에 `{tool_use_id}_start` 파일을 생성하고 밀리초 timestamp를 기록한다 | 통합: 함수 호출 → 파일 읽기 → 파싱 검증 |
| Q7 | `read_and_remove_start_time`이 존재하는 파일에서 밀리초 timestamp를 읽고 파일을 삭제한다 | 통합: 파일 생성 → 함수 호출 → 값 검증 + 파일 부재 확인 |
| Q8 | `read_and_remove_start_time`이 파일이 없을 때 `None`을 반환한다 | 통합: 빈 디렉토리 → 함수 호출 → None |
| Q9 | `read_and_remove_start_time`이 파일 내용이 숫자가 아닐 때 `None`을 반환한다 | 통합: 잘못된 내용 파일 → 함수 호출 → None |

### 에러 처리

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q10 | `pre-tool` 빈 stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q11 | `pre-tool` 잘못된 JSON(`{invalid}`) stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q12 | `pre-tool`에서 `tool_use_id` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E |

---

## Test Pyramid

### Unit Tests

없음 (순수 계산 함수가 없음. 모든 로직이 I/O 포함).

### Integration Tests (adapter + workflow 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_start_time_creates_file` | Q6 | 파일 생성 + 내용 검증 |
| `test_read_and_remove_start_time_returns_value` | Q7 | 값 반환 + 파일 삭제 확인 |
| `test_read_and_remove_start_time_missing_file` | Q8 | 파일 없음 → None |
| `test_read_and_remove_start_time_invalid_content` | Q9 | 잘못된 내용 → None |

### E2E Tests (바이너리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_pre_tool_hook_creates_timing_file` | Q1, Q2 | 파일 생성 + timestamp 검증 |
| `test_pre_tool_then_post_tool_duration` | Q3, Q5 | pre→post 순서 호출 → duration > 0 + 파일 삭제 |
| `test_post_tool_without_pre_tool_fallback` | Q4 | post만 호출 → duration == 0 |
| `test_pre_tool_hook_empty_stdin` | Q10 | 빈 stdin → exit 1 |
| `test_pre_tool_hook_invalid_json` | Q11 | 잘못된 JSON → exit 1 |
| `test_pre_tool_hook_missing_tool_use_id` | Q12 | tool_use_id 누락 → exit 1 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과
- [x] 사용자 승인 완료
