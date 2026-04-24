# Feature 35: Cycle-Task 배정 + 자동 포함 (SEO-22)

## 목적

태스크를 Cycle에 명시적으로 배정(assign/unassign)하고, active Cycle이 있을 때 자동으로 포함시키는 로직을 구현한다. ground-truth 목적 2(변경 전후 비교)에 기여한다: Cycle에 배정된 태스크를 기반으로 사이클 단위 달성도를 측정하는 기반을 제공한다.

## 입력

### `seogi cycle assign`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `cycle_id` | String (positional) | O | 사이클 ID |
| `task_id` | String (positional) | O | 태스크 ID |

### `seogi cycle unassign`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `cycle_id` | String (positional) | O | 사이클 ID |
| `task_id` | String (positional) | O | 태스크 ID |

### MCP 도구

- `cycle_assign`: `cycle_id`, `task_id` (required)
- `cycle_unassign`: `cycle_id`, `task_id` (required)

### 자동 포함 (암묵적 입력)

기존 `task create`, `task move` 명령이 내부적으로 active Cycle을 조회하여 자동 배정.

## 출력

### `seogi cycle assign` 성공

```
Assigned task SEO-1 to cycle abc123
```

### `seogi cycle unassign` 성공

```
Unassigned task SEO-1 from cycle abc123
```

### 자동 포함 시 출력

자동 포함은 사용자에게 별도 메시지를 출력하지 않는다. 조용히 `cycle_tasks`에 기록.

### CLI 실패 출력

모든 실패 시 exit code 1, stderr에 에러 메시지 출력.

### MCP 도구 응답

- 성공: `CallToolResult::success` — 성공 메시지.
- 실패: `CallToolResult::error` — `DomainError` 메시지.

## 시나리오

### 성공

1. **명시적 배정**: `cycle assign <cycle_id> <task_id>` → `cycle_tasks`에 `assigned=planned` 기록.
2. **명시적 해제**: `cycle unassign <cycle_id> <task_id>` → `cycle_tasks`에서 해당 행 삭제.
3. **자동 포함 — task create**: active Cycle이 있을 때 `task create` → 해당 Cycle에 `assigned=auto`로 자동 추가.
4. **자동 포함 — task move (started)**: `task move in_progress` 시 미배정 태스크이면 active Cycle에 `assigned=auto`로 자동 추가.
5. **자동 포함 — task move (completed)**: `task move done` 시 미배정 태스크이면 active Cycle에 `assigned=auto`로 자동 추가.
6. **active Cycle 없음**: active Cycle이 없으면 자동 포함 안 함 (에러 아님, 조용히 무시).
7. **MCP assign/unassign**: MCP 도구로 배정/해제 성공.

### 실패

1. **존재하지 않는 cycle_id**: assign/unassign 시 → `DomainError::Validation`.
2. **존재하지 않는 task_id**: assign/unassign 시 → `DomainError::Validation`.
3. **중복 배정**: 이미 배정된 task를 같은 cycle에 다시 assign → `DomainError::Validation` (workflow에서 `is_assigned_to_cycle`로 사전 검증).
4. **미배정 해제**: 배정되지 않은 task를 unassign → `DomainError::Validation` (adapter `delete`가 false 반환 → workflow에서 에러 변환).
5. **자동 포함 DB 에러**: 자동 포함 중 DB 에러 발생 시 task create/move는 성공하고 자동 포함만 스킵 (best-effort, 에러 무시).
6. **completed Cycle에 배정**: 어떤 상태의 Cycle에든 명시적 배정 가능 (상태 제한 없음). 자동 포함은 active Cycle에만 적용.

## 제약 조건

- **Assigned enum**: `planned` (명시적 배정), `auto` (자동 포함 — task create/move 시). glossary 정의와 일치.
- **active Cycle 판정**: `derive_status(start_date, end_date, today) == Active`인 Cycle. `cycle_repo::find_active_by_workspace`로 조회 (cycle_repo 책임).
- **워크스페이스 범위**: 자동 포함은 태스크가 속한 워크스페이스의 active Cycle에만 적용.
- **미배정 판정 (자동 포함)**: `cycle_task_repo::is_task_in_any_cycle(conn, task_id)`가 false인 경우만 자동 포함. 이미 어떤 cycle에든 배정되어 있으면 스킵.
- **중복 배정 판정 (명시적)**: `cycle_task_repo::is_assigned_to_cycle(conn, cycle_id, task_id)`로 특정 cycle-task 쌍의 존재 여부 확인.
- **자동 포함 실패 시**: DB 에러 등으로 자동 포함이 실패해도 task create/move 자체는 성공해야 한다 (best-effort).
- **cycle_tasks 테이블**: SEO-20에서 이미 생성됨. 추가 마이그레이션 불필요.

## 구현 구조

| 계층 | 모듈 | 변경 내용 |
|------|------|----------|
| Domain | `domain/cycle.rs` | `Assigned` enum 추가 (`Planned`, `Auto`). |
| Adapter | `adapter/cycle_repo.rs` | `find_active_by_workspace` 추가 (active Cycle 조회). |
| Adapter | `adapter/cycle_task_repo.rs` (신규) | `save`, `delete`, `is_assigned_to_cycle`, `is_task_in_any_cycle` 함수. |
| Adapter | `adapter/mod.rs` | `pub mod cycle_task_repo` 추가. |
| Workflow | `workflow/cycle.rs` | `assign`, `unassign` 함수 추가. |
| Workflow | `workflow/task.rs` | `create`/`move_task`에 자동 포함 로직 추가. |
| Entrypoint | `entrypoint/cycle.rs` | `assign`, `unassign` CLI 핸들러 추가. |
| Entrypoint | `entrypoint/mcp.rs` | `cycle_assign`, `cycle_unassign` MCP 도구 추가. |
| Entrypoint | `main.rs` | `CycleAction::Assign`, `CycleAction::Unassign` 추가. |

## 의존성

- Feature 32 (SEO-20): Cycle 스키마 + CRUD.
- Feature 34 (SEO-24): Cycle status 날짜 기반 파생.

---

## QA 목록

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q1 | `Assigned` enum 2개 variant 존재 (Planned, Auto) |
| Q2 | `Assigned::as_str` → 소문자 문자열 반환 |
| Q3 | `Assigned::from_str` 유효값 → 해당 variant |
| Q4 | `Assigned::from_str` 무효값 → `DomainError::Validation` |

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q5 | `cycle_task_repo::save` 후 DB에 행 존재 |
| Q6 | `cycle_task_repo::save` 중복 → `rusqlite::Error` (복합 PK 제약 위반) |
| Q7 | `cycle_task_repo::delete` 성공 → 행 삭제 |
| Q8 | `cycle_task_repo::delete` 미존재 → false 반환 |
| Q9 | `cycle_task_repo::is_assigned_to_cycle` 배정됨 → true |
| Q10 | `cycle_task_repo::is_assigned_to_cycle` 미배정 → false |
| Q11 | `cycle_task_repo::is_task_in_any_cycle` 배정됨 → true |
| Q12 | `cycle_task_repo::is_task_in_any_cycle` 미배정 → false |
| Q13 | `cycle_repo::find_active_by_workspace` active Cycle 존재 → `Some` |
| Q14 | `cycle_repo::find_active_by_workspace` active Cycle 없음 → `None` |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q15 | `cycle::assign` 성공: `cycle_tasks`에 `assigned=planned` 기록 |
| Q16 | `cycle::assign` 존재하지 않는 cycle_id → 에러 |
| Q17 | `cycle::assign` 존재하지 않는 task_id → 에러 |
| Q18 | `cycle::assign` 중복 배정 → 에러 |
| Q19 | `cycle::unassign` 성공: `cycle_tasks`에서 삭제 |
| Q20 | `cycle::unassign` 미배정 → 에러 |
| Q21 | `task::create` active Cycle 있을 때 → 자동 배정 (assigned=auto), `cycle_tasks`에 행 존재 |
| Q22 | `task::create` active Cycle 없을 때 → `cycle_tasks`에 해당 task_id 행 없음 |
| Q23 | `task::move_task` started 전환 시 미배정 → active Cycle에 자동 추가 (assigned=auto), `cycle_tasks`에 행 존재 |
| Q24 | `task::move_task` started 전환 시 이미 배정 → `cycle_tasks`에 추가 행 없음 |
| Q25 | `task::move_task` completed 전환 시 미배정 → active Cycle에 자동 추가 (assigned=auto) |
| Q26 | `task::move_task` active Cycle 없을 때 → `cycle_tasks`에 해당 task_id 행 없음 |

### E2E (CLI + MCP)

| # | 검증 항목 |
|---|----------|
| Q27 | `seogi cycle assign` 성공 메시지 출력 |
| Q28 | `seogi cycle assign` 존재하지 않는 cycle → 에러, exit 1 |
| Q29 | `seogi cycle unassign` 성공 메시지 출력 |
| Q30 | MCP `cycle_assign` 성공 → 성공 메시지 |
| Q31 | MCP `cycle_unassign` 성공 → 성공 메시지 |

---

## Test Pyramid

### Unit (Domain) — 4건

```
test_assigned_variant_count                  → Q1
test_assigned_as_str                         → Q2
test_assigned_from_str_valid                 → Q3
test_assigned_from_str_invalid               → Q4
```

### Integration (Adapter + Workflow) — 22건

```
test_cycle_task_save                         → Q5
test_cycle_task_save_duplicate               → Q6
test_cycle_task_delete                       → Q7
test_cycle_task_delete_not_found             → Q8
test_is_assigned_to_cycle_true               → Q9
test_is_assigned_to_cycle_false              → Q10
test_is_task_in_any_cycle_true               → Q11
test_is_task_in_any_cycle_false              → Q12
test_find_active_cycle_found                 → Q13
test_find_active_cycle_not_found             → Q14
test_assign_success                          → Q15
test_assign_cycle_not_found                  → Q16
test_assign_task_not_found                   → Q17
test_assign_duplicate                        → Q18
test_unassign_success                        → Q19
test_unassign_not_found                      → Q20
test_task_create_auto_assign                 → Q21
test_task_create_no_active_cycle             → Q22
test_task_move_auto_assign_started           → Q23
test_task_move_already_assigned              → Q24
test_task_move_auto_assign_completed         → Q25
test_task_move_no_active_cycle               → Q26
```

### E2E (CLI + MCP) — 5건

```
test_cycle_assign_cli                        → Q27
test_cycle_assign_cli_not_found              → Q28
test_cycle_unassign_cli                      → Q29
test_mcp_cycle_assign                        → Q30
test_mcp_cycle_unassign                      → Q31
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 31건 작성 완료
- [ ] Test Pyramid 31건 (Unit 4 + Integration 22 + E2E 5)
- [ ] 사용자 승인
