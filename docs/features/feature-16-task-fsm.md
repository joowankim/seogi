# Feature 16: FSM + Task 상태 전환

## 목적

태스크의 상태를 변경하는 CLI 명령어를 제공한다. 카테고리 간 전환 규칙(FSM)을 검증하고, 전환 이력을 `task_events`에 기록한다.

## 입력

### `seogi task move`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `task_id` | String (positional) | O | 태스크 ID (e.g., SEO-1) |
| `status` | String (positional) | O | 이동할 상태 이름 (e.g., in_progress, done) |

## 출력

### 성공

```
Moved task SEO-1: backlog → in_progress
```

### 실패

```
Error: Task not found: "SEO-99"
Error: Status not found: "invalid_status"
Error: Cannot transition from backlog (Backlog) to done (Completed). Allowed: unstarted, canceled
```

## 시나리오

### 성공

1. **카테고리 간 허용 전환**: Backlog→Unstarted, Started→Completed 등 허용된 전환 성공.
2. **같은 카테고리 내 전환**: in_progress → in_review (둘 다 Started) 자유 전환.
3. **task_events 기록**: 전환 시 from_status, to_status, session_id, timestamp 기록.

### 실패

1. **존재하지 않는 태스크**: 없는 task_id → 에러.
2. **존재하지 않는 상태**: 없는 status 이름 → 에러.
3. **허용되지 않은 전환**: Backlog→Completed 등 → 에러 (허용 가능한 전환 목록 표시).
4. **같은 상태로 전환**: 현재 상태와 동일 → 에러.

## 제약 조건

- **FSM 전환 규칙**: 카테고리 간 허용된 전환만 가능. 규칙은 순수 함수로 도메인 계층에 구현.

| From \ To | Backlog | Unstarted | Started | Completed | Canceled |
|-----------|---------|-----------|---------|-----------|----------|
| Backlog   | -       | O         | -       | -         | O        |
| Unstarted | O       | -         | O       | -         | O        |
| Started   | -       | O         | -       | O         | O        |
| Completed | -       | -         | O       | -         | -        |
| Canceled  | O       | -         | -       | -         | -        |

- **같은 카테고리 내 자유 전환**: 동일 카테고리의 커스텀 상태 간 전환은 항상 허용.
- **같은 상태 거부**: 현재 상태와 동일한 상태로의 전환은 거부.
- **task_events 기록**: from_status = 이전 상태 이름, to_status = 새 상태 이름, session_id = `CLI_SESSION_ID`.
- **status_id 갱신**: tasks 테이블의 `status_id`와 `updated_at`을 업데이트.

## 의존성

- Feature 14 (Task create/list): 태스크 조회, task_event_repo.

---

## QA 목록

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q1 | `can_transition` Backlog→Unstarted → true |
| Q2 | `can_transition` Backlog→Completed → false |
| Q3 | `can_transition` Completed→Started → true (rework) |
| Q4 | `can_transition` Canceled→Backlog → true (복구) |
| Q5 | `can_transition` 같은 카테고리 → true |
| Q6 | `allowed_transitions` Backlog → [Unstarted, Canceled] |
| Q7 | `allowed_transitions` Started → [Unstarted, Completed, Canceled] |

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q8 | `task_repo::find_by_id` 존재 → `Some(TaskRow)` |
| Q9 | `task_repo::find_by_id` 미존재 → `None` |
| Q10 | `task_repo::update_status` 성공 시 status_id, updated_at 변경 |
| Q11 | `status_repo::find_by_name` 존재 → `Some(Status)` |
| Q12 | `status_repo::find_by_name` 미존재 → `None` |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q13 | `move_task` 허용 전환 → 성공, task_events 1건 기록 |
| Q14 | `move_task` 태스크 미존재 → 에러 |
| Q15 | `move_task` 상태 미존재 → 에러 |
| Q16 | `move_task` 허용되지 않은 전환 → 에러 (허용 목록 포함) |
| Q17 | `move_task` 같은 상태 → 에러 |
| Q18 | `move_task` 같은 카테고리 내 전환 → 성공 |

### E2E (CLI)

| # | 검증 항목 |
|---|----------|
| Q19 | `seogi task move SEO-1 todo` 성공 메시지, DB 반영 |
| Q20 | `seogi task move SEO-99 todo` 에러 출력 |
| Q21 | `seogi task move SEO-1 done` (Backlog→Completed) 에러, 허용 목록 표시 |
| Q22 | `seogi task move SEO-1 backlog` (같은 상태) 에러 출력 |

---

## Test Pyramid

### Unit (Domain) — 7건

```
test_can_transition_backlog_to_unstarted    → Q1
test_can_transition_backlog_to_completed    → Q2
test_can_transition_completed_to_started    → Q3
test_can_transition_canceled_to_backlog     → Q4
test_can_transition_same_category           → Q5
test_allowed_transitions_backlog            → Q6
test_allowed_transitions_started            → Q7
```

### Integration (Adapter + Workflow) — 11건

```
test_task_repo_find_by_id_found             → Q8
test_task_repo_find_by_id_not_found         → Q9
test_task_repo_update_status                → Q10
test_status_repo_find_by_name_found         → Q11
test_status_repo_find_by_name_not_found     → Q12
test_move_task_success                      → Q13
test_move_task_not_found                    → Q14
test_move_task_status_not_found             → Q15
test_move_task_invalid_transition           → Q16
test_move_task_same_status                  → Q17
test_move_task_same_category                → Q18
```

### E2E (CLI) — 4건

```
test_task_move_success                      → Q19
test_task_move_task_not_found               → Q20
test_task_move_invalid_transition           → Q21
test_task_move_same_status                  → Q22
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 22건 작성 완료
- [ ] Test Pyramid 22건 (Unit 7 + Integration 11 + E2E 4)
- [ ] 사용자 승인

승인일: 2026-04-18
