# Feature 30: project→workspace 코드 리네이밍

## 목적

Phase 6 두 번째 단계. domain/adapter/workflow 계층의 코드에서 project를 workspace로 리네이밍한다.
이 Feature는 리팩토링이며 ground-truth에 직접 기여하지 않는다.
Phase 6 완료는 workspace 단위 지표 집계(목적 1)와 workspace별 baseline 비교(목적 2)의 전제 조건이다.

## 입력

- 사용자 입력: 없음 (리팩토링)
- 시스템 입력: SQLite DB 스키마 v7 (workspaces 테이블) — 필수

## 출력

- 반환값: 없음 (리팩토링)
- 부수효과: 소스 파일 리네이밍 및 내부 식별자 변경 (런타임 동작 변경 없음). 상세 변경 목록은 아래 참조.

## 범위

| 계층 | 이 Feature에서 변경 | SEO-17에서 변경 |
|------|:---:|:---:|
| domain (타입, 함수, 필드) | O | - |
| adapter (repo, mapper) | O | - |
| workflow (함수, 변수) | O | - |
| entrypoint (import 경로만) | O | - |
| entrypoint (CLI 명령어명, MCP 도구명) | - | O |
| main.rs (import 경로만) | O | - |
| main.rs (subcommand 이름, --project 플래그) | - | O |

entrypoint/main.rs에서는 import 경로와 타입명만 변경하고, CLI 명령어명(`seogi project`)과 MCP 도구명(`project_create`)은 SEO-17에서 변경한다.

## 변경 목록

### 1. 파일 리네이밍 + mod.rs (4개 파일, 3개 mod.rs)

| 현재 경로 | 변경 경로 |
|-----------|-----------|
| `domain/project.rs` | `domain/workspace.rs` |
| `adapter/project_repo.rs` | `adapter/workspace_repo.rs` |
| `workflow/project.rs` | `workflow/workspace.rs` |

mod.rs 변경:
- `domain/mod.rs`: `pub mod project` → `pub mod workspace`
- `adapter/mod.rs`: `pub mod project_repo` → `pub mod workspace_repo`
- `workflow/mod.rs`: `pub mod project` → `pub mod workspace`

### 2. 도메인 타입 (`domain/workspace.rs`)

| 현재 | 변경 |
|------|------|
| `struct Project` | `struct Workspace` |
| `impl Project` | `impl Workspace` |
| `struct ProjectPrefix` | `struct WorkspacePrefix` |
| `impl ProjectPrefix` | `impl WorkspacePrefix` |
| `ProjectPrefix::from_name()` | `WorkspacePrefix::from_name()` |

### 3. 도메인 필드 (`domain/log.rs`)

ToolUse, ToolFailure, SystemEvent 각각:

| 현재 필드 | 변경 |
|-----------|------|
| `project: String` | `workspace: String` |
| `project_path: String` | `workspace_path: String` |
| `fn project(&self)` | `fn workspace(&self)` |
| `fn project_path(&self)` | `fn workspace_path(&self)` |

`extract_project_from_cwd()` → `extract_workspace_from_cwd()`

### 4. 도메인 필드 (`domain/task.rs`)

| 현재 | 변경 |
|------|------|
| `project_id: String` | `workspace_id: String` |
| `fn project_id(&self)` | `fn workspace_id(&self)` |

### 5. adapter 계층

**`adapter/workspace_repo.rs`** (파일명 변경):
- 함수 파라미터: `project: &Project` → `workspace: &Workspace`
- `project_id: &str` → `workspace_id: &str`

**`adapter/mapper.rs`**:
- `project_from_row()` → `workspace_from_row()`
- `ProjectPrefix::new()` → `WorkspacePrefix::new()`

**`adapter/log_repo.rs`**:
- `.project()` → `.workspace()`
- `.project_path()` → `.workspace_path()`

**`adapter/task_repo.rs`**:
- `TaskListRow.project_name` → `TaskListRow.workspace_name`
- `TaskFilter.project_name` → `TaskFilter.workspace_name`
- SQL alias: `p.name AS project_name` → `p.name AS workspace_name`
- `find_title_and_project()` → `find_title_and_workspace()`

### 6. workflow 계층

**`workflow/workspace.rs`** (파일명 변경):
- `Project::new()` → `Workspace::new()`
- `project_repo::` → `workspace_repo::`
- 변수: `project` → `workspace`, `project_prefix` → `workspace_prefix`

**`workflow/task.rs`**:
- `project_repo::` → `workspace_repo::`
- 변수: `project` → `workspace`
- 파라미터: `project_name` → `workspace_name`

**`workflow/log_tool.rs`, `log_failure.rs`, `log_system.rs`**:
- `extract_project_from_cwd` → `extract_workspace_from_cwd`
- 변수: `project` → `workspace`

**`workflow/report.rs`**:
- 파라미터: `project: Option<&str>` → `workspace: Option<&str>`
- 변수: `filter_project` → `filter_workspace`

### 7. entrypoint 계층 (import 경로 + 타입명만)

**`entrypoint/project.rs`**: import 경로와 타입명 변경 (파일명은 SEO-17)
**`entrypoint/task.rs`**: import 경로, 타입명, 파라미터명 변경
**`entrypoint/mcp.rs`**: import 경로, 타입명, struct 필드명 변경 (MCP 도구명은 SEO-17)
**`main.rs`**: import 경로, 타입명 변경 (subcommand/flag명은 SEO-17)

### 8. models.rs (JSONL 마이그레이션)

| 현재 | 변경 |
|------|------|
| `pub project: String` | `pub workspace: String` |
| `pub project_path: String` | `pub workspace_path: String` |

## 성공 시나리오

1. 변경 목록 1~8의 리네이밍을 적용한다
2. `cargo build` 컴파일 성공 (타입/모듈 경로 불일치 없음)
3. 기존 테스트 스위트 전체 통과로 동작 보존을 확인한다
4. `cargo clippy` 경고 없음

## 실패 시나리오

이 Feature는 순수 리네이밍 리팩토링이므로 새로운 런타임 실패 경로는 발생하지 않는다.
기존 실패 경로(invalid prefix, duplicate name 등)는 함수명/타입명만 변경되며 동작은 동일하다.

## 제약 조건

- CLI 명령어명(`seogi project`)과 MCP 도구명(`project_create`)은 변경하지 않음 (SEO-17 범위)
- `--project` CLI 플래그는 변경하지 않음 (SEO-17 범위)
- DB 스키마는 이미 SEO-15에서 변경 완료
- glossary.md의 `Project`/`ProjectPrefix` → `Workspace`/`WorkspacePrefix` 업데이트는 SEO-18에서 일괄 처리

## 의존하는 기능

- Feature 29 (SEO-15): DB 마이그레이션 완료 필수

---

## QA 목록

1. `Workspace::new(name, prefix, goal)` 후 `workspace_repo::save()` → `workspace_repo::find_by_prefix()`로 동일 name, prefix, goal 조회됨
2. `WorkspacePrefix::new("SEO")`가 유효한 prefix를 생성함
3. `WorkspacePrefix::from_name("Seogi")`가 자동 prefix를 생성함
4. `extract_workspace_from_cwd("/Users/kim/projects/seogi")`가 `"seogi"`를 반환함
5. `workspace_repo::save()` 후 `workspace_repo::list_all()`에 포함됨
6. `workspace_repo::find_by_prefix()`로 workspace 조회 가능
7. `workspace_from_row()`가 DB 행을 `Workspace` 타입으로 변환함
8. `TaskListRow.workspace_name` 필드에 workspace 이름이 포함됨
9. `find_title_and_workspace()`가 task의 title과 workspace name을 반환함
10. `ToolUse`에 workspace/workspace_path를 설정하면 `.workspace()`/`.workspace_path()`가 해당 값을 반환함
11. `Task`에 workspace_id를 설정하면 `.workspace_id()`가 해당 값을 반환함
12. `workflow::workspace::create()` 호출 시 workspace가 생성됨
13. `workflow::workspace::list()` 호출 시 workspace 목록 반환됨
14. cwd가 `/Users/kim/projects/seogi`인 훅 실행 후 `workspace` 컬럼에 `seogi`, `workspace_path` 컬럼에 `/Users/kim/projects/seogi`가 저장됨

---

## Test Pyramid

| # | QA 항목 | 레벨 | 이유 |
|---|---------|------|------|
| 1 | Workspace 생성+조회 | 통합 | 기존 project_repo 테스트 리네이밍 |
| 2-3 | WorkspacePrefix 유효성 | 단위 | 기존 ProjectPrefix 테스트 리네이밍 |
| 4 | extract_workspace_from_cwd | 단위 | 기존 테스트 리네이밍 |
| 5-6 | workspace_repo CRUD | 통합 | 기존 project_repo 테스트 리네이밍 |
| 7 | workspace_from_row | 통합 | 기존 mapper 테스트에서 커버 |
| 8-9 | TaskListRow/find_title_and_workspace | 통합 | 기존 task_repo 테스트 리네이밍 |
| 10-11 | ToolUse/Task getter | 단위 | 기존 도메인 테스트 리네이밍 |
| 12-13 | workflow create/list | 통합 | 기존 workflow 테스트 리네이밍 |
| 14 | 훅 데이터 저장 | E2E | 기존 E2E 테스트 리네이밍 |

기존 테스트의 리네이밍이 주 작업이므로 새 테스트 추가는 불필요.
