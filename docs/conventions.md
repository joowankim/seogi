# Seogi 코딩 컨벤션

Rust 기반 seogi 프로젝트의 코딩 컨벤션. Railway Oriented Programming(ROP)과 함수형 아키텍처를 중심으로 한다.

모든 설계와 구현은 이 문서를 따른다. 예외는 명시적으로 문서화한다.

**주요 참고 자료:**
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Reference](https://doc.rust-lang.org/reference/)
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Scott Wlaschin — F# for Fun and Profit](https://fsharpforfunandprofit.com/)
- [Mark Seemann — Dependency Rejection](https://blog.ploeh.dk/2017/02/02/dependency-rejection/)
- [Gary Bernhardt — Boundaries](https://www.destroyallsoftware.com/talks/boundaries)

---

## 1. 네이밍 컨벤션

### 식별자 스타일

| 종류 | 스타일 | 예시 |
|---|---|---|
| 모듈/함수/변수 | `snake_case` | `fn create_task`, `task_count` |
| 타입/트레이트/Enum | `UpperCamelCase` | `Task`, `Status` |
| 상수/static | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT` |
| 매크로 | `snake_case!` | `println!`, `vec!` |
| 타입 파라미터 | 간결한 `UpperCamelCase` | `T`, `R`, `Key` |
| 라이프타임 | 짧은 `lowercase` | `'a`, `'de` |

**근거**: [Rust API Guidelines — C-CASE](https://rust-lang.github.io/api-guidelines/naming.html)

### 함수 네이밍

- **동사로 시작**: `create_task`, `find_by_id`, `validate_prefix`
- **Boolean 반환**: `is_`, `has_`, `can_`, `should_` 접두사 — `is_empty`, `has_items`
- **컬렉션**: 복수형 — `tasks`, `tool_uses`

### 쿼리 함수 네이밍 (repo 모듈에서)

| 접두사 | 반환 | 예시 |
|---|---|---|
| `find_` | `Option<Entity>` | `find_by_id`, `find_by_session` |
| `list_` | `Vec<Entity>` | `list_by_workspace`, `list_active` |
| `save` | `Result<()>` | `save_tool_use` |
| `delete` | `Result<()>` | `delete_by_id` |

### 변환 함수 네이밍

| 접두사 | 의미 | 비용 | 예시 |
|---|---|---|---|
| `as_` | 저비용 참조 변환 | O(1) | `as_str()`, `as_ref()` |
| `to_` | 복사/할당 변환 | O(n) | `to_string()`, `to_vec()` |
| `into_` | 소유권 이동 변환 | 가변 | `into_iter()` |

**근거**: [C-CONV](https://rust-lang.github.io/api-guidelines/naming.html#ad-hoc-conversions-follow-as_-to_-into_-conventions-c-conv)

### 모듈 기반 네이밍

타입/함수 이름에 접미사 중복을 피한다:

```rust
// BAD
use crate::workflow::create_task_workflow::CreateTaskCommand;
let cmd = CreateTaskCommand { ... };

// GOOD
use crate::workflow::create_task;
use crate::workflow::command;
let cmd = command::CreateTask { ... };
```

### 예약어 회피

Rust 키워드와 충돌 방지:

| 키워드 | 대체 |
|---|---|
| `type` | `kind`, `category` |
| `ref` | `reference`, `target` |
| `move` | `transfer`, `relocate` |
| `self` (변수로) | `this`, `entity` |

**근거**: [The Rust Reference — Keywords](https://doc.rust-lang.org/reference/keywords.html)

---

## 2. 아키텍처 (함수형 3계층)

### 계층 구조

```
entrypoint/      외부 인터페이스 (CLI, 훅)
    ↓
workflow/        샌드위치 조립 (I/O → 순수 → I/O)
    ↓
┌─────────────┬──────────────┐
│ domain/     │  adapter/    │
│ 순수 데이터  │  I/O 함수    │
│ + 순수 함수 │  (DB 액세스)  │
└─────────────┴──────────────┘
```

### 계층 역할

**entrypoint/**
- CLI 명령어 파싱 (clap)
- 훅 stdin 파싱
- workflow 함수 호출
- 에러를 사용자 메시지로 변환

**workflow/**
- 한 유스케이스당 한 함수 (`analyze::run`, `report::run`)
- **Impureim Sandwich** 조립: I/O → 순수 계산 → I/O
- 실패 경로는 `?`로 자동 전파

**domain/**
- 순수 데이터 타입 (Entity, Value Object, Command, Query)
- 순수 함수 (상태 전환, 계산, 검증)
- I/O 없음, DB 모름
- 허용 크레이트: serde, thiserror, chrono, uuid

**adapter/**
- DB 액세스 함수 (`log_repo::save`, `metrics_repo::list_by_range`)
- 파일 I/O, 외부 API 등
- 연결/세션 관리

### 의존성 방향

```
entrypoint  → workflow → domain
                ↓
             adapter → domain (데이터 타입 사용)
```

- **domain은 아무것도 의존하지 않음** (순수)
- **adapter는 domain 타입을 사용** (Entity를 반환하려면 domain을 import)
- **workflow는 adapter와 domain을 조립**
- **entrypoint는 workflow를 호출**

### 계층 간 경계

Shell(불순 계층)과 Core(순수 계층) 사이는 **단순한 값**만 주고받는다.
- 도메인 타입 (`LogEntry`, `SessionMetrics`, `Task`)
- Command/Query DTO
- 원시 타입 (문자열, 숫자)

**금지**: live connection, file handle, trait object를 도메인에 전달.

---

## 3. ROP 원칙 (핵심 철학)

ROP는 함수 체인을 "성공 트랙"과 "실패 트랙" 두 레일로 시각화하는 접근. Rust의 `Result<T, E>` + `?` 연산자가 이를 구현한다.

### 3-1. Dependency Rejection

**Mark Seemann의 원칙:**
> 의존성은 본질적으로 불순하다. 순수 함수는 불순 함수를 호출할 수 없으므로, **순수 함수는 의존성을 가질 수 없다**.

실제로는 "의존성"을 파라미터로 넘기지 말고, **필요한 데이터를 미리 가져와서** 넘긴다.

```rust
// BAD: Dependency Injection (불순 함수를 파라미터로 주입)
fn try_accept(
    read_reservations: impl Fn() -> Vec<Reservation>,  // I/O 숨어있음
    reservation: Reservation,
) -> Option<Reservation> { ... }

// GOOD: Dependency Rejection (데이터만 받음)
fn try_accept(
    reservations: &[Reservation],  // 이미 읽어온 데이터
    reservation: Reservation,
) -> Option<Reservation> { ... }
```

**참고**: [Mark Seemann — Dependency Rejection](https://blog.ploeh.dk/2017/02/02/dependency-rejection/)

### 3-2. Impureim Sandwich

**구조:**
```
┌─────────────────────────┐
│ Impure (Top)            │  I/O, DB 읽기, 설정 로드
├─────────────────────────┤
│ Pure (Middle)           │  비즈니스 로직, 결정
├─────────────────────────┤
│ Impure (Bottom)         │  DB 쓰기, 알림 전송
└─────────────────────────┘
```

**seogi workflow 예시:**

```rust
// workflow/analyze.rs
pub fn run(conn: &mut Connection, session_id: &str) -> Result<SessionMetrics, Error> {
    // [Top: Impure] DB에서 로그 읽기
    let entries = log_repo::list_by_session(conn, session_id)?;

    // [Middle: Pure] 순수 계산
    let metrics = metrics::calculate(&entries);

    // [Bottom: Impure] DB에 저장
    metrics_repo::save(conn, &metrics)?;

    Ok(metrics)
}
```

**참고**: [Mark Seemann — Impureim Sandwich](https://blog.ploeh.dk/2020/03/02/impureim-sandwich/)

### 3-3. Functional Core, Imperative Shell

**Gary Bernhardt의 원칙:**

| | 의존성 | 분기 |
|---|---|---|
| Shell | 많음 (DB, HTTP, ...) | 적음 |
| Core | 없음 | 많음 |

- 비즈니스 로직 복잡성 → Core (순수)
- 기술적 복잡성 → Shell (불순)

**이점:**
- Core는 mock 없이 테스트 가능
- Shell은 통합 테스트 몇 개로 충분
- 책임이 명확히 분리됨

**참고**: [Gary Bernhardt — Boundaries](https://www.destroyallsoftware.com/talks/boundaries)

### 3-4. Parse, don't validate

경계에서 unstructured 데이터를 typed 도메인 값으로 **변환(parse)**. 이후에는 타입이 유효성을 보장.

```rust
// GOOD: 경계에서 parse
impl Prefix {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let s = value.into();
        if !s.chars().all(|c| c.is_ascii_uppercase()) || s.len() < 2 || s.len() > 5 {
            return Err(DomainError::InvalidPrefix(s));
        }
        Ok(Self(s))
    }
}

// 이후로는 Prefix 타입 자체가 유효성 보장 — 재검증 금지
fn generate_task_id(prefix: &Prefix, seq: u32) -> TaskId {
    TaskId(format!("{prefix}-{seq}"))
}
```

**참고**: [Parse, don't validate — Alexis King](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)

### 3-5. 계층 경계의 에러 변환

각 계층은 **자신이 모르는 에러를 노출하지 않는다**.

```
adapter (rusqlite::Error)
    ↓ From 변환
domain (DomainError)
    ↓ From 변환
entrypoint (anyhow::Error → 사용자 메시지)
```

```rust
impl From<rusqlite::Error> for DomainError {
    fn from(err: rusqlite::Error) -> Self { DomainError::Database(err) }
}
```

### 3-6. Result 반환 기준

| 상황 | 반환 타입 |
|---|---|
| 검증 실패 가능 (Value Object 생성) | `Result<T, DomainError>` |
| 규칙 위반 가능 (상태 전환) | `Result<T, DomainError>` |
| I/O (adapter 함수) | `Result<T, E>` |
| 순수 계산 | `T` (Result 불필요) |
| 절대 실패하지 않음 | `T` |

**원칙**: Result를 남용하지 않는다. 실패 가능성이 없는 연산에 Result를 붙이면 호출자가 불필요한 처리를 강요받는다.

---

## 4. 함수 조합과 체이닝

### 기본: `?` 연산자 (Rust idiomatic railway)

```rust
fn run(conn: &mut Connection, cmd: CreateTask) -> Result<Task, Error> {
    let workspace = workspace_repo::find(conn, &cmd.workspace_id)?;
    let task_id = TaskId::new(&workspace.prefix, workspace.next_seq())?;
    let task = Task::new_backlog(task_id, cmd.title)?;
    task_repo::save(conn, &task)?;
    Ok(task)
}
```

`?`는 F#의 `bind`에 해당하는 Rust 표현. 실패 시 즉시 실패 트랙으로 이탈.

**근거**: [Rust Book — Recoverable Errors with Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)

### 콤비네이터 (특정 상황에만)

단순 변환 파이프라인에만 사용:

```rust
// 컬렉션 변환
let tasks: Result<Vec<Task>, Error> = ids.iter()
    .map(|id| task_repo::find(conn, id))
    .collect();  // Vec<Result>를 Result<Vec>으로

// Option 체이닝
let title = description.as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or("Untitled");
```

### 선택 기준

- **`?` 기본**: 여러 단계 순차 실행, 중간 변수 필요
- **콤비네이터**: 단일 값 변환, 중간 변수 불필요
- **혼용 지양**: 한 함수 안에서 스타일 통일

---

## 5. 데이터 타입

### Entity

```rust
// domain/task.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    id: TaskId,
    title: String,
    description: String,
    status: StatusId,
    workspace_id: WorkspaceId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// 상태 전환: self 소모 → Result<Self, Error>
impl Task {
    pub fn move_to(self, new_status: StatusId) -> Result<Self, DomainError> {
        Ok(Self {
            status: new_status,
            updated_at: Utc::now(),
            ..self
        })
    }
}
```

**원칙:**
- 필드는 비공개, 생성은 factory 함수로
- 불변 (상태 변경은 새 인스턴스 반환)
- 비즈니스 로직은 엔티티 메서드 또는 모듈 함수

### Value Object (Newtype 패턴)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(prefix: &Prefix, sequence: u32) -> Self {
        Self(format!("{prefix}-{sequence}"))
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

**Newtype 필수 derive**: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Display`, `Serialize`, `Deserialize`

**근거**:
- [Rust Book — Advanced Types (Newtype)](https://doc.rust-lang.org/book/ch20-03-advanced-types.html)
- [C-NEWTYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#newtypes-provide-static-distinctions-c-newtype)
- [C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#types-eagerly-implement-common-traits-c-common-traits)

### Command/Query DTO

workflow 함수의 입력 타입:

```rust
// domain/command.rs
#[derive(Debug)]
pub struct CreateTask {
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub description: String,
    pub label: Label,
}

// domain/query.rs
#[derive(Debug)]
pub struct ListTasks {
    pub workspace_id: Option<WorkspaceId>,
    pub status: Option<StatusId>,
}
```

Entity를 외부에 직접 노출하지 않음 — 필요 시 별도 Response 타입으로 변환.

### Enum에 동작 추가

```rust
impl StatusCategory {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Canceled)
    }

    pub fn can_transition_to(&self, next: &StatusCategory) -> bool {
        // 카테고리 전환 규칙
    }
}
```

---

## 6. 에러 처리

### thiserror로 도메인 에러 정의

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("task not found: {0}")]
    TaskNotFound(TaskId),

    #[error("invalid status transition: {from} → {to}")]
    InvalidTransition { from: StatusId, to: StatusId },

    #[error("invalid prefix: {0}")]
    InvalidPrefix(String),

    #[error("database error")]
    Database(#[from] rusqlite::Error),
}
```

### anyhow는 entrypoint 계층에서만

- `main.rs`, `entrypoint/`에서만 `anyhow::Result`
- `domain/`, `adapter/`, `workflow/`는 구체적 에러 타입

### From 트레이트로 자동 변환

`?` 연산자가 `From::from`을 호출하여 자동 변환:

```rust
impl From<rusqlite::Error> for DomainError {
    fn from(err: rusqlite::Error) -> Self { DomainError::Database(err) }
}

// 사용
fn load(conn: &Connection, id: &TaskId) -> Result<Task, DomainError> {
    let row = conn.query_row(...)?;  // rusqlite::Error → DomainError 자동 변환
    Ok(row_to_task(&row)?)
}
```

**근거**:
- [std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html)
- [Rust Book — ? Operator and From](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)

### unwrap() 금지 (프로덕션 코드)

- 프로덕션: `?` 또는 명시적 에러 처리
- 테스트: `unwrap()` 허용
- 불변식 명확: `expect("reason")` 사용

### 민감정보 노출 금지

에러 메시지에 비밀번호, 토큰 포함 금지.

---

## 7. 불변성

Rust의 불변성은 **"공유된 가변 상태 금지"**에 가깝지, "모든 변경 금지"가 아니다.

### 허용되는 내부 mutation

소유한 데이터의 내부 변경은 관례적이고 권장됨:

```rust
// GOOD
fn add_item(items: &[Item], new_item: Item) -> Vec<Item> {
    let mut result = items.to_vec();
    result.push(new_item);
    result
}

fn calculate_total(items: &[Item]) -> Money {
    let mut total = Money::zero();
    for item in items {
        total = total.add(&item.price);
    }
    total
}
```

### 금지: 공유된 참조를 통한 변경

도메인 함수에서 외부 상태를 변경하지 말 것:

```rust
// BAD: 외부 상태 변경
fn add_item_bad(items: &mut Vec<Item>, new_item: Item) {
    items.push(new_item);
}
```

### 엔티티 상태 전환: self 소모

```rust
// GOOD: self 소모, 새 인스턴스 반환
impl Task {
    pub fn move_to(self, new_status: StatusId) -> Result<Self, DomainError> {
        Ok(Self { status: new_status, ..self })
    }
}
```

**근거**:
- [Rust Book — References and Borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [Rust Book — Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)

---

## 8. 하드코딩 금지

### 상수 정의

```rust
const DB_PATH: &str = "~/.seogi/seogi.db";
const DOOM_LOOP_THRESHOLD: u32 = 5;
```

### 매직 스트링은 Enum

```rust
// BAD
if status == "in_progress" { ... }

// GOOD
if status == Status::InProgress { ... }
```

### 환경 변수로 설정

민감정보/환경별 설정은 `std::env::var`로 로드.

---

## 9. 함수/파일 크기

- **함수**: 20줄 권장, 50줄 최대
- **파일**: 200~400줄 권장, 800줄 최대
- **중첩 깊이**: 4단계 max
- **파라미터**: 3개 권장, 5개 최대

### 파라미터가 많으면 구조체 또는 builder

```rust
// BAD
pub fn create_task(p: WorkspaceId, t: String, d: String, l: Label, s: StatusId) { ... }

// GOOD
pub fn create_task(cmd: CreateTask) { ... }
```

**근거**: [Clippy — too_many_arguments](https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments) (기본 임계값 7, 이 프로젝트는 5)

---

## 10. 테스트

### TDD 사이클

상세 워크플로우: [docs/tdd-cycle.md](./tdd-cycle.md) 참조

### AAA 패턴

```rust
#[test]
fn move_task_from_backlog_to_in_progress() {
    // Arrange
    let task = Task::new_backlog(...).unwrap();

    // Act
    let moved = task.move_to(StatusId::in_progress()).unwrap();

    // Assert
    assert_eq!(moved.status(), &StatusId::in_progress());
}
```

### 객체 전체 비교

```rust
// GOOD: 전체 비교 — 필드 추가 시 자동 검출
let expected = Task {
    id: actual.id().clone(),
    title: "test".to_string(),
    // ...
};
assert_eq!(actual, expected);
```

### Classicist 접근

Mock 대신 실제 구현체 사용:

```rust
#[test]
fn analyze_saves_metrics() {
    // 실제 인메모리 SQLite 사용
    let mut conn = Connection::open_in_memory().unwrap();
    apply_schema(&mut conn);

    // 실제 adapter 함수 호출
    log_repo::save_tool_use(&mut conn, &tool_use).unwrap();

    // workflow 실행
    let result = workflow::analyze::run(&mut conn, "session-1").unwrap();

    // 실제 DB 상태 검증
    let metrics = metrics_repo::find_latest(&conn, "session-1").unwrap();
    assert_eq!(metrics, result);
}
```

### 커버리지

- 최소 80%, 도메인 로직은 100% 지향
- 브랜치 커버리지 100% 목표 (`cargo llvm-cov --branch`)

---

## 11. 임포트 규칙

### 함수 안 `use` 금지

```rust
// BAD
fn process() {
    use std::fs::File;
    // ...
}

// GOOD
use std::fs::File;

fn process() { ... }
```

### 절대 경로

```rust
// GOOD
use crate::domain::task::Task;

// BAD
use super::super::task::Task;
```

### 임포트 순서

1. `std::`
2. 외부 크레이트
3. `crate::` (내부 모듈)

```rust
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::domain::task::Task;
use crate::domain::error::DomainError;
```

**근거**: [Rust Style Guide — Imports](https://doc.rust-lang.org/style-guide/)

---

## 12. Rust 특화 규칙

### 소유권 설계

필요한 최소 권한만 요구:

| 용도 | 시그니처 |
|---|---|
| 읽기 | `&T` |
| 수정 | `&mut T` |
| 소유권 이동 | `T` |
| 유연한 입력 | `impl Into<T>` / `impl AsRef<T>` |

```rust
// GOOD: 최소 권한
fn format_title(title: &str) -> String { title.to_uppercase() }

// BAD: 불필요한 소유권
fn format_title(title: String) -> String { title.to_uppercase() }
```

**근거**: [C-GENERIC](https://rust-lang.github.io/api-guidelines/flexibility.html#functions-minimize-assumptions-about-parameters-by-using-generics-c-generic)

### From/Into 트레이트

`From`을 구현하면 `Into`는 자동:

```rust
impl From<rusqlite::Error> for DomainError { ... }

// 제네릭 경계에서는 Into 사용
fn new_task<T: Into<String>>(title: T) -> Task { ... }
```

**근거**: [std::convert::From](https://doc.rust-lang.org/std/convert/trait.From.html)

### Display / Debug

엔티티와 Value Object는 두 트레이트 구현:
- `Debug`: 개발자용 (derive 충분)
- `Display`: 사용자 표시용 (직접 구현)

**근거**:
- [std::fmt::Debug](https://doc.rust-lang.org/std/fmt/trait.Debug.html)
- [std::fmt::Display](https://doc.rust-lang.org/std/fmt/trait.Display.html)

### #[must_use]

반환값을 무시하면 안 되는 함수/타입에:

```rust
#[must_use]
pub fn move_to(self, new_status: StatusId) -> Result<Self, DomainError> { ... }
```

**근거**: [Rust Reference — must_use](https://doc.rust-lang.org/reference/attributes/diagnostics.html)

### Exhaustive 패턴 매칭

비즈니스 enum에는 와일드카드(`_`) 회피:

```rust
// GOOD: 새 variant 추가 시 컴파일러 경고
match status {
    Status::Backlog => ...,
    Status::InProgress => ...,
    Status::Done => ...,
}
```

**근거**: [Rust Reference — non_exhaustive](https://doc.rust-lang.org/reference/attributes/type_system.html)

### sync (async 지양)

seogi는 CLI + 훅 위주라 **동기(sync)** 사용. `async/await` 미사용.

---

## 13. Cohesion과 Coupling

FP/ROP도 높은 응집도와 낮은 결합도를 지향한다. 단지 달성 방식이 다르다.

### 응집도 — 모듈로 묶음

관련 함수와 데이터를 **같은 모듈**에 배치:

```rust
// domain/metrics.rs — 메트릭 계산 관련 함수 모음
pub fn calculate(entries: &[LogEntry]) -> SessionMetrics {
    SessionMetrics {
        read_before_edit_ratio: read_before_edit(entries),
        doom_loop_count: doom_loop(entries),
        // ...
    }
}

fn read_before_edit(entries: &[LogEntry]) -> u32 { ... }
fn doom_loop(entries: &[LogEntry]) -> u32 { ... }
```

OOP의 클래스처럼 **모듈 + pub 제어**로 경계를 만든다.

### 결합도 — 시그니처에 드러남

모든 의존성이 함수 시그니처에 명시됨:

```rust
// FP: 결합도가 투명
workflow::analyze::run(conn, session_id)
//                   ↑
//                   이 함수가 conn과 session_id에만 의존함이 명확
```

숨겨진 의존성(생성자 주입, 전역 상태)이 없으므로 결합도 파악이 쉽다.

---

## 예외

이 컨벤션이 적용되지 않는 경우:

- **프로토타입/실험 코드**: 빠른 검증 우선
- **자동 생성 코드**: 도구 출력 그대로 유지
- **외부 API 통합**: 제약에 따라 규칙 완화
- **테스트 코드**: AAA 패턴 외 규칙 완화 가능

각 예외는 주석으로 이유를 명시한다.
