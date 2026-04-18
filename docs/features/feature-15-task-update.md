# Feature 15: Task 업데이트

## 목적

기존 태스크의 title, description, label을 수정하는 CLI 명령어를 제공한다. 수정 시 `updated_at`이 현재 시각으로 갱신된다.

## 입력

### `seogi task update`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `task_id` | String (positional) | O | 태스크 ID (e.g., SEO-1) |
| `--title` | String | X | 변경할 제목 |
| `--description` | String | X | 변경할 설명 |
| `--label` | String | X | 변경할 라벨 |

옵션 중 최소 1개는 필수.

## 출력

### 성공

```
Updated task SEO-1
```

### 실패

```
Error: Task not found: "SEO-99"
Error: At least one of --title, --description, or --label must be specified
Error: invalid label: "invalid". must be one of: feature, bug, refactor, chore, docs
```

## 시나리오

### 성공

1. **title만 수정**: `--title`만 지정하면 title만 변경되고 `updated_at` 갱신.
2. **description만 수정**: `--description`만 지정하면 description만 변경.
3. **label만 수정**: `--label`만 지정하면 label만 변경.
4. **복합 수정**: 여러 옵션을 동시에 지정하면 모두 변경.

### 실패

1. **존재하지 않는 태스크**: 없는 task_id → 에러.
2. **옵션 미지정**: 아무 옵션도 없음 → 에러.
3. **빈 title**: `--title ""`  → 에러.
4. **빈 description**: `--description ""` → 에러.
5. **무효 label**: `--label invalid` → 에러.

## 제약 조건

- **부분 업데이트**: 지정된 필드만 변경. 미지정 필드는 기존 값 유지.
- **updated_at 갱신**: 수정 시 현재 시각으로 갱신. `created_at`은 불변.
- **빈 문자열 거부**: title, description은 빈 문자열 불가.
- **Label 유효성**: Label enum 값만 허용.

## 의존성

- Feature 14 (Task create/list): 태스크 존재 확인, task_repo.

---

## QA 목록

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q1 | `task_repo::find_by_id` 존재하는 id → `Some(Task)` |
| Q2 | `task_repo::find_by_id` 없는 id → `None` |
| Q3 | `task_repo::update` title만 지정 → title만 변경, updated_at 갱신 |
| Q4 | `task_repo::update` 복합 지정 → 모든 필드 변경 |
| Q5 | `task_repo::update` 없는 id → false 반환 |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q6 | `update` 성공 시 OK 반환 |
| Q7 | `update` 존재하지 않는 태스크 → 에러 |
| Q8 | `update` 옵션 미지정 → 에러 |
| Q9 | `update` 빈 title → 에러 |
| Q10 | `update` 무효 label → 에러 |

### E2E (CLI)

| # | 검증 항목 |
|---|----------|
| Q11 | `seogi task update SEO-1 --title "new"` 성공 메시지, DB 반영 |
| Q12 | `seogi task update SEO-99 --title "new"` 에러 출력 |
| Q13 | `seogi task update SEO-1` (옵션 없음) 에러 출력 |
| Q14 | `seogi task update SEO-1 --title "new" --label bug` 복합 수정 성공 |

---

## Test Pyramid

### Integration (Adapter + Workflow) — 10건

```
test_task_repo_find_by_id_found             → Q1
test_task_repo_find_by_id_not_found         → Q2
test_task_repo_update_title_only            → Q3
test_task_repo_update_combined              → Q4
test_task_repo_update_not_found             → Q5
test_update_task_success                    → Q6
test_update_task_not_found                  → Q7
test_update_task_no_options                 → Q8
test_update_task_empty_title                → Q9
test_update_task_invalid_label              → Q10
```

### E2E (CLI) — 4건

```
test_task_update_title                      → Q11
test_task_update_not_found                  → Q12
test_task_update_no_options                 → Q13
test_task_update_combined                   → Q14
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 14건 작성 완료
- [ ] Test Pyramid 14건 (Integration 10 + E2E 4)
- [ ] 사용자 승인

승인일: 2026-04-18
