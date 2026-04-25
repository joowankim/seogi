# Feature 01: 프로젝트 부트스트랩 + DB 초기화

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

Rust CLI 프로젝트에 SQLite를 연결하고, 전체 DB 스키마를 적용하여 이후 Feature들이 즉시 데이터를 저장/조회할 수 있는 기반을 마련한다.

**Ground Truth 연결:**
- 정량 측정: 세션 로그와 메트릭을 SQLite에 저장하기 위한 스키마 기반
- 동치 보장: 기존 JSONL 데이터를 마이그레이션할 수 있는 저장소 준비

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | 없음 (DB 초기화는 모든 명령 실행 전 자동 수행) |
| 환경 | `~/.seogi/` 디렉토리 (없으면 자동 생성) |
| DB 상태 | 파일 없음 → 생성 / 파일 있음 → 기존 스키마 유지 |

---

## 출력

| 항목 | 설명 |
|------|------|
| 파일 생성 | `~/.seogi/seogi.db` SQLite 파일 |
| 스키마 적용 | 9개 테이블: `projects`, `status_categories`, `statuses`, `tasks`, `task_events`, `tool_uses`, `tool_failures`, `system_events`, `session_metrics` |
| 반환값 | `Result<Connection, AdapterError>` |

---

## 성공 시나리오

1. **최초 실행**: `~/.seogi/` 디렉토리와 `seogi.db` 파일이 생성되고, 9개 테이블 스키마가 적용된다.
2. **재실행**: 기존 DB 파일과 스키마가 그대로 유지된다 (`CREATE TABLE IF NOT EXISTS`).
3. **테스트 환경**: 인메모리 DB (`:memory:`)로 격리된 테스트가 가능하다.

## 실패 시나리오

1. **디렉토리 생성 불가**: 파일 시스템 권한 부족 → `AdapterError::Io` 반환.
2. **DB 파일 손상**: SQLite 파일이 손상된 경우 → `AdapterError::Database` 반환.
3. **스키마 적용 실패**: SQL 구문 오류 → `AdapterError::Database` 반환.

---

## 제약 조건

- **성능**: DB 초기화는 모든 명령 실행 전에 호출되므로 빠르게 완료되어야 한다 (훅의 50ms 예산 고려).
- **호환성**: `rusqlite` + `bundled` 피처로 외부 SQLite 의존성 없이 동작.
- **경로**: `~/.seogi/seogi.db` 고정 경로 (현 단계에서 설정 가능성 불필요).
- **멱등성**: 스키마 적용은 반복 실행해도 안전해야 한다 (`IF NOT EXISTS`).

---

## 의존 Feature

없음 (첫 번째 Feature).

---

## 구현 범위

### 코드 구조 리팩토링

기존 플랫 구조를 함수형 3계층으로 재배치:

```
app/src/
├── main.rs                    # 진입점 (변경)
├── lib.rs                     # 모듈 선언 (변경)
├── domain/
│   ├── mod.rs
│   └── error.rs               # DomainError 정의
├── adapter/
│   ├── mod.rs
│   └── db.rs                  # Connection 관리, 스키마 초기화
├── workflow/                  # (이번 Feature에서는 빈 모듈)
│   └── mod.rs
└── entrypoint/                # (이번 Feature에서는 빈 모듈)
    └── mod.rs
```

기존 모듈(`config.rs`, `models.rs`, `log_reader.rs`, `metrics_reader.rs`, `analyzers/`, `commands/`)은 이번 Feature에서 이동하지 않는다. 이후 Feature에서 각 기능을 수직 슬라이스로 구현할 때 점진적으로 재배치한다.

### 신규 구현

| 파일 | 내용 |
|------|------|
| `domain/error.rs` | `DomainError` enum: `Validation(String)` variant (순수 도메인 에러) |
| `adapter/error.rs` | `AdapterError` enum: `Database(#[from] rusqlite::Error)`, `Io(#[from] std::io::Error)` variants |
| `adapter/db.rs` | `initialize_db(path) -> Result<Connection, AdapterError>`: 디렉토리 생성 + DB 열기 + 스키마 적용 |
| `adapter/db.rs` | `initialize_in_memory() -> Result<Connection, AdapterError>`: 테스트용 인메모리 DB |

### 스키마 SQL

`adapter/db.rs`에 `const SCHEMA_SQL: &str`로 임베딩. 9개 테이블의 `CREATE TABLE IF NOT EXISTS` 문:

```sql
CREATE TABLE IF NOT EXISTS projects ( ... );
CREATE TABLE IF NOT EXISTS status_categories ( ... );
CREATE TABLE IF NOT EXISTS statuses ( ... );
CREATE TABLE IF NOT EXISTS tasks ( ... );
CREATE TABLE IF NOT EXISTS task_events ( ... );
CREATE TABLE IF NOT EXISTS tool_uses ( ... );
CREATE TABLE IF NOT EXISTS tool_failures ( ... );
CREATE TABLE IF NOT EXISTS system_events ( ... );
CREATE TABLE IF NOT EXISTS session_metrics ( ... );
```

### Cargo.toml 변경

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
```

---

## QA 목록

### 기능 검증

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | 인메모리 DB 초기화 후 `sqlite_master`에 9개 테이블 이름(`projects`, `status_categories`, `statuses`, `tasks`, `task_events`, `tool_uses`, `tool_failures`, `system_events`, `session_metrics`)이 존재한다 | `SELECT name FROM sqlite_master WHERE type='table'` 결과와 기대 목록 비교 |
| Q2 | 각 테이블의 컬럼 이름·타입·NOT NULL 제약이 스키마 SQL 정의와 일치한다 | `PRAGMA table_info(<table>)` 결과의 `(name, type, notnull)` 튜플 비교 |
| Q3 | `projects` 테이블에 행을 INSERT한 뒤 스키마를 재적용하면, 해당 행이 그대로 남아 있다 | INSERT → `initialize_in_memory` 내부 스키마 적용 함수 재호출 → SELECT COUNT 결과 1 |
| Q4 | `AdapterError::Database(rusqlite::Error)`의 `Display` 출력이 `"Database error: ..."` 형식이다 | `format!("{}", err)` 검증 |
| Q5 | `AdapterError::Io(std::io::Error)`의 `Display` 출력이 `"IO error: not found"`이다 | `format!("{}", err)` == 기대 문자열 |

### 엣지 케이스

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q6 | 존재하지 않는 서브 경로 `tmp/a/b/seogi.db`로 `initialize_db`를 호출하면 `tmp/a/b/` 디렉토리가 생성되고 `Ok(Connection)`이 반환된다 | `tempdir` 하위 미존재 경로 전달 → `result.is_ok()` + `Path::exists` 확인 |
| Q7 | 동일 경로로 `initialize_db`를 2회 호출하면 두 번째도 `Ok(Connection)`을 반환하고, 첫 번째에서 INSERT한 행이 유지된다 | 1차 호출 → INSERT → 2차 호출 → SELECT COUNT 결과 1 |
| Q8 | 초기화된 Connection에서 `PRAGMA foreign_keys` 조회 결과가 `1`이다 | `conn.pragma_query_value(None, "foreign_keys", \|r\| r.get::<_, i64>(0))` == 1 |

---

## Test Pyramid

### Unit Tests (adapter/error, domain/error)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_adapter_error_database_display` | Q4 | `AdapterError::Database(rusqlite::Error)` → `"Database error: ..."` |
| `test_adapter_error_io_display` | Q5 | `AdapterError::Io(io::Error)` → `"IO error: not found"` |

### Integration Tests (adapter 계층 — 인메모리 SQLite)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_schema_creates_all_tables` | Q1 | 초기화 후 `sqlite_master`에서 9개 테이블 확인 |
| `test_schema_columns_match` | Q2 | 각 테이블 `PRAGMA table_info()` 검증 |
| `test_schema_idempotent` | Q3 | 데이터 INSERT → 재초기화 → 데이터 유지 |
| `test_foreign_keys_enabled` | Q8 | `PRAGMA foreign_keys` = 1 확인 |
| `test_initialize_db_creates_directory` | Q6 | 임시 경로에서 디렉토리 자동 생성 확인 |
| `test_initialize_db_idempotent` | Q7 | 동일 경로 2회 호출 → 성공 + 데이터 유지 |

### E2E Tests

이번 Feature에서 E2E 테스트는 없다. DB 초기화는 내부 함수이며 CLI 명령으로 직접 노출되지 않는다. 이후 Feature에서 CLI 명령이 DB를 사용할 때 E2E로 검증된다.

---

## 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 항목이 성공/실패 시나리오를 모두 커버
- [ ] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [ ] 사용자 승인 완료
