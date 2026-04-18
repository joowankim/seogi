# Feature 09: Changelog (`seogi changelog add`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

하니스 변경 이력을 SQLite `changelog` 테이블에 기록한다.

**Ground Truth 연결:**
- **동치 보장**: 하니스 변경 시점을 기록하여, report에서 "이 시점 전후로 지표가 달라졌는가?"를 판단할 수 있는 시간축 마커를 제공한다

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | `seogi changelog add <description>` |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용) |

---

## 출력

| 항목 | 설명 |
|------|------|
| DB 변경 | `changelog` 테이블에 1행 INSERT |
| stdout | `Recorded at <timestamp>` 확인 메시지 |
| 반환값 | exit 0 (성공), exit 1 (실패) |

### changelog 테이블 스키마

```sql
CREATE TABLE IF NOT EXISTS changelog (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);
```

| 컬럼 | 값 |
|------|------|
| `id` | UUID v4 hex |
| `description` | CLI 인자 `description` |
| `timestamp` | 현재 시각 (밀리초 Unix timestamp) |

---

## 성공 시나리오

1. `seogi changelog add "CLAUDE.md 규칙 변경"`이 실행된다.
2. UUID와 현재 timestamp를 생성한다.
3. `changelog` 테이블에 1행을 INSERT한다.
4. stdout에 `Recorded at <timestamp>` 메시지를 출력한다.
5. exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| DB 접근 불가 | exit 1 + stderr에 에러 메시지 |
| description 인자 누락 | clap 자동 에러 처리 |

---

## 제약 조건

- **append-only**: 삭제/수정 없이 추가만. 중복 방지 불필요 (매번 새 UUID)
- **간결함**: domain 타입 없이 workflow에서 직접 처리해도 충분한 수준

---

## 의존 Feature

- **Feature 01: DB 초기화** — `initialize_db` 함수. `changelog` 테이블은 이번 Feature에서 스키마에 추가

---

## 구현 범위

### 수직 슬라이스

changelog는 단순 INSERT이므로 domain 순수 함수가 불필요하다. adapter + workflow + entrypoint로 구성.

```
adapter/db.rs             changelog 테이블 스키마 추가 [수정]
adapter/changelog_repo.rs save_changelog [신규]
    ↓
workflow/changelog.rs     run(conn, description) [신규]
    ↓
main.rs                   Changelog 서브커맨드 연결 변경 [수정]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `adapter/changelog_repo.rs` | `save_changelog(conn, id, description, timestamp)` |
| `workflow/changelog.rs` | `run(conn, description) -> Result<Timestamp>` |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `adapter/db.rs` | `changelog` 테이블 CREATE TABLE 추가 |
| `adapter/mod.rs` | `pub mod changelog_repo;` 추가 |
| `workflow/mod.rs` | `pub mod changelog;` 추가 |
| `main.rs` | `Changelog Add` 서브커맨드를 새 workflow로 연결 |

---

## QA 목록

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | `save_changelog` 호출 후 `changelog` 테이블에 1행이 추가된다 | 통합 |
| Q2 | 저장된 행의 `description`이 인자와 일치한다 | 통합 |
| Q3 | 저장된 행의 `id`가 32자 hex이다 | 통합 |
| Q4 | `workflow::changelog::run` 호출 후 `Timestamp`가 반환된다 | 통합 |
| Q5 | `seogi changelog add "test"` 실행 시 stdout에 "Recorded at"이 포함되고 exit 0이다 | E2E |
| Q6 | `seogi changelog add` (description 없음) 실행 시 exit != 0이다 | E2E |

---

## Test Pyramid

### Integration Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_changelog` | Q1, Q2, Q3 | save → raw SELECT → 전체 비교 |
| `test_workflow_changelog_run` | Q4 | workflow run → Timestamp 반환 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_changelog_add_command` | Q5 | 바이너리 → stdout 확인 |
| `test_changelog_add_no_args` | Q6 | description 없음 → exit != 0 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과 (단순 Feature, 리뷰 생략)
- [ ] 사용자 승인 완료
