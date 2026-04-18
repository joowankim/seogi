# Feature 11: 초기 데이터 시딩 + 스키마 변경

상위 문서: [Phase 2 태스크 관리 설계](../plans/2026-04-15-task-management.md)

---

## 목적

태스크 관리 시스템의 상태(status) 체계를 확립한다. `status_categories` 테이블을 제거하고 코드 enum(`StatusCategory`)으로 대체하여 5개 카테고리를 타입 시스템으로 보장한다. 기본 statuses 7개를 스키마 적용 시 자동 삽입하고, `projects` 테이블에 `next_seq` 컬럼을 추가하여 태스크 시퀀스 채번 기반을 마련한다.

**Ground Truth 연결:**
- 정량 측정: 태스크 기반 성과 지표(사이클 타임, 처리량 등)를 수집하기 위한 상태 체계 기반
- 동치 보장: 상태 전환 규칙이 코드 레벨에서 보장되어 일관된 지표 산출 가능

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | 없음 (스키마 변경과 시딩은 DB 초기화 시 자동 수행) |
| DB 상태 | 기존 `status_categories` 테이블 존재 / `statuses.category_id` 컬럼 존재 / `projects.next_seq` 컬럼 없음 |

---

## 출력

| 항목 | 설명 |
|------|------|
| 스키마 변경 | `status_categories` 테이블 DROP, `statuses.category_id` → `category TEXT`, `projects.next_seq INTEGER NOT NULL` 추가 |
| 데이터 시딩 | `statuses` 테이블에 기본 7개 행 INSERT |
| 도메인 타입 | `StatusCategory` enum (5개 variant) |
| SCHEMA_VERSION | 2 → 3 |

---

## 성공 시나리오

1. **최초 실행 (새 DB)**: `status_categories` 테이블 없이 스키마가 적용되고, `statuses` 테이블에 기본 7개 행이 자동 삽입된다. `projects` 테이블에 `next_seq` 컬럼이 포함된다.
2. **기존 DB 업그레이드 (v2 → v3)**: SQLite는 `ALTER TABLE`로 기존 테이블을 변경할 수 없으므로, `CREATE TABLE IF NOT EXISTS`와 스키마 버전 체크로 처리한다. 기존 Phase 1 데이터(tool_uses, session_metrics 등)는 영향 없다. `status_categories` 테이블은 DROP된다.
3. **재실행**: 이미 v3인 DB에서는 스키마 적용과 시딩이 스킵된다.

## 실패 시나리오

1. **스키마 적용 실패**: SQL 구문 오류 → `AdapterError::Database` 반환.
2. **시딩 데이터 중복**: `INSERT OR IGNORE`로 기존 데이터와 충돌 시 무시.

---

## 제약 조건

- **멱등성**: 스키마 적용 + 시딩은 반복 실행해도 안전해야 한다.
- **하위 호환**: Phase 1에서 사용 중인 테이블(tool_uses, tool_failures, system_events, session_metrics, changelog)은 변경하지 않는다.
- **성능**: DB 초기화 시 실행되므로 훅 50ms 예산 내에서 완료되어야 한다.
- **기본 statuses ID**: UUID v4 hex 형식. 시딩 시 고정 ID를 사용하여 멱등성 보장.

---

## 의존 Feature

없음 (Phase 2 첫 번째 Feature).

---

## 구현 범위

### 도메인 계층

| 파일 | 내용 |
|------|------|
| `domain/status.rs` (신규) | `StatusCategory` enum: `Backlog`, `Unstarted`, `Started`, `Completed`, `Canceled`. `as_str()`, `FromStr` 구현. |

`StatusCategory` enum 설계:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCategory {
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
}
```

- `as_str()` → `"backlog"`, `"unstarted"`, `"started"`, `"completed"`, `"canceled"` (DB 저장용 소문자)
- `FromStr` → 소문자 문자열에서 역변환. 잘못된 값이면 `DomainError::Validation` 반환.

### 어댑터 계층

| 파일 | 변경 내용 |
|------|-----------|
| `adapter/db.rs` | `SCHEMA_SQL` 수정: `status_categories` 제거, `statuses.category TEXT`, `projects.next_seq` 추가 |
| `adapter/db.rs` | `SEED_SQL` 상수 추가: 기본 statuses 7개 `INSERT OR IGNORE` |
| `adapter/db.rs` | `SCHEMA_VERSION` 2 → 3 |
| `adapter/db.rs` | `apply_schema()` 에서 시딩도 함께 실행 |
| `adapter/db.rs` | 마이그레이션 로직: v2 → v3 시 `DROP TABLE IF EXISTS status_categories` 실행 |

### 스키마 SQL 변경

**변경 전 (`statuses`, `projects` 관련 부분):**
```sql
CREATE TABLE IF NOT EXISTS status_categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES status_categories(id),
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,
    goal        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
```

**변경 후:**
```sql
CREATE TABLE IF NOT EXISTS statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    position    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,
    goal        TEXT NOT NULL,
    next_seq    INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
```

### 시딩 데이터

| id (고정 UUID) | name | category | position |
|----------------|------|----------|----------|
| `00000000000000000000000000000001` | backlog | backlog | 0 |
| `00000000000000000000000000000002` | todo | unstarted | 1 |
| `00000000000000000000000000000003` | in_progress | started | 2 |
| `00000000000000000000000000000004` | in_review | started | 3 |
| `00000000000000000000000000000005` | blocked | started | 4 |
| `00000000000000000000000000000006` | done | completed | 5 |
| `00000000000000000000000000000007` | canceled | canceled | 6 |

고정 ID를 사용하여 `INSERT OR IGNORE`로 멱등성을 보장한다.

### 마이그레이션 (v2 → v3)

`setup_connection()`에서 버전 체크 시:
1. `version < 3`이면 `DROP TABLE IF EXISTS status_categories` 실행
2. 새 `SCHEMA_SQL` 적용 (변경된 `statuses`, `projects` 포함)
3. `SEED_SQL` 실행
4. `user_version`을 3으로 업데이트

**주의**: SQLite의 `CREATE TABLE IF NOT EXISTS`는 기존 테이블 구조를 변경하지 않는다. 따라서 기존 v2 DB에서 `statuses` 테이블의 `category_id` 컬럼은 그대로 남는다. 그러나 Phase 1에서 `statuses` 테이블에 실제 데이터를 넣지 않았으므로, `DROP TABLE IF EXISTS statuses` 후 재생성하는 것이 안전하다. 마찬가지로 `projects` 테이블도 Phase 1에서 스키마 테스트용으로만 사용했으므로 재생성한다.

v2 → v3 마이그레이션 SQL:
```sql
DROP TABLE IF EXISTS status_categories;
DROP TABLE IF EXISTS statuses;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS task_events;
DROP TABLE IF EXISTS projects;
```

이후 `SCHEMA_SQL`이 새 구조로 재생성하고, `SEED_SQL`이 기본 데이터를 삽입한다.

---

## QA 목록

### 도메인 (StatusCategory enum)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | `StatusCategory`는 정확히 5개 variant를 갖는다: `Backlog`, `Unstarted`, `Started`, `Completed`, `Canceled` | 각 variant 생성 후 패턴 매칭으로 전수 확인 |
| Q2 | `as_str()`는 각 variant를 소문자 문자열로 변환한다 (`"backlog"`, `"unstarted"`, `"started"`, `"completed"`, `"canceled"`) | 5개 variant에 대해 `as_str()` 결과 비교 |
| Q3 | `FromStr`는 유효한 소문자 문자열을 올바른 variant로 변환한다 | 5개 문자열 파싱 → 기대 variant 비교 |
| Q4 | `FromStr`는 잘못된 문자열에 대해 `Err`를 반환한다 | `"invalid"`, `""`, `"BACKLOG"` 파싱 → `is_err()` |

### 스키마 변경

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q5 | 초기화 후 `sqlite_master`에 `status_categories` 테이블이 존재하지 않는다 | `SELECT name FROM sqlite_master WHERE type='table'` 결과에 `status_categories` 미포함 |
| Q6 | 초기화 후 테이블 목록이 정확히 9개이다: `changelog`, `projects`, `session_metrics`, `statuses`, `system_events`, `task_events`, `tasks`, `tool_failures`, `tool_uses` | 테이블 이름 목록 비교 |
| Q7 | `statuses` 테이블 컬럼이 `(id TEXT, name TEXT NOT NULL, category TEXT NOT NULL, position INTEGER NOT NULL)`이다 | `PRAGMA table_info(statuses)` 결과 비교 |
| Q8 | `projects` 테이블에 `next_seq INTEGER NOT NULL` 컬럼이 포함된다 | `PRAGMA table_info(projects)` 결과 비교 |
| Q9 | `SCHEMA_VERSION`이 3이다 | 초기화 후 `PRAGMA user_version` 결과 == 3 |

### 시딩

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q10 | 초기화 후 `statuses` 테이블에 정확히 7개 행이 존재한다 | `SELECT COUNT(*) FROM statuses` == 7 |
| Q11 | 7개 행의 `(name, category, position)` 값이 기대 데이터와 일치한다 | `SELECT name, category, position FROM statuses ORDER BY position` 결과 비교 |
| Q12 | 시딩 데이터의 `category` 값이 모두 유효한 `StatusCategory` variant 문자열이다 | 각 행의 `category` 값에 대해 `StatusCategory::from_str()` 성공 확인 |

### 멱등성

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q13 | 스키마 + 시딩 적용 후 재적용하면 `statuses` 테이블에 여전히 7개 행만 존재한다 | `user_version` 리셋 → 재적용 → `SELECT COUNT(*)` == 7 |
| Q14 | Phase 1 테이블(tool_uses 등)에 데이터를 INSERT한 뒤 스키마 재적용하면 해당 데이터가 유지된다 | tool_uses에 INSERT → 재적용 → SELECT COUNT 결과 유지 |

### 마이그레이션 (v2 → v3)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q15 | v2 스키마 DB에서 v3로 업그레이드 시 `status_categories` 테이블이 제거된다 | v2 DB 생성 → v3 적용 → `status_categories` 테이블 미존재 확인 |
| Q16 | v2 → v3 업그레이드 시 Phase 1 데이터(tool_uses 등)가 보존된다 | v2 DB에 tool_uses 데이터 INSERT → v3 적용 → 데이터 유지 확인 |

---

## Test Pyramid

### Unit Tests (domain/status.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_status_category_as_str` | Q2 | 5개 variant → 소문자 문자열 변환 |
| `test_status_category_from_str_valid` | Q3 | 유효한 문자열 → variant 변환 |
| `test_status_category_from_str_invalid` | Q4 | 잘못된 문자열 → Err |
| `test_status_category_exhaustive` | Q1 | match 문으로 5개 variant 전수 확인 |

### Integration Tests (adapter/db.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_schema_creates_all_tables` (수정) | Q5, Q6 | 9개 테이블 확인, `status_categories` 미포함 |
| `test_schema_columns_statuses` (수정) | Q7 | `category TEXT NOT NULL` 컬럼 확인 |
| `test_schema_columns_projects` (수정) | Q8 | `next_seq INTEGER NOT NULL` 컬럼 포함 확인 |
| `test_schema_version_set_after_init` (수정) | Q9 | `user_version` == 3 |
| `test_seed_statuses_count` | Q10 | `statuses` 테이블 7개 행 |
| `test_seed_statuses_data` | Q11 | 7개 행의 `(name, category, position)` 값 검증 |
| `test_seed_statuses_valid_categories` | Q12 | 시딩 데이터 `category` → `StatusCategory::from_str()` 성공 |
| `test_seed_idempotent` | Q13 | 재적용 후 여전히 7개 행 |
| `test_schema_preserves_phase1_data` | Q14 | Phase 1 테이블 데이터 유지 |
| `test_migration_v2_to_v3_drops_status_categories` | Q15 | v2 → v3 마이그레이션 시 `status_categories` 제거 |
| `test_migration_v2_to_v3_preserves_data` | Q16 | v2 → v3 마이그레이션 시 Phase 1 데이터 보존 |

### E2E Tests

이번 Feature에서 E2E 테스트는 없다. 스키마 변경과 시딩은 내부 함수이며 CLI 명령으로 직접 노출되지 않는다.

---

## 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 항목이 성공/실패 시나리오를 모두 커버
- [ ] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [ ] 사용자 승인 완료
