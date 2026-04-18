# Feature 07: 마이그레이션 (`seogi migrate`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

기존 `~/seogi-logs/` 디렉토리의 JSONL 로그 파일을 SQLite `tool_uses`와 `tool_failures` 테이블로 마이그레이션한다.

**Ground Truth 연결:**
- **정량 측정**: 기존에 축적된 JSONL 로그 데이터를 SQLite로 통합하여, Feature 06(analyze)에서 과거 세션도 분석 가능하게 한다. 과거 데이터가 없으면 하니스 변경 전 기준선을 구축할 수 없다
- **동치 보장**: 하니스 변경 전 JSONL로 수집된 기준선 데이터를 SQLite로 이관하여, 변경 후 SQLite 데이터와 동일한 쿼리 인터페이스로 비교할 수 있게 한다

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | `seogi migrate` (인자 없음) |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용), `SEOGI_LOG_DIR` (선택, 테스트용. 미설정 시 config.json의 `logDir`) |
| 파일 시스템 | `{logDir}/{project}/*.jsonl` 파일들 (compact 또는 pretty-printed JSON) |

### LogEntry → 도메인 타입 매핑

| 조건 | 대상 테이블 | 매핑 |
|------|-----------|------|
| `tool` 존재 + `failed` != `Some(true)` | `tool_uses` | `tool.name` → `tool_name`, `tool.input` → `tool_input` (JSON 문자열), `tool.duration_ms` → `duration` (Ms), `timestamp` → RFC3339 파싱 후 밀리초 Timestamp |
| `tool` 존재 + `failed` == `Some(true)` | `tool_failures` | `tool.name` → `tool_name`, `tool.error` → `error` (없으면 빈 문자열) |
| `tool` 없음 (시스템 메시지) | 건너뜀 | 마이그레이션 대상 아님. `content`에 `[stop]` 등이 있지만 기존 JSONL에는 구조화된 이벤트 정보가 부족하여 의미 있는 SystemEvent로 변환 불가 |

### ID 생성 (중복 방지)

콘텐츠 기반 결정론적 ID: `SHA-256(session_id + timestamp + tool_name)의 앞 32자 hex`

같은 JSONL 데이터를 두 번 마이그레이션해도 같은 ID가 생성되어 `INSERT OR IGNORE`로 중복을 방지한다.

---

## 출력

| 항목 | 설명 |
|------|------|
| DB 변경 | `tool_uses` 및 `tool_failures` 테이블에 행 INSERT |
| stdout | 마이그레이션 요약 (프로젝트별 파일 수, 행 수, 건너뜀 수) |
| 반환값 | exit 0 (성공), exit 1 (실패) |
| stderr | 실패 시 에러 메시지 |
| 원본 파일 | 보존 (삭제하지 않음) |

---

## 성공 시나리오

1. `seogi migrate`가 실행된다.
2. config.json에서 `logDir`을 읽어 JSONL 디렉토리 경로를 결정한다.
3. `{logDir}/{project}/` 디렉토리를 순회한다 (`metrics/` 하위 디렉토리 제외).
4. 각 `.jsonl` 파일을 파싱한다 (compact/pretty-printed 자동 감지).
5. 각 LogEntry를 도메인 타입으로 변환한다 (ToolUse 또는 ToolFailure).
6. 콘텐츠 기반 ID를 생성하고 `INSERT OR IGNORE`로 DB에 저장한다.
7. 파싱 실패 엔트리는 건너뛰고 stderr에 경고를 출력한다.
8. 마이그레이션 요약을 stdout에 출력하고 exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| logDir 경로가 존재하지 않음 | stdout에 "No log directory found" 출력 후 exit 0 (에러가 아님) |
| 개별 JSONL 파일 읽기 실패 | 해당 파일 건너뛰고 stderr 경고. 다른 파일 계속 처리 |
| 개별 LogEntry 파싱 실패 | 해당 엔트리 건너뛰고 건너뜀 카운터 증가. 다른 엔트리 계속 처리 |
| DB 접근 불가 | exit 1 + stderr에 에러 메시지 |
| config.json 없음 또는 logDir 미설정 | exit 1 + stderr에 에러 메시지 |
| RFC3339 timestamp 파싱 실패 | 해당 엔트리 건너뜀 |

---

## 제약 조건

- **멱등성**: 재실행 시 중복 행이 생기지 않음 (콘텐츠 기반 ID + `INSERT OR IGNORE`)
- **비파괴적**: 원본 JSONL 파일을 삭제하거나 수정하지 않음
- **파싱 호환**: compact(`{...}\n`) 및 pretty-printed(멀티라인) JSONL 모두 지원
- **부분 실패 허용**: 파싱 실패 엔트리는 건너뛰고 나머지를 계속 처리

---

## 의존 Feature

- **Feature 01: DB 초기화** — `tool_uses`, `tool_failures` 테이블 스키마
- **Feature 02: 도구 사용 로깅** — `save_tool_use` adapter 함수
- **Feature 03: 도구 실패 로깅** — `save_tool_failure` adapter 함수

---

## 구현 범위

### 수직 슬라이스

기존 `log_reader.rs`의 JSONL 파서를 재사용한다. `log_reader.rs`는 파일 I/O를 수반하는 adapter 수준 모듈로 간주한다 (현재 루트에 위치하지만 역할상 adapter).

Config 로드(파일 I/O)는 entrypoint(main.rs)에서 수행하고, `log_dir` 경로를 workflow에 인자로 주입한다.

```
domain/migrate.rs          LogEntry → ToolUse/ToolFailure 변환 순수 함수 [신규]
    ↓
adapter 계층               log_reader.rs (JSONL 파싱, 재사용)
                           log_repo.rs (save_tool_use/save_tool_failure, 재사용)
    ↓
workflow/migrate.rs        JSONL 읽기(adapter) → 변환(domain) → DB 저장(adapter) [신규]
    ↓
entrypoint (main.rs)       Config 로드 → log_dir 추출 → workflow 호출 [수정]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `domain/migrate.rs` | `convert_entry(entry) -> Option<MigratedRecord>`, `content_based_id(session_id, timestamp, tool_name) -> String` |
| `workflow/migrate.rs` | `run(conn, log_dir) -> Result<MigrateSummary>` |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `domain/mod.rs` | `pub mod migrate;` 추가 |
| `workflow/mod.rs` | `pub mod migrate;` 추가 |
| `main.rs` | `Migrate` 서브커맨드 추가 |

### 재사용 모듈 (변경 없음)

| 파일 | 재사용 함수 |
|------|-----------|
| `log_reader.rs` | `parse_jsonl_content` (현재 private → pub(crate)로 변경 필요) |
| `adapter/log_repo.rs` | `save_tool_use`, `save_tool_failure` |
| `adapter/db.rs` | `initialize_db` |
| `entrypoint/hooks/mod.rs` | `db_path()` |
| `config.rs` | `Config::load`, `Config::log_dir_expanded` |

---

## QA 목록

### 변환 (순수 함수)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | `tool` 존재 + `failed` 없음인 LogEntry가 `ToolUse`로 변환된다 | 단위 |
| Q2 | `tool` 존재 + `failed: Some(true)`인 LogEntry가 `ToolFailure`로 변환된다 | 단위 |
| Q3 | `tool` 없는 LogEntry는 `None`을 반환한다 (건너뜀) | 단위 |
| Q4 | `content_based_id("sess-1", "2026-04-07T11:00:00Z", "Bash")`가 동일 입력에 대해 항상 같은 32자 hex를 반환한다 | 단위 |
| Q5 | 변환된 ToolUse의 `tool_input`이 원본 `tool.input`의 JSON 문자열과 일치한다 | 단위 |
| Q6 | RFC3339 timestamp가 밀리초 Unix timestamp로 올바르게 변환된다 | 단위 |
| Q7 | `tool.duration_ms`가 `Some(150)`이면 변환된 ToolUse의 `duration`이 `Ms::new(150)`이다 | 단위 |
| Q8 | `tool.duration_ms`가 `None`이면 변환된 ToolUse의 `duration`이 `Ms::zero()`이다 | 단위 |

### workflow

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q9 | compact JSONL 1건을 마이그레이션하면 `tool_uses`에 1행이 추가된다 | 통합 |
| Q10 | pretty-printed JSONL 1건을 마이그레이션하면 `tool_uses`에 1행이 추가된다 | 통합 |
| Q11 | failed LogEntry를 마이그레이션하면 `tool_failures`에 1행이 추가된다 | 통합 |
| Q12 | 같은 JSONL을 두 번 마이그레이션해도 `tool_uses` 행 수가 변하지 않는다 (멱등성) | 통합 |
| Q13 | 파싱 실패 엔트리가 있으면 건너뛰고 나머지는 정상 처리된다 | 통합 |
| Q14 | `tool` 없는 엔트리(시스템 메시지)는 건너뛴다 | 통합 |
| Q15 | `{logDir}/{project}/metrics/` 하위의 `.jsonl` 파일은 마이그레이션 대상에서 제외된다 | 통합 |

### E2E

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q16 | `seogi migrate` 실행 시 JSONL 파일의 ToolUse 행이 DB에 추가되고 exit 0이다 | E2E |
| Q17 | `seogi migrate`를 두 번 실행해도 DB 행 수가 동일하다 (멱등성) | E2E |
| Q18 | logDir이 존재하지 않으면 exit 0이다 (에러가 아님) | E2E |

---

## Test Pyramid

### Unit Tests (domain/migrate.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_convert_tool_use` | Q1, Q5, Q6, Q7 | 성공 LogEntry → ToolUse |
| `test_convert_tool_failure` | Q2 | 실패 LogEntry → ToolFailure |
| `test_convert_no_tool` | Q3 | tool 없음 → None |
| `test_content_based_id_deterministic` | Q4 | 같은 입력 → 같은 ID |
| `test_duration_none_fallback` | Q8 | duration 없음 → Ms::zero() |

### Integration Tests (workflow/migrate.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_migrate_compact_jsonl` | Q9 | compact → tool_uses 1행 |
| `test_migrate_pretty_printed` | Q10 | pretty-printed → tool_uses 1행 |
| `test_migrate_failure` | Q11 | failed → tool_failures 1행 |
| `test_migrate_idempotent` | Q12 | 2회 실행 → 행 수 동일 |
| `test_migrate_skips_unparseable` | Q13 | 잘못된 줄 건너뜀 |
| `test_migrate_skips_no_tool` | Q14 | tool 없음 건너뜀 |
| `test_migrate_skips_metrics_dir` | Q15 | metrics/ 하위 제외 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_migrate_command` | Q16 | 바이너리 호출 → DB 확인 |
| `test_migrate_idempotent_e2e` | Q17 | 2회 호출 → 행 수 동일 |
| `test_migrate_no_logdir` | Q18 | logDir 없음 → exit 0 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과
- [ ] 사용자 승인 완료
