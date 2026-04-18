# Feature 13: Status CRUD

상위 문서: [Phase 2 태스크 관리 설계](../plans/2026-04-15-task-management.md)

---

## 목적

태스크의 상태(Status)를 생성, 수정, 삭제, 조회하는 CLI 명령어를 구현한다. Feature 11에서 시딩된 기본 7개 상태와 사용자가 추가한 커스텀 상태를 동일하게 관리한다. 각 Status는 `StatusCategory` enum에 속하며, position은 전체 상태 중 글로벌 순서로 자동 부여된다.

**Ground Truth 연결:**
- 정량 측정: 상태 전환 이벤트(task_events)의 from_status/to_status가 Status를 참조하므로, 커스텀 상태 추가를 통해 더 세분화된 워크플로우 단계별 지표(사이클 타임 세분화 등)를 수집할 수 있다.
- 동치 보장: 동일한 상태 체계 하에서 하니스 변경 전후의 상태 전환 패턴을 비교함으로써, 워크플로우 효율 변화를 측정할 수 있다.

---

## 입력

### `seogi status create`

| 항목 | 설명 |
|------|------|
| `--category` (필수) | `StatusCategory` enum 값 (backlog, unstarted, started, completed, canceled) |
| `--name` (필수) | 상태 이름 (문자열, 빈 문자열 불가) |
| DB 상태 | statuses 테이블에 시딩된 7개 상태 존재 |

### `seogi status update`

| 항목 | 설명 |
|------|------|
| `<id>` (필수) | 수정할 Status의 id (UUID hex 32글자) |
| `--name` (필수) | 변경할 이름 |

### `seogi status delete`

| 항목 | 설명 |
|------|------|
| `<id>` (필수) | 삭제할 Status의 id |

### `seogi status list`

| 항목 | 설명 |
|------|------|
| `--json` (선택) | JSON 형식 출력 플래그 |

---

## 출력

### `seogi status create`

| 항목 | 설명 |
|------|------|
| stdout | `Created status "testing" (started, position 7)` 형식 |
| DB 변경 | `statuses` 테이블에 1행 INSERT |

### `seogi status update`

| 항목 | 설명 |
|------|------|
| stdout | `Updated status <id>` 형식 |
| DB 변경 | `statuses` 테이블 해당 행의 name UPDATE |

### `seogi status delete`

| 항목 | 설명 |
|------|------|
| stdout | `Deleted status <id>` 형식 |
| DB 변경 | `statuses` 테이블에서 해당 행 DELETE |

### `seogi status list`

| 항목 | 설명 |
|------|------|
| stdout (기본) | 테이블 형식 상태 목록 (category → position 순 정렬) |
| stdout (`--json`) | JSON 배열 |

---

## 성공 시나리오

1. **상태 생성**: `seogi status create --category started --name "testing"` → 카테고리 검증 → 전체 max position + 1 자동 부여 → DB 저장 → 성공 메시지
2. **상태 수정**: `seogi status update <id> --name "qa_review"` → id로 조회 → name 변경 → 성공 메시지
3. **상태 삭제**: `seogi status delete <id>` → id로 조회 → tasks 참조 없음 확인 → 삭제 → 성공 메시지
4. **목록 조회 (테이블)**: `seogi status list` → 전체 조회 → 테이블 출력
5. **목록 조회 (JSON)**: `seogi status list --json` → 전체 조회 → JSON 배열 출력

## 실패 시나리오

1. **필수 인자 누락**: `--category`, `--name`, `<id>` 등 미지정 → clap 자동 처리 (이 문서 범위 밖)
2. **잘못된 카테고리**: `StatusCategory` enum에 없는 값 → `DomainError::Validation` → stderr + exit 1
3. **빈 이름 (create/update 공통)**: `--name ""` → `DomainError::Validation` → stderr + exit 1
4. **존재하지 않는 id**: update/delete 시 해당 id 없음 → `DomainError::Validation` → stderr + exit 1
5. **tasks에서 참조 중인 상태 삭제**: tasks.status_id가 해당 Status를 참조 → `DomainError::Validation` ("사용 중인 상태는 삭제할 수 없습니다") → stderr + exit 1
6. **DB 에러**: SQLite I/O 실패 → `DomainError::Database` → stderr + exit 1

---

## 제약 조건

- **의존성**: Feature 11 (초기 데이터 시딩) 완료 필수
- **카테고리 규칙**: `StatusCategory` enum 5개 값만 허용
- **이름 규칙**: 빈 문자열 불가. 이름 중복은 허용 (같은 이름 다른 카테고리 가능)
- **position**: 전체 max position + 1로 글로벌 순서 자동 부여. 수동 지정 불가. 기존 시딩 데이터(0-6)와 일관
- **update 범위**: name만 변경 가능. category, position 변경 불가
- **기본 상태 동등 취급**: 시딩된 7개 상태도 수정/삭제 가능 (특별 보호 없음)
- **삭제 안전성**: tasks에서 참조 중인 Status는 삭제 불가
- **ID 형식**: UUID v4 hex 32글자
- **에러 전파**: `adapter(rusqlite::Error) → DomainError::Database → entrypoint(anyhow → stderr)`

---

## 의존 Feature

- Feature 11: 초기 데이터 시딩 + 스키마 변경 (StatusCategory enum, 기본 statuses 7개 시딩)

---

## 구현 범위

### 도메인 계층

| 파일 | 내용 |
|------|------|
| `domain/status.rs` (수정) | `Status` 구조체 추가 (기존 `StatusCategory` enum 유지) |

`Status` 구조체:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    id: String,
    name: String,
    category: StatusCategory,
    position: i64,
}
```

- `new(name: &str, category: StatusCategory, position: i64) -> Result<Self, DomainError>`: UUID 생성, 빈 이름 검증
- `from_row(id, name, category, position) -> Self`: DB 복원용
- accessor 메서드: `id()`, `name()`, `category()`, `position()`

### 어댑터 계층

| 파일 | 내용 |
|------|------|
| `adapter/status_repo.rs` (신규) | `save`, `list_all`, `find_by_id`, `update_name`, `delete`, `max_position_in_category`, `is_referenced_by_tasks` |
| `adapter/mapper.rs` (수정) | `status_from_row` 추가 |

- `save(conn, &Status) -> rusqlite::Result<()>`
- `list_all(conn) -> rusqlite::Result<Vec<Status>>`: position 순 정렬
- `find_by_id(conn, id) -> rusqlite::Result<Option<Status>>`
- `update_name(conn, id, name) -> rusqlite::Result<bool>`: 변경된 행 수 > 0 여부 반환
- `delete(conn, id) -> rusqlite::Result<bool>`: 삭제된 행 수 > 0 여부 반환
- `max_position(conn) -> rusqlite::Result<Option<i64>>`: 전체 최대 position
- `is_referenced_by_tasks(conn, status_id) -> rusqlite::Result<bool>`

### 워크플로우 계층

| 파일 | 내용 |
|------|------|
| `workflow/status.rs` (신규) | `create`, `list`, `update`, `delete` 함수 |

`create`:
```
[Pure] StatusCategory::from_str로 파싱
[Impure] max_position 조회
[Pure] Status::new 생성 (position = max + 1, 없으면 0)
[Impure] save
→ Result<Status, DomainError>
```

`update`:
```
[Impure] find_by_id로 존재 확인
[Pure] name 빈 문자열 검증
[Impure] update_name
→ Result<(), DomainError>
```

`delete`:
```
[Impure] find_by_id로 존재 확인
[Impure] is_referenced_by_tasks 확인
[Impure] delete
→ Result<(), DomainError>
```

### 엔트리포인트 계층

| 파일 | 내용 |
|------|------|
| `main.rs` (수정) | `Commands::Status` 서브커맨드 추가 |

```rust
enum StatusAction {
    Create {
        #[arg(long)]
        category: String,
        #[arg(long)]
        name: String,
    },
    Update {
        id: String,
        #[arg(long)]
        name: String,
    },
    Delete {
        id: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}
```

---

## QA 목록

### 도메인 (Status)

| # | QA 항목 |
|---|---------|
| Q1 | `Status::new()` 생성 시 id는 UUID hex 32글자이다 |
| Q2 | `Status::new()` 생성 시 전달한 name, category, position이 accessor로 조회된다 |
| Q3 | `Status::new()`는 name이 빈 문자열이면 에러를 반환한다 |

### 어댑터 (status_repo)

| # | QA 항목 |
|---|---------|
| Q4 | `save`로 저장한 Status가 `list_all`로 조회된다 |
| Q5 | `list_all`은 position 순으로 정렬하여 반환한다 |
| Q6 | `find_by_id`는 해당 id의 Status를 반환하고, 없으면 None을 반환한다 |
| Q7 | `update_name`은 해당 Status의 name을 변경하고 true를 반환한다 |
| Q8 | 존재하지 않는 id로 `update_name` 호출 시 false를 반환한다 |
| Q9 | `delete`는 해당 Status를 삭제하고 true를 반환한다 |
| Q10 | 존재하지 않는 id로 `delete` 호출 시 false를 반환한다 |
| Q11 | `max_position`은 전체 최대 position을 반환한다 |
| Q12 | `is_referenced_by_tasks`는 tasks에서 참조 중이면 true, 아니면 false를 반환한다 |

### 워크플로우 (status)

| # | QA 항목 |
|---|---------|
| Q13 | `create` 호출 시 position은 전체 max position + 1로 자동 부여된다 (시딩 후 첫 생성 시 7) |
| Q14 | 유효하지 않은 category 문자열로 `create` 호출 시 에러를 반환한다 |
| Q15 | `update` 호출 시 name이 변경된다 |
| Q15a | `update` 시 빈 이름이면 에러를 반환한다 |
| Q16 | 존재하지 않는 id로 `update` 호출 시 에러를 반환한다 |
| Q17 | `delete` 호출 시 해당 Status가 삭제된다 |
| Q18 | 존재하지 않는 id로 `delete` 호출 시 에러를 반환한다 |
| Q19 | tasks에서 참조 중인 Status를 `delete` 호출 시 에러를 반환하고 DB 미변경이다 |

### E2E (CLI)

| # | QA 항목 |
|---|---------|
| Q20 | `seogi status create --category started --name "testing"` 실행 시 성공 메시지를 출력하고 exit 0이다 |
| Q21 | `seogi status list` 실행 시 시딩된 7개를 포함한 테이블을 출력한다 |
| Q22 | `seogi status list --json` 실행 시 유효한 JSON 배열을 출력한다 |
| Q23 | `seogi status update <id> --name "renamed"` 실행 시 성공 메시지를 출력한다 |
| Q24 | `seogi status delete <id>` 실행 시 성공 메시지를 출력한다 |

---

## Test Pyramid

### Unit Tests (domain/status.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_status_new_id` | Q1 | UUID hex 32글자 |
| `test_status_new_fields` | Q2 | name, category, position 보존 |
| `test_status_new_empty_name` | Q3 | 빈 이름 → 에러 |

### Integration Tests (adapter/status_repo.rs, workflow/status.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_and_list_all` | Q4, Q5 | save → list_all에 포함, position 순 정렬 |
| `test_find_by_id` | Q6 | 존재/미존재 |
| `test_update_name` | Q7, Q8 | 존재하는 id → true, 없는 id → false |
| `test_delete` | Q9, Q10 | 존재하는 id → true, 없는 id → false |
| `test_max_position` | Q11 | 전체 최대 position |
| `test_is_referenced_by_tasks` | Q12 | tasks 참조 여부 |
| `test_workflow_create_auto_position` | Q13 | position 자동 부여 |
| `test_workflow_create_invalid_category` | Q14 | 잘못된 category → 에러 |
| `test_workflow_update_success` | Q15 | name 변경 |
| `test_workflow_update_empty_name` | Q15a | 빈 이름 → 에러 |
| `test_workflow_update_not_found` | Q16 | 없는 id → 에러 |
| `test_workflow_delete_success` | Q17 | 삭제 성공 |
| `test_workflow_delete_not_found` | Q18 | 없는 id → 에러 |
| `test_workflow_delete_referenced` | Q19 | tasks 참조 → 에러, DB 미변경 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_cli_status_create` | Q20 | create → 성공 메시지, exit 0 |
| `test_cli_status_list_table` | Q21 | list → 테이블 (7개 + 생성분) |
| `test_cli_status_list_json` | Q22 | list --json → JSON 배열 |
| `test_cli_status_update` | Q23 | update → 성공 메시지 |
| `test_cli_status_delete` | Q24 | delete → 성공 메시지 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] 목적, 입력/출력, 시나리오, 제약 모두 명시됨
- [x] QA 목록의 각 항목이 테스트 가능한 명제
- [x] Test Pyramid 분배표 작성됨
- [x] 의존하는 Feature 순서 명확
- [x] 사용자 승인 완료 (2026-04-18)
