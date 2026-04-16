---
name: seogi-rust-convention
description: seogi 프로젝트의 Rust 코딩 컨벤션. Rust 파일을 작성하거나 수정할 때 참조. 함수형 3계층 아키텍처, ROP 원칙, 네이밍 규칙 포함. 전체 참조는 docs/conventions.md.
---

# Seogi Rust 컨벤션 (Quick Reference)

전체 상세: `docs/conventions.md` 참조

## 1. 아키텍처 (함수형 3계층)

```
entrypoint/   외부 인터페이스 (CLI, 훅)
    ↓
workflow/     샌드위치 조립 (I/O → 순수 → I/O)
    ↓
┌─────────────┬──────────────┐
│ domain/     │  adapter/    │
│ 순수 데이터 │  I/O 함수    │
│ + 순수 함수 │  (DB 액세스) │
└─────────────┴──────────────┘
```

**핵심 규칙:**
- Repository trait/Handler struct **없음** — 모듈 + 함수로 조직
- domain은 I/O 없음, DB 모름 (허용: serde, thiserror, chrono, uuid)
- adapter는 domain 타입을 사용 (import OK)
- workflow는 Impureim Sandwich로 조립

## 2. ROP 핵심 원칙

### Dependency Rejection
순수 함수는 의존성 없음. **데이터**를 미리 가져와서 넘김:

```rust
// BAD: 불순 함수 주입
fn process(load: impl Fn() -> Data, ...) { ... }

// GOOD: 데이터만 받기
fn process(data: &Data, ...) { ... }
```

### Impureim Sandwich
workflow는 3층 구조:

```rust
pub fn run(conn: &mut Connection, id: &str) -> Result<Metrics, Error> {
    let entries = log_repo::list_by_session(conn, id)?;  // 불순 top
    let metrics = metrics::calculate(&entries);          // 순수 middle
    metrics_repo::save(conn, &metrics)?;                 // 불순 bottom
    Ok(metrics)
}
```

### Parse, don't validate
경계에서 typed 도메인 값으로 변환, 내부는 재검증 금지:

```rust
impl Prefix {
    pub fn new(s: impl Into<String>) -> Result<Self, DomainError> { ... }
}
// 이후 Prefix 타입 자체가 유효성 보장
```

### 계층 경계 에러 변환
`adapter (rusqlite::Error) → domain (DomainError) → entrypoint (anyhow)` 로 From 변환.

## 3. 네이밍

| 종류 | 스타일 |
|---|---|
| 모듈/함수/변수 | `snake_case` |
| 타입/트레이트/Enum | `UpperCamelCase` |
| 상수 | `SCREAMING_SNAKE_CASE` |

**함수 접두사**:
- 동사로 시작 (`create_`, `find_`, `list_`)
- Boolean: `is_/has_/can_/should_`

**adapter 쿼리 함수**: `find_` (단일), `list_` (복수), `save`, `delete`

**변환 함수**: `as_` (저비용), `to_` (할당), `into_` (이동)

**모듈 기반 네이밍** — 접미사 중복 제거:
```rust
// BAD: CreateTaskCommand
// GOOD: command::CreateTask
```

**예약어 회피**: `type` → `kind`, `move` → `transfer`

## 4. 데이터 타입

### Value Object (Newtype)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl std::fmt::Display for TaskId { ... }
```

**필수 derive**: Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize

### Entity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task { /* 비공개 필드 */ }

impl Task {
    pub fn new_backlog(...) -> Result<Self, DomainError> { ... }

    // 상태 전환: self 소모
    pub fn move_to(self, new_status: StatusId) -> Result<Self, DomainError> {
        Ok(Self { status: new_status, ..self })
    }
}
```

## 5. 에러 처리

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(TaskId),
    #[error("database error")]
    Database(#[from] rusqlite::Error),
}
```

- `thiserror` → domain, adapter, workflow
- `anyhow` → entrypoint, main.rs만
- `unwrap()` 프로덕션 금지 (`?` 또는 `expect("reason")` 사용)

## 6. 함수 조합

**기본은 `?` 연산자**:

```rust
fn run(...) -> Result<Task, Error> {
    let project = project_repo::find(conn, &id)?;
    let task = Task::new(project.prefix)?;
    task_repo::save(conn, &task)?;
    Ok(task)
}
```

**콤비네이터는 단순 변환에만**:

```rust
let tasks: Result<Vec<Task>, _> = ids.iter()
    .map(|id| find(conn, id))
    .collect();
```

## 7. 불변성

- Rust의 불변성은 "공유된 가변 상태 금지"
- 소유한 데이터의 내부 mutation은 **허용** (관례적)
- 엔티티 상태 전환은 `self` 소모 → 새 인스턴스

```rust
// GOOD: 내부 mutation
fn add(items: &[T], new: T) -> Vec<T> {
    let mut r = items.to_vec();
    r.push(new);  // 소유한 데이터 내부 변경 OK
    r
}

// BAD: 외부 상태 변경
fn add_bad(items: &mut Vec<T>, new: T) { items.push(new); }
```

## 8. 크기 제한

- 함수: 50줄 max
- 파일: 800줄 max
- 중첩: 4단계 max
- 파라미터: 5개 max (builder 예외)

## 9. 임포트 순서

```rust
use std::path::Path;           // std

use anyhow::Result;            // 외부
use rusqlite::Connection;

use crate::domain::task::Task; // 내부
```

함수 안 `use` 금지. `crate::` 절대 경로 사용.

## 10. sync 사용

seogi는 CLI + 훅 위주. **동기(sync)** 코드. `async/await` 사용 안 함.

## 11. Cohesion & Coupling

- **응집도**: 관련 함수를 같은 모듈에 배치 (`domain/metrics.rs`에 `calculate`, `read_before_edit`)
- **결합도**: 모든 의존성이 함수 시그니처에 명시됨 (숨겨진 의존성 없음)

## 12. 자주 실수하는 것

- ❌ Repository trait 정의 (ROP 방식에서는 불필요)
- ❌ Handler struct 정의 (workflow 모듈 함수로 대체)
- ❌ `~Service`, `~Manager` 접미사
- ❌ domain에 rusqlite 사용
- ❌ `unwrap()` 남용
- ❌ 원시 타입(String) 대신 Newtype 안 씀
- ❌ async 사용
- ❌ 함수 안 `use` 선언
