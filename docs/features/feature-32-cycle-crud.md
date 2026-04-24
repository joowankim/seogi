# Feature 32: Cycle 스키마 + CRUD (SEO-20)

## 목적

Cycle(기간별 목표 단위)의 스키마를 생성하고 기본 CRUD(create/list/update)를 구현한다. ground-truth 목적 2(변경 전후 비교)에 기여한다: Cycle 단위로 throughput, cycle_time 등 태스크 지표를 집계하여, 하니스 변경 전후의 기준선(baseline) 비교 구간을 제공한다.

## 입력

### `seogi cycle create`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `--workspace` | String | O | 워크스페이스 이름 |
| `--name` | String | O | 사이클 이름 (e.g., "Sprint 1") |
| `--start` | String | O | 시작일 (YYYY-MM-DD) |
| `--end` | String | O | 종료일 (YYYY-MM-DD) |

### `seogi cycle list`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `--workspace` | String | X | 워크스페이스 이름 필터 |
| `--json` | flag | X | JSON 형식 출력 |

### `seogi cycle update`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `cycle_id` | String (positional) | O | 사이클 ID |
| `--name` | String | X | 변경할 이름 |
| `--start` | String | X | 변경할 시작일 |
| `--end` | String | X | 변경할 종료일 |

### MCP 도구

- `cycle_create`: workspace, name, start_date, end_date
- `cycle_list`: workspace (optional)
- `cycle_update`: cycle_id, name (optional), start_date (optional), end_date (optional)

## 출력

### `seogi cycle create` 성공

```
Created cycle abc123 "Sprint 1"
```

### `seogi cycle list` 테이블 출력

```
ID         NAME          STATUS     START        END          WORKSPACE
abc123     Sprint 1      planned    2026-05-01   2026-05-14   Seogi
def456     Sprint 2      planned    2026-05-15   2026-05-28   Seogi
```

### `seogi cycle list --json`

```json
[
  {
    "id": "abc123...",
    "workspace_name": "Seogi",
    "name": "Sprint 1",
    "status": "planned",
    "start_date": "2026-05-01",
    "end_date": "2026-05-14",
    "created_at": "2026-04-24T12:00:00+00:00",
    "updated_at": "2026-04-24T12:00:00+00:00"
  }
]
```

### `seogi cycle update` 성공

```
Updated cycle abc123
```

### MCP 도구 성공 응답

기존 MCP 도구 패턴과 동일:
- 성공: `CallToolResult::success` — JSON 형식으로 cycle 정보 반환.
- 실패: `CallToolResult::error` — `DomainError` 메시지 문자열.

### CLI 실패 출력

모든 실패 시 exit code 1, stderr에 에러 메시지 출력. 기존 CLI 패턴(`anyhow::anyhow!("{e}")`)과 동일.

## 시나리오

### 성공

1. **사이클 생성**: 유효한 워크스페이스, 이름, 시작일, 종료일로 사이클을 생성하면 UUID hex 32글자 ID가 할당되고, 초기 상태는 `planned`로 설정된다.
2. **사이클 목록 조회**: 전체 사이클을 테이블 형식으로 출력한다. `created_at DESC` 정렬.
3. **워크스페이스 필터**: `--workspace`로 특정 워크스페이스의 사이클만 조회한다.
4. **JSON 출력**: `--json` 플래그로 JSON 형식 출력한다.
5. **사이클 수정**: 이름, 시작일, 종료일을 선택적으로 수정한다. `updated_at`이 갱신된다.
6. **MCP create**: `cycle_create` 도구로 사이클을 생성하고 JSON 응답을 반환한다.
7. **MCP list**: `cycle_list` 도구로 사이클 목록을 JSON으로 반환한다.
8. **MCP update**: `cycle_update` 도구로 사이클을 수정한다.

### 실패

1. **존재하지 않는 워크스페이스**: `--workspace`에 없는 워크스페이스 이름 → `DomainError::Validation`.
2. **빈 이름**: `--name`이 빈 문자열 → `DomainError::Validation`.
3. **잘못된 날짜 형식**: `--start` 또는 `--end`가 YYYY-MM-DD가 아닌 경우 → `DomainError::Validation`.
4. **시작일 > 종료일**: 시작일이 종료일보다 늦은 경우 → `DomainError::Validation`.
5. **존재하지 않는 사이클 ID**: update 시 없는 ID → `DomainError::Validation`.
6. **수정 필드 없음**: update 시 name, start, end 모두 None → `DomainError::Validation`.
7. **update 잘못된 날짜 형식**: update 시 `--start` 또는 `--end`가 YYYY-MM-DD가 아닌 경우 → `DomainError::Validation`.
8. **update 시작일 > 종료일**: update 결과 start_date > end_date가 되는 경우 → `DomainError::Validation`. (한쪽만 변경 시 기존 값과 비교)
9. **update 빈 이름**: update 시 `--name ""`(빈 문자열) → `DomainError::Validation`.
10. **list 존재하지 않는 워크스페이스 필터**: `--workspace`에 없는 워크스페이스 이름 → 빈 목록 반환 (에러 아님). 워크스페이스 존재 여부를 검증하지 않고 DB 쿼리 결과를 그대로 반환한다.

## 제약 조건

- **Cycle ID**: UUID hex 32글자. `uuid::Uuid::new_v4().simple().to_string()`.
- **CycleStatus**: 3개 고정 값 (`planned`, `active`, `completed`). 코드 enum으로 관리. SEO-20에서는 생성 시 항상 `planned`. 상태 전환은 SEO-21에서 구현.
- **날짜 형식**: `start_date`, `end_date`는 YYYY-MM-DD 문자열로 저장. DB에 TEXT로 저장.
- **타임스탬프**: `created_at`, `updated_at`은 RFC3339 형식 (`DateTime<Utc>`).
- **cycle_tasks 테이블**: SEO-20에서 마이그레이션으로 테이블만 생성. CRUD는 SEO-22에서 구현. 스키마:
  ```sql
  CREATE TABLE cycle_tasks (
      cycle_id    TEXT NOT NULL REFERENCES cycles(id),
      task_id     TEXT NOT NULL REFERENCES tasks(id),
      assigned    TEXT NOT NULL,      -- planned | auto
      PRIMARY KEY (cycle_id, task_id)
  );
  ```
- **스키마 버전**: v7 → v8 마이그레이션으로 cycles, cycle_tasks 테이블 추가.

## 구현 구조

| 계층 | 모듈 | 책임 |
|------|------|------|
| Domain | `domain/cycle.rs` | `Cycle`, `CycleStatus` 엔티티/enum. 생성 검증(이름, 날짜). |
| Adapter | `adapter/cycle_repo.rs` | `save`, `list_all`, `list_by_workspace`, `find_by_id`, `update`. |
| Adapter | `adapter/mapper.rs` | `cycle_from_row` 매퍼 추가. |
| Adapter | `adapter/sql/migration_v7_to_v8.sql` | cycles, cycle_tasks 테이블 DDL. |
| Workflow | `workflow/cycle.rs` | `create`, `list`, `update`. 워크스페이스 검증, 날짜 파싱. |
| Entrypoint | `entrypoint/cycle.rs` | CLI 핸들러 (create, list, update). |
| Entrypoint | `entrypoint/mcp.rs` | MCP 도구 추가 (`cycle_create`, `cycle_list`, `cycle_update`). |
| Entrypoint | `main.rs` | `Commands::Cycle`, `CycleAction` enum 추가. |

## 의존성

- Workspace CRUD (Feature 12): 워크스페이스 존재 확인.

---

## QA 목록

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q1 | `CycleStatus` enum 3개 variant 존재 (planned, active, completed) |
| Q2 | `CycleStatus::as_str` → 소문자 문자열 반환 |
| Q3 | `CycleStatus::from_str` 유효값 → 해당 variant |
| Q4 | `CycleStatus::from_str` 무효값 → `DomainError::Validation` |
| Q5 | `Cycle::new` 유효 입력 → id 32글자 hex |
| Q6 | `Cycle::new` 유효 입력 → status가 `planned` |
| Q7 | `Cycle::new` 유효 입력 → 필드값 보존 (workspace_id, name, start_date, end_date, timestamps) |
| Q8 | `Cycle::new` 빈 name → `DomainError::Validation` |
| Q9 | `Cycle::new` start_date > end_date → `DomainError::Validation` |
| Q10 | `Cycle::from_row` 필드값 보존 |

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q11 | `cycle_repo::save` 후 DB에서 조회 시 필드 일치 |
| Q12 | `cycle_repo::list_by_workspace` 해당 워크스페이스만 반환, `created_at DESC` 정렬 |
| Q13 | `cycle_repo::list_all` 전체 반환 |
| Q14 | `cycle_repo::find_by_id` 존재하는 ID → `Some(Cycle)` |
| Q15 | `cycle_repo::find_by_id` 없는 ID → `None` |
| Q16 | `cycle_repo::update` 이름 변경 시 DB 반영 |
| Q17 | `cycle_repo::update` 시작일/종료일 변경 시 DB 반영 |
| Q18 | 마이그레이션 v7→v8 적용 후 cycles, cycle_tasks 테이블 존재 |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q19 | `cycle::create` 성공 시 Cycle 반환, DB에 1건 저장 |
| Q20 | `cycle::create` 존재하지 않는 워크스페이스 → 에러 |
| Q21 | `cycle::create` 빈 이름 → 에러 |
| Q22 | `cycle::create` 잘못된 날짜 형식 → 에러 |
| Q23 | `cycle::create` start > end → 에러 |
| Q24 | `cycle::list` 워크스페이스 필터 적용 |
| Q25 | `cycle::list` 필터 없이 전체 반환 |
| Q26 | `cycle::update` 이름만 변경 → 성공, updated_at 갱신 |
| Q27 | `cycle::update` 시작일/종료일 변경 → 성공 |
| Q28 | `cycle::update` 없는 cycle_id → 에러 |
| Q29 | `cycle::update` 필드 전부 None → 에러 |
| Q30 | `cycle::update` 잘못된 날짜 형식 → 에러 |
| Q31 | `cycle::update` 결과 start > end → 에러 |
| Q32 | `cycle::update` 빈 이름 → 에러 |
| Q33 | `cycle::list` 존재하지 않는 워크스페이스 필터 → 빈 목록 반환 |

### E2E (CLI + MCP)

| # | 검증 항목 |
|---|----------|
| Q34 | `seogi cycle create` 성공 메시지에 cycle ID 포함 |
| Q35 | `seogi cycle create` 존재하지 않는 워크스페이스 → stderr 에러 출력, exit 1 |
| Q36 | `seogi cycle list` 테이블 형식 출력 |
| Q37 | `seogi cycle list --json` JSON 형식 출력 |
| Q38 | `seogi cycle list --workspace "..."` 필터링 |
| Q39 | `seogi cycle update` 성공 메시지 출력 |
| Q40 | MCP `cycle_create` 성공 → JSON 응답에 id, name 포함 |
| Q41 | MCP `cycle_list` → JSON 배열 응답 |
| Q42 | MCP `cycle_update` 성공 → 성공 메시지 |

---

## Test Pyramid

### Unit (Domain) — 10건

```
test_cycle_status_variant_count              → Q1
test_cycle_status_as_str                     → Q2
test_cycle_status_from_str_valid             → Q3
test_cycle_status_from_str_invalid           → Q4
test_cycle_new_id_format                     → Q5
test_cycle_new_initial_status                → Q6
test_cycle_new_fields                        → Q7
test_cycle_new_empty_name                    → Q8
test_cycle_new_start_after_end               → Q9
test_cycle_from_row_fields                   → Q10
```

### Integration (Adapter + Workflow) — 23건

```
test_cycle_repo_save_and_find                → Q11
test_cycle_repo_list_by_workspace            → Q12
test_cycle_repo_list_all                     → Q13
test_cycle_repo_find_by_id_found             → Q14
test_cycle_repo_find_by_id_not_found         → Q15
test_cycle_repo_update_name                  → Q16
test_cycle_repo_update_dates                 → Q17
test_migration_v7_to_v8                      → Q18
test_workflow_create_success                 → Q19
test_workflow_create_unknown_workspace       → Q20
test_workflow_create_empty_name              → Q21
test_workflow_create_invalid_date            → Q22
test_workflow_create_start_after_end         → Q23
test_workflow_list_with_filter               → Q24
test_workflow_list_all                       → Q25
test_workflow_update_name                    → Q26
test_workflow_update_dates                   → Q27
test_workflow_update_not_found               → Q28
test_workflow_update_no_fields               → Q29
test_workflow_update_invalid_date            → Q30
test_workflow_update_start_after_end         → Q31
test_workflow_update_empty_name              → Q32
test_workflow_list_unknown_workspace         → Q33
```

### E2E (CLI + MCP) — 9건

```
test_cycle_create_cli                        → Q34
test_cycle_create_cli_unknown_workspace      → Q35
test_cycle_list_cli_table                    → Q36
test_cycle_list_cli_json                     → Q37
test_cycle_list_cli_workspace_filter         → Q38
test_cycle_update_cli                        → Q39
test_mcp_cycle_create                        → Q40
test_mcp_cycle_list                          → Q41
test_mcp_cycle_update                        → Q42
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 42건 작성 완료
- [ ] Test Pyramid 42건 (Unit 10 + Integration 23 + E2E 9)
- [ ] 사용자 승인
