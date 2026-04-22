# Feature 26: 단일 태스크 조회

## 목적

`seogi task get <id>`로 단일 태스크의 상세 정보(description 포함)를 조회하는 인터페이스를 제공한다. 에이전트가 작업 시작 시 해당 태스크의 description을 읽는 용도.

## 입력

### `seogi task get`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `task_id` | String (positional) | O | 태스크 ID (e.g., SEO-1) |
| `--json` | bool (flag) | X | JSON 형식으로 출력 |

### MCP: `task_get`

| 파라미터 | 타입 | 필수 | 설명 |
|----------|------|------|------|
| `task_id` | String | O | 태스크 ID (e.g., SEO-1) |

## 출력

### 성공 (기본)

```
ID:          SEO-1
Title:       MCP 서버 부트스트랩
Description: ## 목표
             rmcp 크레이트를 사용하여 ...
Label:       feature
Status:      backlog
Project:     Seogi
Created:     2026-04-19T02:51:54+00:00
Updated:     2026-04-19T03:41:15+00:00
```

### 성공 (--json)

```json
{
  "id": "SEO-1",
  "title": "MCP 서버 부트스트랩",
  "description": "## 목표\nrmcp 크레이트를 사용하여 ...",
  "label": "feature",
  "status_name": "backlog",
  "project_name": "Seogi",
  "created_at": "2026-04-19T02:51:54+00:00",
  "updated_at": "2026-04-19T03:41:15+00:00"
}
```

### 성공 (MCP)

MCP는 항상 JSON으로 반환. `--json` 출력과 동일한 구조.

### 실패

```
Error: Task not found: "SEO-99"
```

## 시나리오

### 성공

1. **기본 출력**: `seogi task get SEO-1` → 키-값 형식으로 상세 정보 출력.
2. **JSON 출력**: `seogi task get SEO-1 --json` → JSON 형식으로 출력.
3. **MCP 조회**: `task_get(task_id: "SEO-1")` → JSON 응답.

### 실패

1. **존재하지 않는 태스크**: `seogi task get SEO-99` → 에러.
2. **MCP 미존재 태스크**: `task_get(task_id: "SEO-99")` → MCP 에러 응답.

## 제약 조건

- **읽기 전용**: DB 상태를 변경하지 않는다.
- **TaskListRow 재사용**: 기존 `list_all` 쿼리의 JOIN 패턴과 `TaskListRow` 구조체를 재사용한다.
- **description 전체 출력**: list와 달리 description을 생략 없이 전체 출력한다.

## 의존성

- Feature 14 (Task create/list): `TaskListRow`, `task_repo`, `task_list_row_from_row` 재사용.

---

## QA 목록

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q1 | `task_repo::find_by_id_detailed` 존재하는 id → `Some(TaskListRow)` |
| Q2 | `task_repo::find_by_id_detailed` 없는 id → `None` |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q3 | `get` 존재하는 태스크 → `Ok(TaskListRow)` 반환 |
| Q4 | `get` 없는 태스크 → `DomainError` 반환 |

### E2E (CLI)

| # | 검증 항목 |
|---|----------|
| Q5 | `seogi task get SEO-1` 키-값 형식 출력, 모든 필드 포함 |
| Q6 | `seogi task get SEO-1 --json` JSON 출력, 모든 필드 포함 |
| Q7 | `seogi task get SEO-99` 에러 메시지 출력 |

### E2E (MCP)

| # | 검증 항목 |
|---|----------|
| Q8 | `task_get(task_id: "SEO-1")` JSON 응답, 모든 필드 포함 |
| Q9 | `task_get(task_id: "SEO-99")` 에러 응답 |

---

## Test Pyramid

### Integration (Adapter + Workflow) — 4건

```
test_task_repo_find_by_id_detailed_found       → Q1
test_task_repo_find_by_id_detailed_not_found   → Q2
test_get_task_success                          → Q3
test_get_task_not_found                        → Q4
```

### E2E (CLI + MCP) — 5건

```
test_task_get_default_output                   → Q5
test_task_get_json_output                      → Q6
test_task_get_not_found                        → Q7
test_mcp_task_get_success                      → Q8
test_mcp_task_get_not_found                    → Q9
```

---

## 완료 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 목록 9건 작성 완료
- [x] Test Pyramid 9건 (Integration 4 + E2E 5)
- [ ] 사용자 승인

승인일:
