# Feature 31: project→workspace 외부 인터페이스 리네이밍

## 목적

Phase 6 세 번째 단계. CLI 명령어와 MCP 도구명에서 project를 workspace로 변경한다.
이 Feature는 리팩토링이며 ground-truth에 직접 기여하지 않는다.
Phase 6 완료는 workspace 단위 지표 집계(목적 1)와 workspace별 baseline 비교(목적 2)의 전제 조건이다.

## 입력

- 사용자 입력: 없음 (리팩토링)
- 시스템 입력: SEO-16에서 리네이밍된 domain/adapter/workflow 코드 — 필수

## 출력

- 반환값: 없음 (리팩토링)
- 부수효과: 외부 인터페이스 리네이밍 (런타임 동작 변경 없음). 상세 변경 목록은 아래 참조.

## 변경 목록

### 1. 파일 리네이밍

| 현재 경로 | 변경 경로 |
|-----------|-----------|
| `entrypoint/project.rs` | `entrypoint/workspace.rs` |
| `tests/project_command_test.rs` | `tests/workspace_command_test.rs` |

`entrypoint/mod.rs`: `pub mod project` → `pub mod workspace`

### 2. CLI 서브커맨드 (`main.rs`)

| 현재 | 변경 |
|------|------|
| `Commands::Project { action }` | `Commands::Workspace { action }` |
| `enum ProjectAction` | `enum WorkspaceAction` |
| `ProjectAction::Create` | `WorkspaceAction::Create` |
| `ProjectAction::List` | `WorkspaceAction::List` |
| 도움말 "프로젝트 관리/생성/목록" | "워크스페이스 관리/생성/목록" |

### 3. CLI 플래그 (`main.rs`)

| 현재 | 변경 | 사용처 |
|------|------|--------|
| `--project` | `--workspace` | `task create`, `task list`, `report` |

### 4. MCP 도구명 (`entrypoint/mcp.rs`)

| 현재 | 변경 |
|------|------|
| `project_create` | `workspace_create` |
| `project_list` | `workspace_list` |
| `struct ProjectCreateParams` | `struct WorkspaceCreateParams` |
| `async fn project_create()` | `async fn workspace_create()` |
| `async fn project_list()` | `async fn workspace_list()` |

### 5. MCP 파라미터 (`entrypoint/mcp.rs`)

`TaskCreateParams`와 `TaskListParams`의 `project` 필드 → `workspace` 필드.

### 6. 테스트 파일

- `tests/workspace_command_test.rs` (파일명 변경): CLI `"project"` → `"workspace"` 서브커맨드
- `tests/mcp_tools_test.rs`: `"project_create"` → `"workspace_create"`, `"project_list"` → `"workspace_list"`
- `tests/task_command_test.rs`: `"project"` 서브커맨드, `"--project"` 플래그 → `"--workspace"`
- `tests/report_command_test.rs`: `create_project()` → `create_workspace()`, `"--project"` → `"--workspace"`

## 성공 시나리오

1. 변경 목록 1~6을 적용한다
2. `cargo build` 컴파일 성공
3. 기존 테스트 스위트 전체 통과로 동작 보존을 확인한다
4. `cargo clippy` 경고 없음

## 실패 시나리오

이 Feature는 순수 리네이밍 리팩토링이므로 새로운 런타임 실패 경로는 발생하지 않는다.
기존 실패 경로(invalid prefix, duplicate name 등)는 명령어명/도구명만 변경되며 동작은 동일하다.

## 제약 조건

- domain/adapter/workflow 계층 코드는 변경하지 않음 (SEO-16에서 완료)
- DB 스키마는 변경하지 않음 (SEO-15에서 완료)
- glossary.md 업데이트는 SEO-18에서 일괄 처리

## 의존하는 기능

- Feature 30 (SEO-16): 코드 리네이밍 완료 필수

---

## QA 목록

### CLI

1. `seogi workspace create --name Test --prefix TST --goal goal` 실행 시 workspaces 테이블에 한 행 추가됨
2. `seogi workspace create` 중복 prefix 시 에러 메시지 출력 및 exit code 1
3. `seogi workspace list` 실행 시 workspace 목록 테이블 출력됨
4. `seogi workspace list --json` 실행 시 JSON 배열 출력됨
5. `seogi workspace list` 빈 DB에서 실행 시 빈 목록 출력됨
6. `seogi task create --workspace Seogi --title t --description d --label feature` 실행 시 태스크 생성됨
7. `seogi task list --workspace Seogi` 실행 시 해당 workspace의 태스크만 필터링됨
8. `seogi report --from 2026-01-01 --to 2026-12-31 --workspace Seogi` 실행 시 해당 workspace의 리포트 출력됨

### MCP

9. MCP `workspace_create` 도구 호출 시 workspace가 생성되고 JSON 응답 반환됨
10. MCP `workspace_list` 도구 호출 시 workspace 목록 JSON 응답 반환됨
11. MCP `task_create`의 `project` 파라미터가 `workspace`로 변경됨
12. MCP `task_list`의 `project` 파라미터가 `workspace`로 변경됨
13. MCP tools/list 응답에 `workspace_create`, `workspace_list`가 포함되고 `project_create`, `project_list`가 없음

---

## Test Pyramid

| # | QA 항목 | 레벨 | 이유 |
|---|---------|------|------|
| 1-5 | CLI workspace CRUD | E2E | 기존 project_command_test 리네이밍 |
| 6-8 | CLI --workspace 플래그 | E2E | 기존 task_command/report_command 테스트 리네이밍 |
| 9-13 | MCP 도구명/파라미터 | E2E | 기존 mcp_tools_test 리네이밍 |

모든 항목이 E2E (바이너리/MCP 서버 호출). QA 11-12는 단위 테스트로도 가능하나,
기존 E2E 테스트(mcp_tools_test)에서 이미 커버되므로 중복 방지를 위해 E2E 유지.
기존 테스트 리네이밍이 주 작업이므로 새 테스트 추가는 불필요.
