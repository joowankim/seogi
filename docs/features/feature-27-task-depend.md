# Feature 27: 의존성 라벨링

## 목적

태스크 간 의존 관계를 시스템으로 명시하여, 작업 순서와 블로킹 상태를 파악할 수 있도록 한다.

## 입력

### `seogi task create` (확장)

기존 `task create`에 optional `--depends-on` 옵션 추가:

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `--depends-on` | String | X | 의존 대상 태스크 ID |

MCP `task_create`에도 optional `depends_on` 파라미터 추가:

| 파라미터 | 타입 | 필수 | 설명 |
|----------|------|------|------|
| `depends_on` | String | X | 의존 대상 태스크 ID |

생성과 동시에 의존 관계를 설정한다. 순환 검증, 존재 검증은 `depend`와 동일.

### `seogi task depend`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `task_id` | String (positional) | O | 의존하는 태스크 ID |
| `--on` | String | O | 의존 대상 태스크 ID |

### `seogi task undepend`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `task_id` | String (positional) | O | 의존 관계를 제거할 태스크 ID |
| `--on` | String | O | 제거할 의존 대상 태스크 ID |

### MCP: `task_depend`

| 파라미터 | 타입 | 필수 | 설명 |
|----------|------|------|------|
| `task_id` | String | O | 의존하는 태스크 ID |
| `depends_on` | String | O | 의존 대상 태스크 ID |

### MCP: `task_undepend`

| 파라미터 | 타입 | 필수 | 설명 |
|----------|------|------|------|
| `task_id` | String | O | 의존 관계를 제거할 태스크 ID |
| `depends_on` | String | O | 제거할 의존 대상 태스크 ID |

## 출력

### depend 성공

```
Added dependency: SEO-2 depends on SEO-1
```

### undepend 성공

```
Removed dependency: SEO-2 no longer depends on SEO-1
```

### task list (blocked 표시)

```
ID         TITLE                    STATUS           LABEL
SEO-1      첫 번째 태스크           in_progress      feature
SEO-2      두 번째 태스크           todo [blocked]   feature
```

`[blocked]`는 미완료 의존성이 있는 태스크에 표시.

### task get (의존성 목록)

기존 출력에 `Depends on:` 필드 추가:

```
ID:          SEO-2
Title:       두 번째 태스크
...
Depends on:  SEO-1
```

의존성이 없으면 해당 필드 생략.

### task get --json (의존성 목록)

```json
{
  "id": "SEO-2",
  ...
  "depends_on": ["SEO-1"]
}
```

### 실패

```
Error: Task not found: "SEO-99"
Error: Circular dependency detected: SEO-1 → SEO-2 → SEO-1
Error: Dependency already exists: SEO-2 depends on SEO-1
Error: Dependency not found: SEO-2 does not depend on SEO-1
Error: Cannot depend on self: "SEO-1"
```

## 시나리오

### 성공

1. **생성 시 의존 관계**: `seogi task create ... --depends-on SEO-1` → 태스크 생성 + 의존 관계 설정.
2. **의존 관계 추가**: `seogi task depend SEO-2 --on SEO-1` → 성공 메시지.
3. **의존 관계 제거**: `seogi task undepend SEO-2 --on SEO-1` → 성공 메시지.
4. **task list blocked 표시**: 미완료 의존성이 있는 태스크에 `[blocked]` 표시.
5. **task list 의존성 완료**: 의존 대상이 모두 done이면 `[blocked]` 미표시.
6. **task get 의존성 출력**: 의존 대상 목록이 출력됨.
7. **MCP depend**: `task_depend` 도구로 의존 관계 추가.
8. **MCP undepend**: `task_undepend` 도구로 의존 관계 제거.
9. **MCP create + depends_on**: `task_create`에 `depends_on` 전달 시 생성과 동시에 의존 관계 설정.

### 실패

1. **존재하지 않는 태스크**: 없는 task_id 또는 depends_on → 에러.
2. **자기 자신에 의존**: `depend SEO-1 --on SEO-1` → 에러.
3. **순환 의존성**: SEO-1→SEO-2→SEO-1 형성 시 → 에러.
4. **중복 의존**: 이미 존재하는 의존 관계 추가 → 에러.
5. **존재하지 않는 의존 관계 제거**: 없는 관계 제거 → 에러.

## 제약 조건

- **DB 스키마**: `task_dependencies(task_id TEXT, depends_on_task_id TEXT, PRIMARY KEY(task_id, depends_on_task_id))` + 양 컬럼 FK → tasks(id).
- **순환 의존성 방지**: 추가 전 DFS로 순환 검증. 도메인 순수 함수로 구현.
- **blocked 판정**: 의존 대상 중 하나라도 completed/canceled 카테고리가 아니면 blocked.
- **task_get 확장**: `TaskListRow`에 `depends_on: Vec<String>` 필드 추가는 하지 않음. `task_get` workflow에서 별도 조회 후 합성.
- **task_list 확장**: `list_all` 쿼리를 변경하지 않고, entrypoint에서 blocked 여부를 별도 조회 후 표시.

## 의존성

- Feature 26 (Task get): `task_get`에서 의존성 표시 필요.

---

## QA 목록

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q1 | `task_dependency_repo::save` 성공 → 행 추가 |
| Q2 | `task_dependency_repo::save` 중복 → 에러 |
| Q3 | `task_dependency_repo::delete` 존재하는 관계 → true |
| Q4 | `task_dependency_repo::delete` 없는 관계 → false |
| Q5 | `task_dependency_repo::list_dependencies` 태스크의 의존 대상 목록 반환 |
| Q6 | `task_dependency_repo::list_all_edges` 전체 간선 반환 (순환 검증용) |

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q7 | `detect_cycle` 순환 없음 → false |
| Q8 | `detect_cycle` 직접 순환 (A→B→A) → true |
| Q9 | `detect_cycle` 간접 순환 (A→B→C→A) → true |
| Q10 | 자기 자신 의존 검증 → 에러 |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q11 | `depend` 성공 |
| Q12 | `depend` 존재하지 않는 태스크 → 에러 |
| Q13 | `depend` 자기 자신 → 에러 |
| Q14 | `depend` 순환 → 에러 |
| Q15 | `depend` 중복 → 에러 |
| Q15a | `create` with `depends_on` → 태스크 생성 + 의존 관계 설정 |
| Q15b | `create` with 존재하지 않는 `depends_on` → 에러 (태스크 미생성) |
| Q16 | `undepend` 성공 |
| Q17 | `undepend` 없는 관계 → 에러 |
| Q18 | `is_blocked` 미완료 의존성 → true |
| Q19 | `is_blocked` 의존성 모두 완료 → false |
| Q20 | `is_blocked` 의존성 없음 → false |

### E2E (CLI)

| # | 검증 항목 |
|---|----------|
| Q21 | `seogi task depend SEO-2 --on SEO-1` 성공 메시지 |
| Q21a | `seogi task create ... --depends-on SEO-1` 생성 + 의존 관계 설정 |
| Q22 | `seogi task depend SEO-99 --on SEO-1` 에러 |
| Q23 | `seogi task undepend SEO-2 --on SEO-1` 성공 메시지 |
| Q24 | `seogi task list` blocked 태스크에 `[blocked]` 표시 |
| Q25 | `seogi task get SEO-2` 의존성 목록 포함 |
| Q26 | `seogi task get SEO-2 --json` depends_on 배열 포함 |

### E2E (MCP)

| # | 검증 항목 |
|---|----------|
| Q27 | `task_depend` 성공 응답 |
| Q28 | `task_depend` 순환 → 에러 응답 |
| Q29 | `task_undepend` 성공 응답 |
| Q30 | `task_get` 의존성 포함 응답 |
| Q30a | `task_create` with `depends_on` → 생성 + 의존 관계 설정 응답 |

---

## Test Pyramid

### Unit (Domain) — 4건

```
test_detect_cycle_no_cycle                     → Q7
test_detect_cycle_direct                       → Q8
test_detect_cycle_indirect                     → Q9
test_self_dependency                           → Q10
```

### Integration (Adapter + Workflow) — 18건

```
test_task_dependency_repo_save                 → Q1
test_task_dependency_repo_save_duplicate       → Q2
test_task_dependency_repo_delete_found         → Q3
test_task_dependency_repo_delete_not_found     → Q4
test_task_dependency_repo_list_dependencies    → Q5
test_task_dependency_repo_list_all_edges       → Q6
test_depend_success                            → Q11
test_depend_task_not_found                     → Q12
test_depend_self                               → Q13
test_depend_circular                           → Q14
test_depend_duplicate                          → Q15
test_create_with_depends_on                    → Q15a
test_create_with_invalid_depends_on            → Q15b
test_undepend_success                          → Q16
test_undepend_not_found                        → Q17
test_is_blocked_with_pending                   → Q18
test_is_blocked_all_completed                  → Q19
test_is_blocked_no_dependencies                → Q20
```

### E2E (CLI + MCP) — 12건

```
test_task_depend_success                       → Q21
test_task_create_with_depends_on               → Q21a
test_task_depend_not_found                     → Q22
test_task_undepend_success                     → Q23
test_task_list_blocked                         → Q24
test_task_get_with_dependencies                → Q25
test_task_get_json_with_dependencies           → Q26
test_mcp_task_depend_success                   → Q27
test_mcp_task_depend_circular                  → Q28
test_mcp_task_undepend_success                 → Q29
test_mcp_task_get_with_dependencies            → Q30
test_mcp_task_create_with_depends_on           → Q30a
```

---

## 완료 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 목록 34건 작성 완료
- [x] Test Pyramid 34건 (Unit 4 + Integration 18 + E2E 12)
- [ ] 사용자 승인

승인일:
