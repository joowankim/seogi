# Feature 02: 도구 사용 로깅 (`seogi hook post-tool`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

Claude Code가 도구를 성공적으로 호출했을 때 SQLite `tool_uses` 테이블에 기록한다.

**Ground Truth 연결:**
- **정량 측정**: 도구 사용 빈도, 종류, 소요 시간 등 프록시 지표의 원천 데이터를 자동 수집
- **동치 보장**: 하니스 변경 전후의 도구 사용 패턴 비교를 위한 기준선 데이터 축적

---

## 입력

| 항목 | 설명 |
|------|------|
| stdin | Claude Code PostToolUse 훅이 전달하는 JSON (아래 스키마 참조) |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용. 미설정 시 `~/.seogi/seogi.db`) |
| DB 상태 | Feature 01에서 초기화된 SQLite DB + `tool_uses` 테이블 |

### stdin JSON 스키마

```json
{
  "session_id": "string (필수)",
  "tool_name": "string (필수)",
  "tool_input": { ... },
  "tool_response": { ... },
  "tool_use_id": "string (필수)",
  "cwd": "string (필수)",
  "transcript_path": "string (필수)",
  "permission_mode": "string (필수)",
  "hook_event_name": "PostToolUse (필수)"
}
```

**프로젝트 정보 추출**: `cwd` 경로의 마지막 디렉토리명을 `project`로, `cwd` 전체를 `project_path`로 사용한다.

---

## 출력

| 항목 | 설명 |
|------|------|
| DB 변경 | `tool_uses` 테이블에 1행 INSERT |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 출력 |

### tool_uses 테이블 컬럼 매핑

| 컬럼 | 값 |
|------|------|
| `id` | UUID v4 hex |
| `session_id` | stdin `session_id` |
| `project` | `cwd` 경로의 마지막 디렉토리명 |
| `project_path` | stdin `cwd` |
| `tool_name` | stdin `tool_name` |
| `tool_input` | stdin `tool_input`을 JSON 문자열로 직렬화 |
| `duration_ms` | 0 (Feature 05에서 pre-tool 타이밍 구현 후 계산) |
| `timestamp` | 현재 시각 (밀리초 Unix timestamp) |

---

## 성공 시나리오

1. Claude Code가 도구를 성공적으로 호출하면 PostToolUse 훅이 실행된다.
2. `seogi hook post-tool`이 stdin에서 JSON을 읽는다.
3. JSON을 파싱하여 `ToolUse` 도메인 타입으로 변환한다.
4. SQLite `tool_uses` 테이블에 1행을 INSERT한다.
5. exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| stdin이 유효하지 않은 JSON | exit 1 + stderr에 에러 메시지 |
| `session_id` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| `tool_name` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| DB 접근 불가 (파일 잠김, 손상 등) | exit 1 + stderr에 에러 메시지 |

**참고**: DB 에러 시 `hook-errors.log` 기록 + macOS 알림(5분 쿨다운)은 Phase 1 후반 Feature에서 공통 에러 핸들러로 구현한다. 이번 Feature에서는 stderr 출력 + exit 1로 충분하다.

---

## 제약 조건

- **성능**: 훅 실행 시간 < 50ms (프로세스 기동 + JSON 파싱 + SQLite INSERT 포함)
- **호환성**: Claude Code PostToolUse 훅 프로토콜 준수 (stdin JSON)
- **멱등성 불필요**: 같은 도구 호출이 2번 기록되어도 문제 없음 (UUID로 구분)
- **duration_ms**: 이번 Feature에서는 항상 0. Feature 05(pre-tool)에서 실제 계산 구현

---

## 의존 Feature

- **Feature 01: DB 초기화** — `tool_uses` 테이블 스키마, `initialize_db` 함수

---

## 구현 범위

### 수직 슬라이스

```
domain/log.rs          ToolUse 타입 (Newtype 패턴)
    ↓
adapter/log_repo.rs    save_tool_use 함수
adapter/mapper.rs      ToolUse ↔ Row 변환
    ↓
workflow/log_tool.rs   Impureim Sandwich (파싱 → 저장)
    ↓
entrypoint/hooks/      post_tool.rs (stdin 파싱 → workflow 호출)
    mod.rs
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `domain/log.rs` | `ToolUse` 구조체 + factory 함수 |
| `adapter/log_repo.rs` | `save_tool_use(conn, tool_use)`, `find_by_session(conn, session_id)` |
| `adapter/mapper.rs` | `tool_use_to_row`, `row_to_tool_use` 변환 함수 |
| `workflow/log_tool.rs` | `run(conn, stdin_json) -> Result<()>` |
| `entrypoint/hooks/mod.rs` | 훅 모듈 선언 |
| `entrypoint/hooks/post_tool.rs` | `run() -> Result<()>` (stdin 읽기 → workflow 호출) |

### main.rs 변경

`hook` 서브커맨드 추가:
```
seogi hook post-tool    # stdin에서 JSON 읽어 tool_uses에 저장
```

### Cargo.toml 변경

없음 (Feature 01에서 필요한 의존성 모두 추가됨).

---

## QA 목록

### 기능 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | 유효한 JSON stdin 전달 시 `tool_uses` 테이블에 정확히 1행이 추가된다 | E2E: 바이너리 호출 → `SELECT COUNT(*) FROM tool_uses` == 1 |
| Q2 | 저장된 행의 `session_id`가 stdin JSON의 `session_id` 값과 일치한다 | E2E/통합: INSERT 후 `SELECT session_id` 비교 |
| Q3 | 저장된 행의 `tool_name`이 stdin JSON의 `tool_name` 값과 일치한다 | 통합: INSERT 후 `SELECT tool_name` 비교 |
| Q4 | `tool_input`이 `{"command":"ls"}` JSON일 때 저장된 `tool_input` 컬럼이 `"{\"command\":\"ls\"}"` 문자열이다 | 통합: INSERT 후 `SELECT tool_input` 비교 |
| Q5 | `cwd`가 `/Users/kim/projects/seogi`일 때 `project`가 `"seogi"`, `project_path`가 `"/Users/kim/projects/seogi"`이다 | 단위: `extract_project_from_cwd` 함수 테스트 |
| Q6 | `duration_ms`가 0으로 저장된다 (Feature 05 전까지) | 통합: INSERT 후 `SELECT duration_ms` == 0 |
| Q7 | `timestamp`가 밀리초 Unix timestamp이고 현재 시각 ±1초 이내이다 | 통합: INSERT 전후 시각 비교 |
| Q8 | `id`가 UUID v4 hex 형식(32자 hex)이다 | 통합: INSERT 후 `SELECT id`에 정규식 매칭 |
| Q9 | `find_by_session`으로 저장된 행을 조회하면 원본 `ToolUse`와 동일한 값이 반환된다 | 통합: save → find → 전체 비교 |

### 에러 처리

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q10 | 빈 stdin은 exit 1 + stderr에 에러 메시지 포함 + DB 행 수 변화 없음 | E2E: 빈 stdin → exit code 1 + stderr 비어있지 않음 + SELECT COUNT == 0 |
| Q11 | 잘못된 JSON(`{invalid}`) stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E: 잘못된 JSON → exit code 1 + stderr 비어있지 않음 |
| Q12 | `session_id` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E: `session_id` 없는 JSON → exit code 1 |
| Q13 | `tool_name` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E: `tool_name` 없는 JSON → exit code 1 |

### 타입 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q14 | `ToolUse`는 `Debug`, `Clone`, `PartialEq` derive를 갖는다 | 단위: `assert_eq!(tool_use.clone(), tool_use)` |
| Q15 | `ToolUse`의 각 필드를 getter로 읽을 수 있다 | 단위: 각 getter 반환값 검증 |

---

## Test Pyramid

### Unit Tests (domain 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_tool_use_creation` | Q14, Q15 | `ToolUse` 생성 + 각 필드 getter 검증 |
| `test_extract_project_from_cwd` | Q5 | `cwd` → `project` 추출 로직 |
| `test_extract_project_from_root_cwd` | Q5 | `cwd`가 `/`인 경우 처리 |

### Integration Tests (adapter + workflow 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_tool_use_inserts_row` | Q1, Q2, Q3, Q4, Q6, Q7, Q8 | `save_tool_use` → DB 행 검증 |
| `test_find_by_session_returns_saved` | Q9 | save → find → 전체 비교 |
| `test_find_by_session_empty` | Q9 | 존재하지 않는 세션 → 빈 Vec |
| `test_workflow_log_tool_run` | Q1 | workflow `run` → DB 행 추가 확인 |

### E2E Tests (바이너리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_post_tool_hook_saves_to_db` | Q1, Q2 | 유효한 JSON stdin → DB 저장 확인 |
| `test_post_tool_hook_empty_stdin` | Q10 | 빈 stdin → exit 1 |
| `test_post_tool_hook_invalid_json` | Q11 | 잘못된 JSON → exit 1 |
| `test_post_tool_hook_missing_session_id` | Q12 | session_id 누락 → exit 1 |
| `test_post_tool_hook_missing_tool_name` | Q13 | tool_name 누락 → exit 1 |

---

## 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 항목이 성공/실패 시나리오를 모두 커버
- [ ] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [ ] 의존하는 Feature 순서 명확
- [ ] 사용자 승인 완료
