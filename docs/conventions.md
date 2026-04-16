# Seogi 코딩 컨벤션

Rust 기반 seogi 프로젝트의 코딩 컨벤션. `~/.claude/rules-storage/python/bootstrap/`의 Python DDD 규칙을 Rust 환경에 맞게 번역한 것.

모든 설계와 구현은 이 문서를 따른다. 예외는 명시적으로 문서화한다.

각 규칙의 근거는 섹션 끝에 링크로 표시한다.

**주요 참고 자료:**
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — 네이밍, 상호운용, 유연성 가이드
- [The Rust Reference](https://doc.rust-lang.org/reference/) — 언어 사양
- [The Rust Book](https://doc.rust-lang.org/book/) — 개념과 패턴
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/) — 공식 스타일

---

## 1. 네이밍 컨벤션

### 식별자 스타일

| 종류 | 스타일 | 예시 |
|---|---|---|
| 모듈/함수/변수 | `snake_case` | `fn create_task`, `task_count` |
| 타입/트레이트/Enum | `UpperCamelCase` | `TaskRepository`, `Status` |
| 상수/static | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT` |
| 매크로 | `snake_case!` | `println!`, `vec!` |
| 타입 파라미터 | 간결한 `UpperCamelCase` | `T`, `R`, `Key` |
| 라이프타임 | 짧은 `lowercase` | `'a`, `'de` |

**근거**: [Rust API Guidelines — Naming (C-CASE)](https://rust-lang.github.io/api-guidelines/naming.html) / [RFC 430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md)

### 함수 이름

- **동사로 시작**: `create_task`, `update_status`, `validate_prefix`
- **Boolean 반환**: `is_`, `has_`, `can_`, `should_` 접두사
- **컬렉션**: 복수형 (`tasks`, `tool_uses`, `user_ids`)

### Repository 쿼리 메서드

| 접두사 | 반환 | 예시 |
|---|---|---|
| `find_` | `Option<Entity>` | `find_by_id`, `find_by_session` |
| `list_` | `Vec<Entity>` | `list_by_project`, `list_active` |
| `save` | `Result<()>` | `save` |
| `delete` | `Result<()>` | `delete` |

### Domain Service 이름

**`~Service`, `~Handler` 접미사 금지.** 역할을 드러내는 구체적 이름 사용:

| 패턴 | 용도 | 예시 |
|---|---|---|
| `~Calculator` | 계산 | `MetricsCalculator`, `CycleTimeCalculator` |
| `~Policy` | 비즈니스 규칙 | `StatusTransitionPolicy` |
| `~Validator` | 검증 | `TaskIdValidator` |
| `~Evaluator` | 평가/판단 | `RegressionEvaluator` |
| `~Estimator` | 추정 | `SizeEstimator` |
| `~Allocator` | 할당 | `SessionAllocator` |
| `~Counter` / `~Totalizer` | 집계 | `TokenCounter` |
| `~Matcher` | 매칭 | `SessionTaskMatcher` |
| `~Registry` | 등록/조회 | `ProjectRegistry` |
| `~Transfer` | 이동/전송 | - |

**Application Layer의 `~Handler`는 예외**. 유스케이스 오케스트레이션 목적.

### 모듈 기반 네이밍 (접미사 중복 제거)

```rust
// BAD: 접미사 중복
use crate::application::command::CreateTaskCommand;
let cmd = CreateTaskCommand { ... };

// GOOD: 모듈 기반
use crate::application::command;
let cmd = command::CreateTask { ... };
```

### 예약어 회피

Rust 키워드와 충돌 방지 (`r#` raw identifier 회피):

| Rust 키워드 | 대체 |
|---|---|
| `type` | `kind`, `category` |
| `ref` | `reference`, `target` |
| `move` | `transfer`, `relocate` |
| `self` (변수로) | `this`, `entity` |

변수명으로 `id`는 Rust에서 관례적으로 허용. 다만 스코프에서 구분이 필요하면 `task_id`, `session_id` 처럼 도메인 접두사 사용.

**근거**: [The Rust Reference — Keywords](https://doc.rust-lang.org/reference/keywords.html) (strict keywords 전체 목록)

---

## 2. 아키텍처: 4계층 DDD

### 계층 구조

```
entrypoint (CLI, MCP 서버, 훅)
    ↓
application (Handler, Command/Query DTO)
    ↓
domain (Entity, Value Object, Repository trait, Domain Service)
    ↓
adapter (Repository 구현, SQLite, Mapper)
```

### 의존성 방향

- **아래로만 흐름**: 상위 계층은 하위 계층에 의존. 역방향 금지.
- **domain은 인프라 의존성 금지**: `rusqlite`, `clap`, `reqwest` 등 기술 의존성 금지
- **domain에 허용되는 외부 크레이트**: `serde` (직렬화), `thiserror` (에러), `chrono` (시간), `uuid` (id 생성) 등 범용 유틸리티는 허용
- **Repository 인터페이스는 domain**, **구현은 adapter**

### 동기/비동기

- **seogi는 동기(sync)**: CLI + 훅 위주라 비동기 필요 없음
- `async/await` 미사용 (단, 향후 MCP 서버에서 필요하면 해당 모듈만 도입)

### 디렉토리 구조 (계층 먼저, 모듈은 계층 안에)

```
cli/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── log.rs             # LogEntry 엔티티
│   │   ├── metrics.rs         # SessionMetrics 엔티티
│   │   ├── task.rs            # Task 엔티티 (Phase 2)
│   │   ├── project.rs         # Project 엔티티 (Phase 2)
│   │   ├── repository.rs      # Repository trait 정의
│   │   ├── metrics_calculator.rs  # Domain Service
│   │   └── error.rs
│   ├── application/
│   │   ├── mod.rs
│   │   ├── command.rs         # Command DTO
│   │   ├── query.rs           # Query DTO
│   │   ├── analyze_handler.rs
│   │   ├── report_handler.rs
│   │   └── log_tool_handler.rs
│   ├── adapter/
│   │   ├── mod.rs
│   │   ├── db.rs              # SQLite 연결
│   │   ├── log_repository.rs  # SqliteLogRepository
│   │   ├── metrics_repository.rs
│   │   └── mapper.rs          # Entity ↔ Row 변환
│   └── entrypoint/
│       ├── mod.rs
│       ├── cli/
│       │   ├── mod.rs
│       │   ├── analyze.rs
│       │   ├── report.rs
│       │   └── migrate.rs
│       └── hooks/
│           ├── mod.rs
│           ├── pre_tool.rs
│           ├── post_tool.rs
│           ├── post_tool_failure.rs
│           ├── notification.rs
│           └── stop.rs
└── tests/
```

---

## 3. 엔티티와 값 객체

### 엔티티 불변성

- 상태 변경은 **새 인스턴스 반환** (`self -> Self`)
- `&mut self` 대신 `self` 소모 + 새 인스턴스
- 비즈니스 로직은 엔티티 메서드 안에

```rust
impl Task {
    pub fn move_to(self, new_status: Status) -> Result<Self, DomainError> {
        if !self.status.can_transition_to(&new_status) {
            return Err(DomainError::InvalidTransition { ... });
        }
        Ok(Self { status: new_status, ..self })
    }
}
```

### Enum에 동작 추가

```rust
impl StatusCategory {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Canceled)
    }

    pub fn can_transition_to(&self, next: &StatusCategory) -> bool { ... }
}
```

### Factory 메서드

상태 기반 또는 입력 기반으로 생성:

```rust
impl Task {
    // 상태 기반
    pub fn new_backlog(project_id: ProjectId, title: String, ...) -> Result<Self, DomainError> { ... }

    // 입력 기반 (필요 시)
    pub fn from_template(template: &TaskTemplate) -> Self { ... }
}
```

### Value Object로 primitive 대체

```rust
// BAD: String 남용
pub struct Task {
    id: String,
    project_id: String,
}

// GOOD: Newtype 패턴 + 필수 derive 트레이트
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(prefix: &Prefix, sequence: u32) -> Self { ... }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

Newtype에 **반드시 구현할 트레이트**:
- `Debug`: 로깅/디버깅
- `Clone`: 복제 가능성
- `PartialEq`, `Eq`: 비교
- `Hash`: HashMap 키로 사용 가능
- `Display`: 사용자 표시용 (`{}` 포매팅)
- `Serialize`, `Deserialize`: DB/JSON 입출력

**근거**:
- [Rust Book — Advanced Types (Newtype Pattern)](https://doc.rust-lang.org/book/ch20-03-advanced-types.html)
- [Rust API Guidelines — C-NEWTYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#newtypes-provide-static-distinctions-c-newtype)
- [Rust API Guidelines — C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#types-eagerly-implement-common-traits-c-common-traits): "crates that define new types should eagerly implement all applicable, common traits"

### 애그리거트 루트

- 1 애그리거트 = 1 Repository
- 다른 애그리거트는 **ID로만 참조** (객체 참조 금지)
- 애그리거트 크기 최소화

```rust
// GOOD: ID 참조
pub struct Task {
    project_id: ProjectId,  // Project 애그리거트를 ID로 참조
    ...
}

// BAD: 객체 참조
pub struct Task {
    project: Project,  // 직접 참조 금지
}
```

---

## 4. DTO와 스키마

### 계층별 데이터 구조

| 계층 | 타입 | 용도 |
|---|---|---|
| Entrypoint | `cli::Args` (clap) | CLI 입력 |
| Application | `command::X`, `query::X` | 유스케이스 입력 |
| Domain | `Entity`, `ValueObject` | 비즈니스 상태 |
| Adapter | Row structs | DB 매핑 |

### Command/Query 분리

```rust
// application/command.rs
pub struct CreateTask {
    pub project_id: ProjectId,
    pub title: String,
    pub description: String,
    pub label: Label,
}

// application/query.rs
pub struct ListTasks {
    pub project_id: Option<ProjectId>,
    pub status: Option<StatusId>,
}
```

### Mapper로 Entity ↔ Row 변환

```rust
// adapter/mapper.rs
pub fn row_to_task(row: &rusqlite::Row) -> Result<Task, rusqlite::Error> { ... }
pub fn task_to_params(task: &Task) -> [&dyn ToSql; N] { ... }
```

---

## 5. 에러 처리

### thiserror로 도메인 에러 정의

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("task not found: {0}")]
    TaskNotFound(TaskId),

    #[error("invalid status transition: {from:?} → {to:?}")]
    InvalidTransition { from: StatusId, to: StatusId },

    #[error("invalid prefix: {0}")]
    InvalidPrefix(String),

    #[error("database error")]
    Database(#[from] rusqlite::Error),
}
```

### anyhow는 바이너리 진입점에서만

- `main.rs`, `entrypoint/` 계층에서만 `anyhow::Result`
- `domain/`, `application/`은 구체적 에러 타입 (`Result<T, DomainError>`)

### Early return + ? 연산자

```rust
// GOOD
pub fn handle(cmd: CreateTask, repo: &mut impl TaskRepository) -> Result<Task, DomainError> {
    if cmd.title.is_empty() {
        return Err(DomainError::InvalidTitle);
    }

    let task = Task::new_backlog(cmd.project_id, cmd.title, cmd.description, cmd.label)?;
    repo.save(&task)?;
    Ok(task)
}

// BAD: 깊은 중첩
pub fn handle(cmd: CreateTask) -> Result<Task, DomainError> {
    if !cmd.title.is_empty() {
        if let Ok(task) = Task::new_backlog(...) {
            match repo.save(&task) {
                Ok(_) => Ok(task),
                Err(e) => Err(e),
            }
        } else { ... }
    } else { ... }
}
```

### 민감정보 노출 금지

에러 메시지에 비밀번호, 토큰 등 포함 금지.

**근거**:
- [Rust Book — Recoverable Errors with Result (`?` Operator)](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html): `?` 연산자는 `From::from`을 통해 자동 에러 변환
- [thiserror crate](https://docs.rs/thiserror/) — 라이브러리 에러
- [anyhow crate](https://docs.rs/anyhow/) — 애플리케이션 에러

---

## 6. 불변성

Rust의 불변성은 **"공유된 가변 상태 금지"**에 가깝지, "모든 변경 금지"가 아니다. Rust 소유권 시스템이 안전한 변경을 자동으로 보장한다.

### 허용되는 내부 mutation

소유한 데이터의 내부 변경은 **관례적이고 권장됨**:

```rust
// GOOD: 소유한 Vec의 내부 변경
fn add_item(items: &[Item], new_item: Item) -> Vec<Item> {
    let mut result = items.to_vec();
    result.push(new_item);  // 내부 mutation — 허용
    result
}
```

### 금지되는 것

**공유된 참조를 통한 변경**:

```rust
// BAD: &mut로 외부 상태 변경
fn add_item_bad(items: &mut Vec<Item>, new_item: Item) {
    items.push(new_item);  // 호출자의 상태를 변경
}
```

### 엔티티 상태 전환

엔티티 상태 변경은 **`self` 소모 + 새 인스턴스 반환**:

```rust
// GOOD
impl Task {
    pub fn move_to(self, new_status: Status) -> Result<Self, DomainError> {
        // self를 소모하고 새 Task 반환
        Ok(Self { status: new_status, ..self })
    }
}

// BAD: &mut self로 내부 mutation
impl Task {
    pub fn move_to(&mut self, new_status: Status) {
        self.status = new_status;  // 비즈니스 로직에선 지양
    }
}
```

### 함수 내부 구현

함수 내부에서 계산 과정의 `mut` 변수는 자유롭게 사용:

```rust
// GOOD: 함수 내부 mut은 구현 세부사항
fn calculate_total(items: &[Item]) -> Money {
    let mut total = Money::zero();
    for item in items {
        total = total.add(&item.price);
    }
    total
}
```

**근거**:
- [Rust Book — References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html): "variables are immutable by default, so are references"
- [Rust Book — Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html): 불변 참조 내에서도 mutate 가능한 패턴(필요 시 `RefCell`)

---

## 7. 하드코딩 금지

### 상수 정의

```rust
const DB_PATH: &str = "~/.seogi/seogi.db";
const DOOM_LOOP_THRESHOLD: u32 = 5;
const MAX_FILE_SIZE_MB: u32 = 10;
```

### 매직 스트링은 Enum

```rust
// BAD
if status == "in_progress" { ... }

// GOOD
if status == Status::InProgress { ... }
```

### 민감정보는 환경 변수

현재 seogi는 비밀이 없지만, 향후 필요 시 `std::env::var`로 로드.

---

## 8. 함수/파일 크기

- **함수**: 20줄 권장, 50줄 최대 (초과 시 분할)
- **파일**: 200~400줄 권장, 800줄 최대
- **중첩 깊이**: 4단계 max (early return + `?`로 완화)
- **파라미터**: 3개 권장, 5개 최대 (넘으면 구조체로)

### 함수 분할 예시

```rust
// GOOD: 작은 함수로 분할
pub fn handle(cmd: CreateTask) -> Result<Task, DomainError> {
    validate(&cmd)?;
    let task = build_task(&cmd)?;
    save_task(task)
}

fn validate(cmd: &CreateTask) -> Result<(), DomainError> { ... }
fn build_task(cmd: &CreateTask) -> Result<Task, DomainError> { ... }
fn save_task(task: Task) -> Result<Task, DomainError> { ... }
```

### 파라미터가 많으면 구조체 또는 builder로

```rust
// BAD
pub fn create_task(project_id: ProjectId, title: String, description: String, label: Label, status: StatusId, assignee: Option<String>) { ... }

// GOOD: 구조체로 묶기
pub fn create_task(input: CreateTaskInput) { ... }
pub struct CreateTaskInput { ... }

// GOOD: builder 패턴 (필드가 많고 optional이 많을 때)
pub fn create_task() -> TaskBuilder { TaskBuilder::default() }
// builder.title("...").description("...").label(Label::Feature).build()
```

**예외**: 구조체 빌드용 `new()` 함수나 대안 생성자는 5개 이상 파라미터 허용. 도메인 경계에서만 적용.

**근거**:
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/) — 공식 스타일 (파라미터 수 제한 없음)
- [Clippy — `too_many_arguments`](https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments) — 기본 임계값 7개. 이 프로젝트는 더 엄격하게 5개로 제한.

---

## 9. 테스트

### TDD 사이클

1. **RED**: 실패하는 테스트 먼저 작성
2. **GREEN**: 테스트를 통과하는 최소 코드
3. **REFACTOR**: 통과 상태 유지하며 개선

### AAA 패턴

```rust
#[test]
fn test_move_task_from_backlog_to_in_progress() {
    // Arrange
    let task = Task::new_backlog(...).unwrap();

    // Act
    let moved = task.move_to(Status::InProgress).unwrap();

    // Assert
    assert_eq!(moved.status, Status::InProgress);
}
```

### 객체 전체 비교

```rust
// GOOD: 전체 비교 — 필드 추가 시 자동 검출
let expected = Task {
    id: actual.id.clone(),  // 동적 값은 actual에서 가져옴
    title: "test".to_string(),
    status: Status::Backlog,
    ...
};
assert_eq!(actual, expected);

// BAD: 부분 비교 — 필드 추가를 놓칠 수 있음
assert_eq!(actual.title, "test");
assert_eq!(actual.status, Status::Backlog);
```

### 테스트 이름

서술적으로: `test_<대상>_<조건>_<기대결과>`

```rust
#[test]
fn test_create_task_with_duplicate_id_returns_error() { ... }

#[test]
fn test_move_task_from_done_to_backlog_is_rejected() { ... }
```

### 테스트 격리

- 각 테스트는 독립 실행 가능
- 임시 디렉토리/DB 사용 후 정리
- 테스트 간 상태 공유 금지

### 커버리지

- 최소 80%, 도메인 로직은 100% 지향
- `cargo-llvm-cov` 등으로 측정

---

## 10. 의존성 주입

### 생성자 주입

```rust
pub struct AnalyzeHandler<R: LogRepository, M: MetricsRepository> {
    log_repo: R,
    metrics_repo: M,
}

impl<R: LogRepository, M: MetricsRepository> AnalyzeHandler<R, M> {
    pub fn new(log_repo: R, metrics_repo: M) -> Self {
        Self { log_repo, metrics_repo }
    }
}
```

### 트레이트 기반 교체

테스트에서 mock 구현체로 교체:

```rust
struct MockLogRepository { ... }
impl LogRepository for MockLogRepository { ... }

#[test]
fn test_handler_with_mock() {
    let handler = AnalyzeHandler::new(MockLogRepository::new(), ...);
    ...
}
```

### Service Locator 금지

- 숨겨진 의존성 금지
- 전역 상태 금지 (`static mut`, `lazy_static` 최소화)

### Generic vs Trait Object 선택

| 방식 | 용도 | 장단점 |
|---|---|---|
| `<R: Repository>` 제네릭 | 기본값 | 컴파일 타임 확정, zero-cost, 단일 구현에 최적화 |
| `Box<dyn Repository>` | 런타임 유연성 필요 | 동적 디스패치, 힙 할당, 여러 구현을 동시 보관할 때 |

**seogi는 기본적으로 제네릭**. CLI 하나가 한 가지 Repository만 쓰므로 제네릭이 맞음. Trait Object는 필요 시에만.

```rust
// GOOD (제네릭)
pub struct AnalyzeHandler<R: LogRepository> {
    repo: R,
}

// 피해야 할 패턴 (이유 없는 Box<dyn>)
pub struct AnalyzeHandler {
    repo: Box<dyn LogRepository>,  // 런타임 비용 + 이유 없음
}
```

### Repository trait의 `&mut self` 문제

SQLite 트랜잭션은 `&mut Connection`을 요구한다. Repository trait의 `save` 메서드는 `&mut self`가 필요할 수 있음:

```rust
pub trait TaskRepository {
    fn find_by_id(&self, id: &TaskId) -> Result<Option<Task>, DomainError>;
    fn save(&mut self, task: &Task) -> Result<(), DomainError>;  // &mut self 주의
}
```

Repository 인스턴스를 여러 핸들러가 공유할 경우, 내부 가변성(`RefCell`, `Mutex`) 또는 함수마다 새로 생성하는 방식으로 대응.

**근거**:
- [Rust Book — Trait Objects](https://doc.rust-lang.org/book/ch18-02-trait-objects.html): "If you'll only ever have homogeneous collections, using generics and trait bounds is preferable because the definitions will be monomorphized at compile time"
- 동적 디스패치 비용: "This lookup incurs a runtime cost that doesn't occur with static dispatch. Dynamic dispatch also prevents the compiler from choosing to inline a method's code"
- [Rust API Guidelines — C-GENERIC](https://rust-lang.github.io/api-guidelines/flexibility.html#functions-minimize-assumptions-about-parameters-by-using-generics-c-generic)

---

## 11. 임포트 규칙

### 함수 안 `use` 금지

```rust
// BAD
fn process() {
    use std::fs::File;  // 함수 안 use 금지
    ...
}

// GOOD — 모듈 상단에
use std::fs::File;

fn process() { ... }
```

### 절대 경로 사용

```rust
// GOOD
use crate::domain::task::Task;

// BAD — 상대 경로
use super::super::task::Task;
```

### 임포트 순서

1. `std::`
2. 외부 크레이트
3. `crate::` (내부 모듈)

```rust
use std::path::Path;
use std::io::Write;

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::domain::task::Task;
use crate::domain::error::DomainError;
```

**근거**: [Rust Style Guide — Imports](https://doc.rust-lang.org/style-guide/#imports). `rustfmt`의 `group_imports` 옵션과 일치.

---

## 12. Rust 특화 규칙 (ROP + 이디오매틱)

### Result<T, E> 체인

`?` 연산자로 자연스럽게 ROP:

```rust
pub fn run(cmd: CreateTask) -> Result<Task, DomainError> {
    let project = project_repo.find_by_id(&cmd.project_id)?
        .ok_or(DomainError::ProjectNotFound(cmd.project_id.clone()))?;
    let task_id = generate_task_id(&project.prefix)?;
    let task = Task::new_backlog(task_id, cmd.title, cmd.description, cmd.label)?;
    task_repo.save(&task)?;
    Ok(task)
}
```

### `unwrap()` 금지 (프로덕션 코드)

- 프로덕션 경로: 반드시 `?` 또는 명시적 에러 처리
- 테스트 코드: `unwrap()` 허용
- 불변식이 명확한 경우: `expect("reason")`로 의도 명시

### Option 콤비네이터

```rust
// GOOD
users.iter()
    .find(|u| u.id == id)
    .map(|u| u.email.clone())

// BAD: 깊은 match 중첩
match users.iter().find(|u| u.id == id) {
    Some(u) => Some(u.email.clone()),
    None => None,
}
```

### Enum으로 상태 표현

```rust
// GOOD: 불가능한 상태를 표현 불가
enum TaskState {
    Backlog { created_at: DateTime },
    InProgress { started_at: DateTime, session_id: SessionId },
    Done { completed_at: DateTime, cycle_time_ms: u64 },
}
```

### 패턴 매칭 exhaustive

```rust
// GOOD: 새 variant 추가 시 컴파일러가 경고
match status {
    Status::Backlog => ...,
    Status::InProgress => ...,
    Status::Done => ...,
    // 와일드카드(_) 회피 — 비즈니스 enum은 exhaustive로
}
```

**근거**: [Rust Reference — `non_exhaustive`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute) / [RFC 2008](https://rust-lang.github.io/rfcs/2008-non-exhaustive.html). 와일드카드 회피는 관례적 모범 사례로, 새 variant 추가 시 컴파일러 경고로 놓침 방지.

### 소유권 설계 원칙

함수 파라미터는 **필요한 최소 권한**을 요구:

| 용도 | 시그니처 |
|---|---|
| 읽기만 | `&T` (빌림) |
| 수정 | `&mut T` |
| 소유권 이동/저장 | `T` |
| 유연한 입력 | `impl Into<T>` or `impl AsRef<T>` |

```rust
// GOOD: 최소 권한
fn format_title(title: &str) -> String { title.to_uppercase() }

// BAD: 불필요한 소유권 요구
fn format_title(title: String) -> String { title.to_uppercase() }
```

**근거**:
- [Rust API Guidelines — C-GENERIC (Flexibility)](https://rust-lang.github.io/api-guidelines/flexibility.html#functions-minimize-assumptions-about-parameters-by-using-generics-c-generic): "The fewer assumptions a function makes about its inputs, the more widely usable it becomes"
- [std::convert::AsRef](https://doc.rust-lang.org/std/convert/trait.AsRef.html)

### From/Into 트레이트

타입 변환은 `From`/`Into`로 정의. **항상 `From`을 구현**하고 `Into`는 자동 유도:

```rust
impl From<rusqlite::Error> for DomainError {
    fn from(err: rusqlite::Error) -> Self {
        DomainError::Database(err)
    }
}

// 자동으로 `Into`도 구현됨, `?`에서 자동 변환
fn save_task(conn: &Connection, task: &Task) -> Result<(), DomainError> {
    conn.execute(...)?;  // rusqlite::Error → DomainError 자동 변환
    Ok(())
}
```

제네릭 함수 경계에서는 `Into`를 사용:

```rust
// Into로 받으면 From 구현 타입도 자동 수용
fn new_task_id<P: Into<Prefix>>(prefix: P, seq: u32) -> TaskId { ... }
```

**근거**:
- [std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html): "One should always prefer implementing `From` over `Into` because implementing `From` automatically provides one with an implementation of `Into` thanks to the blanket implementation"
- [Rust Book — `?` Operator and From](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)

### Display / Debug

엔티티와 Value Object는 두 트레이트 구현:

- `Debug`: 개발자용 (`#[derive(Debug)]` 충분)
- `Display`: 사용자 표시용 (직접 구현)

```rust
impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

**근거**:
- [std::fmt::Debug](https://doc.rust-lang.org/std/fmt/trait.Debug.html): "Debug should format the output in a programmer-facing, debugging context. Generally speaking, you should just `derive` a `Debug` implementation"
- [std::fmt::Display](https://doc.rust-lang.org/std/fmt/trait.Display.html): "Display is similar to Debug, but Display is for user-facing output, and so cannot be derived"

### 메서드 접두사 관례

| 접두사 | 의미 | 비용 | 예시 |
|---|---|---|---|
| `as_` | 저비용 참조 변환 | O(1), no alloc | `as_str()`, `as_ref()` |
| `to_` | 복사/할당 변환 | 할당 O(n) | `to_string()`, `to_vec()` |
| `into_` | 소유권 이동 변환 | O(1) or O(n) | `into_iter()`, `into_bytes()` |

**근거**: [Rust API Guidelines — C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#ad-hoc-conversions-follow-as_-to_-into_-conventions-c-conv): "Conversions prefixed `as_` and `into_` typically decrease abstraction... Conversions prefixed `to_`, on the other hand, typically stay at the same level of abstraction but do some work to change from one representation to another"

### #[must_use]

반환값을 무시하면 안 되는 함수/타입에 붙임:

```rust
#[must_use]
pub fn move_to(self, new_status: Status) -> Result<Self, DomainError> { ... }
```

`Result`는 자동으로 must_use. Builder 반환값이나 새 인스턴스 반환 메서드에는 명시.

**근거**: [Rust Reference — `must_use` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute): "The must_use attribute is used to issue a diagnostic warning when a value is not 'used'"

---

## 13. Railway Oriented Programming (ROP)

ROP는 함수 체인을 "성공 트랙"과 "실패 트랙" 두 레일로 시각화하는 접근. Rust의 `Result<T, E>`가 자연스럽게 구현한다. 아래는 이 프로젝트의 ROP 원칙.

### Parse, don't validate

경계에서 unstructured 데이터를 typed 도메인 값으로 **변환(parse)**. 이후에는 타입 자체가 유효성을 보장하므로 재검증 불필요.

```rust
// GOOD: 경계에서 parse — 이후로는 유효성 보장됨
pub fn new(value: impl Into<String>) -> Result<Prefix, DomainError> {
    let s = value.into();
    if !s.chars().all(|c| c.is_ascii_uppercase()) || s.len() < 2 || s.len() > 5 {
        return Err(DomainError::InvalidPrefix(s));
    }
    Ok(Self(s))
}

// 이후로는 Prefix 타입 자체가 유효함을 보장
fn generate_task_id(prefix: &Prefix, seq: u32) -> TaskId {
    // 재검증 금지 — 타입이 이미 유효성 보장
    TaskId(format!("{prefix}-{seq}"))
}

// BAD: validate — 매번 재검증 필요, typed 보장 없음
pub fn validate_prefix(s: &str) -> Result<(), DomainError> { ... }
```

- 경계(entrypoint, adapter): 원시 데이터 → 도메인 타입으로 parse
- 도메인 내부: 이미 검증된 타입만 사용
- 잘못된 값을 **표현 불가능**하게 (enum으로 상태 모델링)

**근거**: 원문 ["Parse, don't validate" by Alexis King](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)

### 계층 경계의 에러 변환

각 계층은 **자신이 모르는 에러를 노출하지 않는다**. `From` 트레이트로 경계에서 변환:

```
[adapter] rusqlite::Error
    ↓ From 변환
[domain]  DomainError
    ↓ From 변환
[entrypoint] anyhow::Error → 사용자 메시지
```

```rust
// adapter 경계에서 rusqlite::Error → DomainError
impl From<rusqlite::Error> for DomainError {
    fn from(err: rusqlite::Error) -> Self { DomainError::Database(err) }
}

// domain 경계에서 DomainError → anyhow::Error (자동)
fn main() -> anyhow::Result<()> {
    let task = create_task(...)?;  // DomainError → anyhow::Error
    Ok(())
}
```

**원칙**: 하위 계층의 구체적 에러 타입이 상위 계층에 그대로 노출되면 안 됨.

### 콤비네이터 체인

`?` 연산자는 early-return을 간결하게 하지만, 긴 파이프라인은 콤비네이터(`map`, `and_then`, `or_else`)로 표현하면 가독성이 높아짐:

```rust
// ? 연산자 스타일 (간단한 경우)
pub fn handle(cmd: CreateTask) -> Result<Task, DomainError> {
    let project = project_repo.find_by_id(&cmd.project_id)?
        .ok_or_else(|| DomainError::ProjectNotFound(cmd.project_id.clone()))?;
    let task_id = TaskId::new(&project.prefix, project.next_sequence())?;
    let task = Task::new_backlog(task_id, cmd.title, cmd.description, cmd.label)?;
    task_repo.save(&task)?;
    Ok(task)
}

// 콤비네이터 체인 (파이프라인 강조)
pub fn handle(cmd: CreateTask) -> Result<Task, DomainError> {
    project_repo.find_by_id(&cmd.project_id)?
        .ok_or_else(|| DomainError::ProjectNotFound(cmd.project_id.clone()))
        .and_then(|p| TaskId::new(&p.prefix, p.next_sequence()))
        .and_then(|id| Task::new_backlog(id, cmd.title, cmd.description, cmd.label))
        .and_then(|task| task_repo.save(&task).map(|_| task))
}
```

**선택 기준**:
- 단순 순차 실행 → `?` 연산자
- 각 단계가 변환/결합을 포함 → 콤비네이터
- 팀원 가독성 우선 → 둘 중 한 스타일로 통일

**근거**: [std::result::Result 콤비네이터](https://doc.rust-lang.org/std/result/enum.Result.html#method.and_then)

### 순수 함수와 side effect 격리

도메인 로직은 **입력만으로 출력을 결정하는 순수 함수**로 작성. I/O는 Repository로 분리.

```rust
// GOOD: Domain Service는 순수
impl MetricsCalculator {
    pub fn calculate(entries: &[LogEntry]) -> SessionMetrics {
        // 입력만으로 결정, I/O 없음
        ...
    }
}

// I/O는 application에서 조합
impl AnalyzeHandler {
    pub fn handle(&mut self, cmd: AnalyzeSession) -> Result<SessionMetrics, DomainError> {
        let entries = self.log_repo.list_by_session(&cmd.session_id)?;  // I/O
        let metrics = MetricsCalculator::calculate(&entries);  // 순수 계산
        self.metrics_repo.save(&metrics)?;  // I/O
        Ok(metrics)
    }
}
```

이 분리로 순수 함수는 **테스트가 쉬움** (mock 불필요, 입력 → 출력만 검증).

### Result 반환 기준

| 상황 | 반환 타입 | 예시 |
|---|---|---|
| 검증 실패 가능 (Value Object 생성) | `Result<T, DomainError>` | `Prefix::new`, `TaskId::new` |
| 규칙 위반 가능 (상태 전환) | `Result<T, DomainError>` | `Task::move_to` |
| I/O (Repository, 파일) | `Result<T, E>` | `find_by_id`, `save` |
| 순수 계산 (Domain Service) | `T` (Result 불필요) | `MetricsCalculator::calculate` |
| 절대 실패하지 않음 | `T` | `Status::is_terminal` |

**원칙**: Result를 남용하지 않음. 실패 가능성이 없는 연산에 `Result`를 붙이면 호출자가 불필요한 처리를 강요받음.

---

## 예외

이 컨벤션이 적용되지 않는 경우:

- **프로토타입/실험 코드**: 빠른 검증이 우선
- **자동 생성 코드**: 도구 출력 그대로 유지
- **외부 API 통합**: 제약에 따라 규칙 완화
- **테스트 코드**: AAA 패턴 외에는 규칙 완화 가능

각 예외는 주석으로 이유를 명시한다.
