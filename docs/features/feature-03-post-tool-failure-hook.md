# Feature 03: 도구 실패 로깅 (`seogi hook post-tool-failure`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

Claude Code가 도구 호출에 실패했을 때 SQLite `tool_failures` 테이블에 기록한다.

**Ground Truth 연결:**
- **정량 측정**: 도구 실패 빈도, 실패 도구 종류, 에러 패턴 등 프록시 지표의 원천 데이터를 자동 수집
- **동치 보장**: 하니스 변경 전후의 도구 실패율 비교를 위한 기준선 데이터 축적

---

## 입력

| 항목 | 설명 |
|------|------|
| stdin | Claude Code PostToolUseFailure 훅이 전달하는 JSON (아래 스키마 참조) |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용. 미설정 시 `~/.seogi/seogi.db`) |
| DB 상태 | Feature 01에서 초기화된 SQLite DB + `tool_failures` 테이블 |

### stdin JSON 스키마

```json
{
  "session_id": "string (필수)",
  "tool_name": "string (필수)",
  "tool_input": { ... },
  "error": "string (필수)",
  "tool_use_id": "string (필수)",
  "cwd": "string (필수)",
  "transcript_path": "string (필수)",
  "permission_mode": "string (필수)",
  "hook_event_name": "PostToolUseFailure (필수)"
}
```

**프로젝트 정보 추출**: `cwd` 경로의 마지막 디렉토리명을 `project`로, `cwd` 전체를 `project_path`로 사용한다 (Feature 02의 `extract_project_from_cwd` 재사용).

---

## 출력

| 항목 | 설명 |
|------|------|
| DB 변경 | `tool_failures` 테이블에 1행 INSERT |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 출력 |

### tool_failures 테이블 컬럼 매핑

| 컬럼 | 값 |
|------|------|
| `id` | UUID v4 hex |
| `session_id` | stdin `session_id` |
| `project` | `cwd` 경로의 마지막 디렉토리명 |
| `project_path` | stdin `cwd` |
| `tool_name` | stdin `tool_name` |
| `error` | stdin `error` |
| `timestamp` | 현재 시각 (밀리초 Unix timestamp) |

---

## 성공 시나리오

1. Claude Code가 도구 호출에 실패하면 PostToolUseFailure 훅이 실행된다.
2. `seogi hook post-tool-failure`이 stdin에서 JSON을 읽는다.
3. JSON을 파싱하여 `ToolFailure` 도메인 타입으로 변환한다.
4. SQLite `tool_failures` 테이블에 1행을 INSERT한다.
5. exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| stdin이 유효하지 않은 JSON | exit 1 + stderr에 에러 메시지 |
| `session_id` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| `tool_name` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| `error` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| DB 접근 불가 (파일 잠김, 손상 등) | exit 1 + stderr에 에러 메시지 |

---

## 제약 조건

- **성능**: 훅 실행 시간 < 50ms (프로세스 기동 + JSON 파싱 + SQLite INSERT 포함)
- **호환성**: Claude Code PostToolUseFailure 훅 프로토콜 준수 (stdin JSON)
- **멱등성 불필요**: 같은 실패가 2번 기록되어도 문제 없음 (UUID로 구분)
- **코드 재사용**: `extract_project_from_cwd`는 Feature 02의 `domain/log.rs`에서 재사용

---

## 의존 Feature

- **Feature 01: DB 초기화** — `tool_failures` 테이블 스키마, `initialize_db` 함수
- **Feature 02: 도구 사용 로깅** — `extract_project_from_cwd` 함수, 패턴 참조

---

## 구현 범위

### 수직 슬라이스

```
domain/log.rs          ToolFailure 타입 추가 (기존 파일)
    ↓
adapter/log_repo.rs    save_tool_failure 함수 추가 (기존 파일)
adapter/mapper.rs      tool_failure_from_row 함수 추가 (기존 파일)
    ↓
workflow/log_failure.rs  Impureim Sandwich (파싱 → 저장) [신규]
    ↓
entrypoint/hooks/        post_tool_failure.rs (stdin 파싱 → workflow 호출) [신규]
    mod.rs               PostToolFailure 추가
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `workflow/log_failure.rs` | `run(conn, stdin_json) -> Result<()>` |
| `entrypoint/hooks/post_tool_failure.rs` | `run() -> Result<()>` (stdin 읽기 → workflow 호출) |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `domain/log.rs` | `ToolFailure` 구조체 + factory + getters 추가 |
| `adapter/log_repo.rs` | `save_tool_failure`, `list_failures_by_session` 함수 추가 |
| `adapter/mapper.rs` | `tool_failure_from_row` 함수 추가 |
| `workflow/mod.rs` | `pub mod log_failure;` 추가 |
| `entrypoint/hooks/mod.rs` | `pub mod post_tool_failure;` 추가 |
| `main.rs` | `HookAction::PostToolFailure` 서브커맨드 추가 |

### Cargo.toml 변경

없음 (필요한 의존성 모두 존재).

---

## QA 목록

### 기능 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | 유효한 JSON stdin 전달 시 `tool_failures` 테이블에 정확히 1행이 추가된다 | E2E: 바이너리 호출 → `SELECT COUNT(*) FROM tool_failures` == 1 |
| Q2 | 저장된 행의 `session_id`가 stdin JSON의 `session_id` 값과 일치한다 | E2E/통합: INSERT 후 `SELECT session_id` 비교 |
| Q3 | 저장된 행의 `tool_name`이 stdin JSON의 `tool_name` 값과 일치한다 | 통합: INSERT 후 `SELECT tool_name` 비교 |
| Q4 | 저장된 행의 `error`가 stdin JSON의 `error` 값과 일치한다 | 통합: INSERT 후 `SELECT error` 비교 |
| Q5 | `cwd`가 `/Users/kim/projects/seogi`일 때 `project`가 `"seogi"`, `project_path`가 `"/Users/kim/projects/seogi"`이다 | 단위: `extract_project_from_cwd` 재사용 (Feature 02에서 이미 검증됨) |
| Q6 | `timestamp`가 밀리초 Unix timestamp이고 현재 시각 ±1초 이내이다 | 통합: INSERT 전후 시각 비교 |
| Q7 | `id`가 UUID v4 hex 형식(32자 hex)이다 | 통합: INSERT 후 `SELECT id`에 정규식 매칭 |
| Q8 | `list_failures_by_session`으로 저장된 행을 조회하면 원본 `ToolFailure`와 동일한 값이 반환된다 | 통합: save → find → 전체 비교 |

### 에러 처리

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q9 | 빈 stdin은 exit 1 + stderr에 에러 메시지 포함 + DB 행 수 변화 없음 | E2E: 빈 stdin → exit code 1 + stderr 비어있지 않음 + SELECT COUNT == 0 |
| Q10 | 잘못된 JSON(`{invalid}`) stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E: 잘못된 JSON → exit code 1 + stderr 비어있지 않음 |
| Q11 | `session_id` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E: `session_id` 없는 JSON → exit code 1 |
| Q12 | `tool_name` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E: `tool_name` 없는 JSON → exit code 1 |
| Q13 | `error` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E: `error` 없는 JSON → exit code 1 |

### 타입 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q14 | `ToolFailure`는 `Debug`, `Clone`, `PartialEq` derive를 갖는다 | 단위: `assert_eq!(tool_failure.clone(), tool_failure)` |
| Q15 | `ToolFailure`의 각 필드를 getter로 읽을 수 있다 | 단위: 각 getter 반환값 검증 |

---

## Test Pyramid

### Unit Tests (domain 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_tool_failure_creation` | Q14, Q15 | `ToolFailure` 생성 + 각 필드 getter 검증 |
| `test_tool_failure_display` | — | `Display` 형식 검증 `[{session_id}] {tool_name} FAILED ({id})` |

### Integration Tests (adapter + workflow 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_tool_failure_inserts_row` | Q1, Q2, Q3, Q4, Q6, Q7 | `save_tool_failure` → DB 행 검증 |
| `test_list_failures_by_session_returns_saved` | Q8 | save → find → 전체 비교 |
| `test_list_failures_by_session_empty` | Q8 | 존재하지 않는 세션 → 빈 Vec |
| `test_workflow_log_failure_run` | Q1 | workflow `run` → DB 행 추가 확인 |

### E2E Tests (바이너리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_post_tool_failure_hook_saves_to_db` | Q1, Q2 | 유효한 JSON stdin → DB 저장 확인 |
| `test_post_tool_failure_hook_empty_stdin` | Q9 | 빈 stdin → exit 1 |
| `test_post_tool_failure_hook_invalid_json` | Q10 | 잘못된 JSON → exit 1 |
| `test_post_tool_failure_hook_missing_session_id` | Q11 | session_id 누락 → exit 1 |
| `test_post_tool_failure_hook_missing_tool_name` | Q12 | tool_name 누락 → exit 1 |
| `test_post_tool_failure_hook_missing_error` | Q13 | error 누락 → exit 1 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [ ] 사용자 승인 완료
