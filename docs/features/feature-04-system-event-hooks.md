# Feature 04: 시스템 이벤트 로깅 (`seogi hook notification`, `seogi hook stop`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

Claude Code의 Notification과 Stop 훅을 통해 알림 발생 및 세션 종료 이벤트를 SQLite `system_events` 테이블에 기록한다.

**Ground Truth 연결:**
- **정량 측정**: 알림 빈도(permission_prompt, idle_prompt 등), 세션 종료 사유(end_turn, max_tokens 등) 패턴 데이터를 자동 수집하여 하니스 품질 지표의 원천 데이터로 활용
- **동치 보장**: 하니스 변경 전후의 알림 발생 패턴과 세션 종료 사유 분포를 비교하기 위한 기준선 데이터 축적

---

## 입력

| 항목 | 설명 |
|------|------|
| stdin | Claude Code Notification 또는 Stop 훅이 전달하는 JSON (아래 스키마 참조) |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용. 미설정 시 `~/.seogi/seogi.db`) |
| DB 상태 | Feature 01에서 초기화된 SQLite DB + `system_events` 테이블 |

### Notification stdin JSON 스키마

```json
{
  "session_id": "string (필수)",
  "message": "string (필수)",
  "notification_type": "string (필수, permission_prompt|idle_prompt|auth_success|elicitation_dialog)",
  "title": "string (선택)",
  "cwd": "string (필수)",
  "transcript_path": "string (필수)",
  "hook_event_name": "Notification (필수)"
}
```

### Stop stdin JSON 스키마

```json
{
  "session_id": "string (필수)",
  "stop_reason": "string (필수, end_turn|max_tokens|stop_sequence)",
  "cwd": "string (필수)",
  "transcript_path": "string (필수)",
  "permission_mode": "string (필수)",
  "hook_event_name": "Stop (필수)"
}
```

**프로젝트 정보 추출**: `cwd` 경로의 마지막 디렉토리명을 `project`로, `cwd` 전체를 `project_path`로 사용한다 (Feature 02의 `extract_project_from_cwd` 재사용).

---

## 출력

| 항목 | 설명 |
|------|------|
| DB 변경 | `system_events` 테이블에 1행 INSERT |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 출력 |

### system_events 테이블 컬럼 매핑

| 컬럼 | Notification 값 | Stop 값 |
|------|-----------------|---------|
| `id` | UUID v4 hex | UUID v4 hex |
| `session_id` | stdin `session_id` | stdin `session_id` |
| `project` | `cwd` 마지막 디렉토리명 | `cwd` 마지막 디렉토리명 |
| `project_path` | stdin `cwd` | stdin `cwd` |
| `event_type` | `"Notification"` | `"Stop"` |
| `content` | stdin `message` | stdin `stop_reason` |
| `timestamp` | 현재 시각 (밀리초 Unix timestamp) | 현재 시각 (밀리초 Unix timestamp) |

---

## 성공 시나리오

### Notification 훅

1. Claude Code가 알림을 발생시키면 Notification 훅이 실행된다.
2. `seogi hook notification`이 stdin에서 JSON을 읽는다.
3. JSON을 파싱하여 `SystemEvent` 도메인 타입으로 변환한다 (`event_type`="Notification", `content`=`message`).
4. SQLite `system_events` 테이블에 1행을 INSERT한다.
5. exit 0으로 종료한다.

### Stop 훅

1. Claude Code 세션이 종료되면 Stop 훅이 실행된다.
2. `seogi hook stop`이 stdin에서 JSON을 읽는다.
3. JSON을 파싱하여 `SystemEvent` 도메인 타입으로 변환한다 (`event_type`="Stop", `content`=`stop_reason`).
4. SQLite `system_events` 테이블에 1행을 INSERT한다.
5. exit 0으로 종료한다.

> **Note**: Stop 훅에서 `seogi analyze`를 백그라운드로 호출하는 기능은 Feature 06에서 구현한다. 이번 Feature에서는 이벤트 기록만 수행한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| stdin이 유효하지 않은 JSON | exit 1 + stderr에 에러 메시지 |
| `session_id` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| Notification에서 `message` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| Stop에서 `stop_reason` 필드 누락 | exit 1 + stderr에 에러 메시지 |
| `cwd` 등 기타 필수 필드 누락 | serde 역직렬화 실패로 일괄 처리 — exit 1 + stderr에 에러 메시지 |
| DB 접근 불가 (파일 잠김, 손상 등) | exit 1 + stderr에 에러 메시지 |

**참고**: `hook_event_name` 필드는 검증하지 않는다. 각 서브커맨드(`notification`, `stop`)가 이미 이벤트 유형을 결정하므로, JSON 내 `hook_event_name` 값은 참조하지 않는다. `cwd`, `transcript_path` 등 나머지 필수 필드 누락은 serde 역직렬화 실패로 일괄 처리되며, 잘못된 JSON 케이스(Q13, Q17)와 동일한 에러 경로를 탄다.

---

## 제약 조건

- **성능**: 훅 실행 시간 < 50ms (프로세스 기동 + JSON 파싱 + SQLite INSERT 포함)
- **호환성**: Claude Code Notification/Stop 훅 프로토콜 준수 (stdin JSON)
- **멱등성 불필요**: 같은 이벤트가 2번 기록되어도 문제 없음 (UUID로 구분)
- **코드 재사용**: `extract_project_from_cwd`는 Feature 02의 `domain/log.rs`에서 재사용. `db_path()`는 Feature 03의 `hooks/mod.rs`에서 재사용.

---

## 의존 Feature

- **Feature 01: DB 초기화** — `system_events` 테이블 스키마, `initialize_db` 함수
- **Feature 02: 도구 사용 로깅** — `extract_project_from_cwd` 함수, 패턴 참조
- **Feature 03: 도구 실패 로깅** — `db_path()` 공통 함수, 패턴 참조

---

## 구현 범위

### 수직 슬라이스

```
domain/log.rs             SystemEvent 타입 추가 (기존 파일)
    ↓
adapter/log_repo.rs       save_system_event 함수 추가 (기존 파일)
adapter/mapper.rs         system_event_from_row 함수 추가 (기존 파일)
    ↓
workflow/log_system.rs    Impureim Sandwich [신규]
    ↓
entrypoint/hooks/         notification.rs [신규]
    mod.rs                stop.rs [신규]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `workflow/log_system.rs` | `run(conn, stdin_json, event_type) -> Result<()>` |
| `entrypoint/hooks/notification.rs` | `run() -> Result<()>` |
| `entrypoint/hooks/stop.rs` | `run() -> Result<()>` |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `domain/log.rs` | `SystemEvent` 구조체 + factory + getters 추가 |
| `adapter/log_repo.rs` | `save_system_event`, `list_system_events_by_session` 함수 추가 |
| `adapter/mapper.rs` | `system_event_from_row` 함수 추가 |
| `workflow/mod.rs` | `pub mod log_system;` 추가 |
| `entrypoint/hooks/mod.rs` | `pub mod notification; pub mod stop;` 추가 |
| `main.rs` | `HookAction::Notification`, `HookAction::Stop` 서브커맨드 추가 |

### Cargo.toml 변경

없음.

---

## QA 목록

### 기능 검증 — Notification

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | 유효한 Notification JSON stdin 전달 시 `system_events` 테이블에 정확히 1행이 추가되고 exit 0으로 종료된다 | E2E: 바이너리 호출 → exit 0 + `SELECT COUNT(*)` == 1 |
| Q2 | 저장된 행의 `session_id`, `event_type`("Notification"), `content`(=message), `project`, `project_path`가 stdin JSON과 일치한다 | E2E: INSERT 후 `SELECT session_id, event_type, content, project, project_path` 비교 |

### 기능 검증 — Stop

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q3 | 유효한 Stop JSON stdin 전달 시 `system_events` 테이블에 정확히 1행이 추가되고 exit 0으로 종료된다 | E2E: 바이너리 호출 → exit 0 + `SELECT COUNT(*)` == 1 |
| Q4 | 저장된 행의 `session_id`, `event_type`("Stop"), `content`(=stop_reason), `project`, `project_path`가 stdin JSON과 일치한다 | E2E: INSERT 후 `SELECT session_id, event_type, content, project, project_path` 비교 |

### 기능 검증 — 공통

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q5 | `save_system_event`로 저장한 행의 `id`, `session_id`, `project`, `project_path`, `event_type`, `content`, `timestamp`가 원본과 일치한다. `timestamp`는 save 직전/직후 시각 범위(±1초) 내에 있다 | 통합: save → raw SELECT → 전체 비교 + timestamp 범위 검증 |
| Q6 | `list_system_events_by_session`으로 저장된 행을 조회하면 원본 `SystemEvent`와 동일한 값이 반환된다 | 통합: save → list → 전체 비교 |
| Q7 | workflow `run`에 Notification JSON을 전달하면 `event_type`="Notification", `content`=message로 1행이 추가된다 | 통합: workflow run → list → 검증 |
| Q8 | workflow `run`에 Stop JSON을 전달하면 `event_type`="Stop", `content`=stop_reason으로 1행이 추가된다 | 통합: workflow run → list → 검증 |

### 에러 처리

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q9 | `notification` 빈 stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q10 | `notification` 잘못된 JSON(`{invalid}`) stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q11 | `notification`에서 `session_id` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q12 | `notification`에서 `message` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q13 | `stop` 빈 stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q14 | `stop` 잘못된 JSON(`{invalid}`) stdin은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q15 | `stop`에서 `session_id` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E |
| Q16 | `stop`에서 `stop_reason` 필드 누락 JSON은 exit 1 + stderr에 에러 메시지 포함 | E2E |

### 타입 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q17 | workflow `run`에 잘못된 DB 연결을 전달하면 에러를 반환한다 | 통합: 닫힌 connection → `Err(AdapterError::Database(_))` |
| Q18 | `SystemEvent`는 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` derive를 갖고, `assert_eq!(event.clone(), event)`가 성공한다 | 단위 |
| Q19 | `SystemEvent`의 각 필드를 getter로 읽을 수 있다 | 단위: 각 getter 반환값 검증 |
| Q20 | `SystemEvent`의 `Display` 출력이 `[{session_id}] {event_type} ({id})` 형식이다 | 단위 |

---

## Test Pyramid

### Unit Tests (domain 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_system_event_creation` | Q18, Q19 | `SystemEvent` 생성 + 각 필드 getter 검증 |
| `test_system_event_display` | Q20 | `Display` 형식 검증 `[{session_id}] {event_type} ({id})` |

### Integration Tests (adapter + workflow 계층)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_system_event_inserts_row` | Q5 | `save_system_event` → DB 행 검증 |
| `test_list_system_events_by_session_returns_saved` | Q6 | save → list → 전체 비교 |
| `test_list_system_events_by_session_empty` | Q6 | 존재하지 않는 세션 → 빈 Vec |
| `test_workflow_notification_run` | Q7 | workflow `run` → Notification 이벤트 DB 행 추가 확인 |
| `test_workflow_stop_run` | Q8 | workflow `run` → Stop 이벤트 DB 행 추가 확인 |
| `test_workflow_run_with_bad_connection` | Q17 | 닫힌 DB 연결 → AdapterError 반환 |

### E2E Tests (바이너리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_notification_hook_saves_to_db` | Q1, Q2 | 유효한 Notification JSON → DB 저장 + 필드 확인 |
| `test_stop_hook_saves_to_db` | Q3, Q4 | 유효한 Stop JSON → DB 저장 + 필드 확인 |
| `test_notification_hook_empty_stdin` | Q9 | 빈 stdin → exit 1 |
| `test_notification_hook_invalid_json` | Q10 | 잘못된 JSON → exit 1 |
| `test_notification_hook_missing_session_id` | Q11 | session_id 누락 → exit 1 |
| `test_notification_hook_missing_message` | Q12 | message 누락 → exit 1 |
| `test_stop_hook_empty_stdin` | Q13 | 빈 stdin → exit 1 |
| `test_stop_hook_invalid_json` | Q14 | 잘못된 JSON → exit 1 |
| `test_stop_hook_missing_session_id` | Q15 | session_id 누락 → exit 1 |
| `test_stop_hook_missing_stop_reason` | Q16 | stop_reason 누락 → exit 1 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과
- [x] 사용자 승인 완료
