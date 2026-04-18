# Feature 12: Project CRUD

상위 문서: [Phase 2 태스크 관리 설계](../plans/2026-04-15-task-management.md)

---

## 목적

태스크를 묶는 관리 단위인 Project의 생성과 조회를 구현한다. 각 프로젝트는 고유한 `ProjectPrefix`(대문자 알파벳 3글자)를 가지며, 이후 태스크 ID(`{prefix}-{seq}`)의 접두사로 사용된다.

**Ground Truth 연결:**
- 정량 측정: 프로젝트 단위로 태스크를 그룹화함으로써, SessionMetrics를 프로젝트별로 집계할 수 있게 된다. 프로젝트 성격(신규 개발 vs 리팩터링)에 따른 프록시 지표 차이를 분석할 수 있다.
- 동치 보장: 동일 프로젝트 내에서 하니스 변경 전후의 태스크 지표를 비교함으로써, 프로젝트 특성이라는 혼란 변수를 통제한 상태에서 효율 비교가 가능해진다.

---

## 입력

### `seogi project create`

| 항목 | 설명 |
|------|------|
| `--name` (필수) | 프로젝트 이름 (문자열) |
| `--prefix` (선택) | `ProjectPrefix` — 대문자 알파벳 3글자. 미지정 시 이름 앞 3글자 대문자로 자동 생성 |
| `--goal` (필수) | 프로젝트 목표 (문자열) |
| DB 상태 | Feature 11에서 시딩된 statuses 존재, projects 테이블에 `next_seq` 컬럼 포함 |

### `seogi project list`

| 항목 | 설명 |
|------|------|
| `--json` (선택) | JSON 형식 출력 플래그 |
| DB 상태 | projects 테이블 |

---

## 출력

### `seogi project create`

| 항목 | 설명 |
|------|------|
| stdout | 생성된 프로젝트 정보 (이름, prefix) |
| DB 변경 | `projects` 테이블에 1행 INSERT (id: UUID hex, next_seq: 1) |

### `seogi project list`

| 항목 | 설명 |
|------|------|
| stdout (기본) | 테이블 형식 프로젝트 목록 |
| stdout (`--json`) | JSON 배열 |

---

## 성공 시나리오

1. **명시적 prefix로 생성**: `seogi project create --name "Seogi" --prefix "SEO" --goal "하니스 계측"` → `ProjectPrefix` 검증 통과 → 중복 없음 확인 → DB 저장 → 성공 메시지 출력
2. **자동 prefix로 생성**: `seogi project create --name "Seogi" --goal "하니스 계측"` → 이름 앞 3글자 "Seo" → 대문자 변환 "SEO" → 이하 동일
3. **목록 조회 (테이블)**: `seogi project list` → projects 전체 조회 → 테이블 형식 출력
4. **목록 조회 (JSON)**: `seogi project list --json` → projects 전체 조회 → JSON 배열 출력
5. **빈 목록 조회**: 프로젝트가 없을 때 list → 빈 테이블/빈 JSON 배열 출력

## 실패 시나리오

1. **필수 인자 누락**: `--name` 또는 `--goal` 미지정 → clap이 자동 처리 (usage 출력 + exit 2). 이 문서의 범위 밖.
2. **잘못된 prefix 형식**: 대문자 알파벳 3글자가 아닌 값 → `DomainError` 반환 → stderr에 에러 메시지 + 비정상 종료
3. **중복 prefix**: 이미 같은 prefix의 프로젝트 존재 → `DomainError` 반환 → stderr에 에러 메시지 + 비정상 종료
4. **자동 prefix 생성 불가**: 이름이 3글자 미만이거나 앞 3글자가 ASCII 알파벳이 아닌 경우 → `DomainError` 반환 → stderr에 에러 메시지 ("--prefix를 직접 지정하세요")
5. **빈 문자열 입력**: `--name ""` 또는 `--goal ""` → 허용하지 않음. 도메인에서 빈 문자열 검증 → `DomainError` 반환
6. **DB 에러**: SQLite I/O 실패 → `DomainError::Database` 반환 → stderr에 에러 메시지

---

## 제약 조건

- **의존성**: Feature 11 (초기 데이터 시딩 + 스키마 변경) 완료 필수
- **ProjectPrefix 규칙**: 정확히 대문자 ASCII 알파벳 3글자 (`[A-Z]{3}`). 프로젝트 간 UNIQUE
- **name 중복**: 허용한다. prefix만 UNIQUE 제약
- **빈 문자열 금지**: name, goal은 빈 문자열을 허용하지 않는다
- **ID 형식**: UUID v4 hex 32글자
- **next_seq 초기값**: 도메인에서 1로 설정 (DB DEFAULT 아님)
- **에러 전파**: `adapter(rusqlite::Error) → DomainError::Database → entrypoint(anyhow → stderr)`

---

## 의존 Feature

- Feature 11: 초기 데이터 시딩 + 스키마 변경 (StatusCategory enum, statuses 시딩, projects.next_seq 추가)

---

## 구현 범위

### 도메인 계층

| 파일 | 내용 |
|------|------|
| `domain/project.rs` (신규) | `ProjectPrefix` newtype, `Project` 구조체 |

`ProjectPrefix` newtype:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPrefix(String);
```

- `new(value: &str) -> Result<Self, DomainError>`: 대문자 알파벳 3글자 검증
- `from_name(name: &str) -> Result<Self, DomainError>`: 이름 앞 3글자를 대문자로 변환하여 생성. 3글자 미만이거나 비ASCII이면 에러
- `as_str() -> &str`: 내부 문자열 참조

`Project` 구조체:

```rust
pub struct Project {
    pub id: String,
    pub name: String,
    pub prefix: ProjectPrefix,
    pub goal: String,
    pub next_seq: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

- `new(name: &str, prefix: ProjectPrefix, goal: &str) -> Self`: UUID hex id 생성, next_seq = 1, 현재 시각 설정

### 어댑터 계층

| 파일 | 내용 |
|------|------|
| `adapter/project_repo.rs` (신규) | `save`, `find_all`, `find_by_prefix` 함수 |
| `adapter/mapper.rs` (수정) | `project_from_row` 추가 |

- `save(conn: &Connection, project: &Project) -> Result<(), AdapterError>`: INSERT
- `find_all(conn: &Connection) -> Result<Vec<Project>, AdapterError>`: SELECT 전체
- `find_by_prefix(conn: &Connection, prefix: &ProjectPrefix) -> Result<Option<Project>, AdapterError>`: prefix로 조회 (중복 검증용)

### 워크플로우 계층

| 파일 | 내용 |
|------|------|
| `workflow/project.rs` (신규) | `create`, `list` 함수 |

`create`:
```
[Impure] find_by_prefix로 중복 확인
[Pure] Project::new 생성
[Impure] save로 저장
→ Result<Project, DomainError>
```

`list`:
```
[Impure] find_all로 전체 조회
→ Result<Vec<Project>, DomainError>
```

### 엔트리포인트 계층

| 파일 | 내용 |
|------|------|
| `main.rs` (수정) | `Commands::Project` 서브커맨드 추가 |

```rust
enum Commands {
    // 기존...
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

enum ProjectAction {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        goal: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}
```

---

## QA 목록

### 도메인 (ProjectPrefix)

| # | QA 항목 |
|---|---------|
| Q1 | `ProjectPrefix::new()`는 대문자 알파벳 3글자를 허용한다 ("SEO", "LOC", "ABC") |
| Q2 | `ProjectPrefix::new()`는 소문자("seo"), 숫자 포함("SE1"), 길이 부적합("SE", "SEOG"), 빈 문자열을 모두 거부한다 |
| Q3 | `ProjectPrefix::as_str()`은 생성 시 전달한 문자열을 그대로 반환한다 |
| Q4 | `ProjectPrefix::from_name("Seogi")`는 `ProjectPrefix("SEO")`를 반환한다 |
| Q5 | `ProjectPrefix::from_name()`은 이름이 3글자 미만("ab")이거나 앞 3글자가 비ASCII("서기프")이면 에러를 반환한다 |

### 도메인 (Project)

| # | QA 항목 |
|---|---------|
| Q6 | `Project::new()` 생성 시 id는 32글자 hex 문자열이다 |
| Q7 | `Project::new()` 생성 시 `next_seq`은 1이다 |
| Q8 | `Project::new()` 생성 시 `created_at`과 `updated_at`은 현재 시각 근처이다 |
| Q9 | `Project::new()`는 name 또는 goal이 빈 문자열이면 에러를 반환한다 |

### 어댑터 (project_repo)

| # | QA 항목 |
|---|---------|
| Q10 | `save`로 저장한 Project가 `find_all`로 조회된다 |
| Q11 | Project가 없을 때 `find_all`은 빈 Vec을 반환한다 |
| Q12 | `find_by_prefix`는 해당 prefix의 Project를 반환한다 |
| Q13 | 존재하지 않는 prefix로 `find_by_prefix` 호출 시 `None`을 반환한다 |

### 워크플로우 (project)

| # | QA 항목 |
|---|---------|
| Q14 | 유효한 입력으로 `create` 호출 시 Project가 DB에 저장되고 반환된다 |
| Q15 | 중복 prefix로 `create` 호출 시 에러를 반환하고 DB는 미변경이다 |
| Q16 | prefix 미지정(None)으로 `create` 호출 시 이름에서 자동 생성하여 저장된다 |

### E2E (CLI)

| # | QA 항목 |
|---|---------|
| Q17 | `seogi project create --name "Seogi" --prefix "SEO" --goal "..."` 실행 시 성공 메시지를 출력하고 exit 0이다 |
| Q18 | 중복 prefix로 create 시 에러를 stderr에 출력하고 exit code가 0이 아니다 |
| Q19 | `seogi project list` 실행 시 테이블 형식으로 프로젝트 목록을 출력한다 |
| Q20 | `seogi project list --json` 실행 시 유효한 JSON 배열을 출력한다 |
| Q21 | 프로젝트가 없을 때 `seogi project list` 실행 시 빈 결과(헤더만 또는 빈 배열)를 출력한다 |

---

## Test Pyramid

### Unit Tests (domain/project.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_project_prefix_valid` | Q1 | 유효한 대문자 3글자 → 성공 |
| `test_project_prefix_invalid` | Q2 | 소문자, 숫자, 길이 부적합 등 → 에러 |
| `test_project_prefix_as_str` | Q3 | `as_str()` 반환값 일치 |
| `test_project_prefix_from_name` | Q4 | 이름에서 prefix 자동 생성 |
| `test_project_prefix_from_name_invalid` | Q5 | 짧은 이름, 비ASCII → 에러 |
| `test_project_new_id` | Q6 | UUID hex 32글자 검증 |
| `test_project_new_next_seq` | Q7 | next_seq == 1 |
| `test_project_new_timestamps` | Q8 | created_at/updated_at 현재 시각 근처 |
| `test_project_new_empty_name_or_goal` | Q9 | 빈 name 또는 goal → 에러 |

### Integration Tests (adapter/project_repo.rs, workflow/project.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_save_and_find_all` | Q10 | save → find_all에 포함 |
| `test_find_all_empty` | Q11 | 빈 DB → 빈 Vec |
| `test_find_by_prefix_found` | Q12 | 존재하는 prefix → Some(Project) |
| `test_find_by_prefix_not_found` | Q13 | 없는 prefix → None |
| `test_workflow_create_success` | Q14 | workflow create → DB 저장 + 반환 |
| `test_workflow_create_duplicate_prefix` | Q15 | 중복 prefix → 에러, DB 미변경 |
| `test_workflow_create_auto_prefix` | Q16 | prefix None → 이름에서 자동 생성하여 저장 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_cli_project_create` | Q17 | 명시적 prefix로 create → 성공 메시지, exit 0 |
| `test_cli_project_create_duplicate` | Q18 | 중복 prefix → stderr 에러, exit != 0 |
| `test_cli_project_list_table` | Q19 | list → 테이블 출력 |
| `test_cli_project_list_json` | Q20 | list --json → JSON 배열 |
| `test_cli_project_list_empty` | Q21 | 빈 목록 → 빈 결과 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] 목적, 입력/출력, 시나리오, 제약 모두 명시됨
- [x] QA 목록의 각 항목이 테스트 가능한 명제
- [x] Test Pyramid 분배표 작성됨
- [x] 의존하는 Feature 순서 명확
- [x] 사용자 승인 완료 (2026-04-18)
