# Feature 34: Cycle status 필드 제거 → 날짜 기반 파생 (SEO-24)

## 목적

Cycle의 `status` 컬럼을 DB에서 제거하고, `start_date`/`end_date` + 현재 날짜로 파생하도록 변경한다. ground-truth 목적 2(변경 전후 비교)에 기여한다: 날짜 기반으로 active 구간이 자동 결정되어, 수동 전환 없이 baseline 비교 구간을 제공한다.

또한 "워크스페이스당 active Cycle 1개" 제약을 "같은 워크스페이스 내 날짜 구간 겹침 불가"로 대체하여, create/update 시 겹침을 검증한다.

## 입력

기존 `seogi cycle create/list/update` 및 MCP `cycle_create/cycle_list/cycle_update`의 인터페이스는 변경 없음. `status` 파라미터가 없었으므로 외부 인터페이스에 영향 없음.

### 시스템 입력 (파생 계산)

| 입력 | 타입 | 설명 |
|------|------|------|
| `start_date` | String (YYYY-MM-DD) | Cycle의 시작일 |
| `end_date` | String (YYYY-MM-DD) | Cycle의 종료일 |
| `today` | `NaiveDate` | 현재 날짜 |

## 출력

### 파생 규칙

| 조건 | CycleStatus |
|------|-------------|
| `today < start_date` | `planned` |
| `start_date <= today <= end_date` | `active` |
| `today > end_date` | `completed` |

### DB 변경 (부수효과)

- `cycles` 테이블에서 `status` 컬럼 제거 (마이그레이션 v8→v9).
- `schema.sql`에서 `status` 컬럼 삭제.

### CLI/MCP 출력

기존과 동일하게 `status` 필드를 출력하되, DB 값이 아닌 파생 값을 사용.

## 시나리오

### 성공

1. **status 파생 — planned**: `today < start_date`인 Cycle → `CycleStatus::Planned`.
2. **status 파생 — active**: `start_date <= today <= end_date`인 Cycle → `CycleStatus::Active`.
3. **status 파생 — completed**: `today > end_date`인 Cycle → `CycleStatus::Completed`.
4. **경계값 — start_date == today**: `CycleStatus::Active`.
5. **경계값 — end_date == today**: `CycleStatus::Active`.
6. **cycle list**: 조회 시 파생된 status가 포함되어 출력.
7. **cycle create 겹침 없음**: 같은 워크스페이스 내 기존 Cycle과 날짜 구간이 겹치지 않으면 생성 성공.
8. **cycle update 겹침 없음**: 날짜 변경 후에도 겹치지 않으면 수정 성공.
9. **마이그레이션**: v8→v9 적용 후 `cycles` 테이블에 `status` 컬럼 없음.

### 실패

1. **cycle create 겹침**: 같은 워크스페이스 내 기존 Cycle과 날짜 구간이 겹치면 → `DomainError::Validation`.
2. **cycle update 겹침**: 날짜 변경 후 다른 Cycle과 구간이 겹치면 → `DomainError::Validation`. (자기 자신은 제외)

## 제약 조건

- **날짜 구간 겹침 판정**: 두 구간 `[s1, e1]`과 `[s2, e2]`가 겹치는 조건은 `s1 <= e2 AND s2 <= e1`.
- **겹침 검증 시점**: `cycle::create`는 항상 검증. `cycle::update`는 `start_date` 또는 `end_date`가 변경될 때만 검증 (날짜 미변경 시 스킵).
- **`CycleStatus` enum 유지**: 파생 결과 타입으로 사용. `from_str`은 유지 (테스트/직렬화 용도).
- **`Cycle::status` 시그니처**: `pub fn status(&self, today: NaiveDate) -> CycleStatus`. 순수 함수.
- **마이그레이션**: SQLite는 `ALTER TABLE DROP COLUMN`을 지원하므로 직접 사용. 기존 `status` 컬럼 데이터는 날짜 기반 파생으로 대체되므로 손실을 허용한다.
- **기존 인터페이스에 status 파라미터 없음**: SEO-20에서 구현한 `cycle_create`/`cycle_update`의 입력에 status 파라미터가 없었으므로 (status는 생성 시 항상 `planned`로 고정), 외부 인터페이스 변경이 불필요하다.

## 구현 구조

| 계층 | 모듈 | 변경 내용 |
|------|------|----------|
| Domain | `domain/cycle.rs` | `Cycle` 구조체에서 `status` 필드 제거. `Cycle::status(&self, today) -> CycleStatus` 파생 메서드 추가. `Cycle::new`/`from_row`에서 status 제거. `dates_overlap` 순수 함수 추가. |
| Adapter | `adapter/cycle_repo.rs` | `CYCLE_COLUMNS`에서 `status` 제거. `save`에서 `status` 제거. `list_by_workspace_overlapping` 추가 (겹침 조회). |
| Adapter | `adapter/mapper.rs` | `cycle_from_row`에서 `status` 파싱 제거. |
| Adapter | `adapter/sql/migration_v8_to_v9.sql` | `ALTER TABLE cycles DROP COLUMN status`. |
| Adapter | `adapter/sql/schema.sql` | `cycles` 테이블에서 `status` 컬럼 제거. |
| Adapter | `adapter/db.rs` | 마이그레이션 v8→v9 등록. |
| Workflow | `workflow/cycle.rs` | `create`/`update`에 겹침 검증 추가. `list`에서 파생 status 포함. |
| Entrypoint | `entrypoint/cycle.rs` | list 출력 시 파생 status 사용 (현재 날짜 전달). |
| Entrypoint | `entrypoint/mcp.rs` | cycle_list/cycle_create 응답에 파생 status 포함. |

## 의존성

- Feature 32 (SEO-20): Cycle 스키마 + CRUD.

---

## QA 목록

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q1 | `Cycle::status(today < start_date)` → `CycleStatus::Planned` |
| Q2 | `Cycle::status(start_date <= today <= end_date)` → `CycleStatus::Active` |
| Q3 | `Cycle::status(today > end_date)` → `CycleStatus::Completed` |
| Q4 | `Cycle::status(today == start_date)` → `CycleStatus::Active` (경계) |
| Q5 | `Cycle::status(today == end_date)` → `CycleStatus::Active` (경계) |
| Q6 | `dates_overlap([s1,e1], [s2,e2])` 겹치는 경우 → true |
| Q7 | `dates_overlap([s1,e1], [s2,e2])` 겹치지 않는 경우 → false |
| Q8 | `dates_overlap` 인접 구간 (e1 == s2) → true (같은 날 겹침) |
| Q9 | `dates_overlap` 인접 구간 (e1 + 1 == s2) → false |
| Q10 | `Cycle::new` status 파라미터 없이 생성 성공 |
| Q11 | `Cycle::from_row` status 파라미터 없이 복원 성공 |

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q12 | `cycle_repo::save` 후 DB에 status 컬럼 없이 저장 확인 |
| Q13 | `cycle_repo::find_by_id` status 없이 Cycle 복원 |
| Q14 | `cycle_repo::list_by_workspace_overlapping` 겹치는 Cycle 반환 |
| Q15 | `cycle_repo::list_by_workspace_overlapping` 겹치지 않으면 빈 Vec |
| Q16 | `cycle_repo::list_by_workspace_overlapping` 자기 자신 제외 (exclude_id) |
| Q17 | 마이그레이션 v8→v9 적용 후 cycles 테이블에 status 컬럼 없음 |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q18 | `cycle::create` 겹침 없으면 성공 |
| Q19 | `cycle::create` 겹치면 에러 |
| Q20 | `cycle::update` 날짜 변경 후 겹침 없으면 성공 |
| Q21 | `cycle::update` 날짜 변경 후 겹치면 에러 |
| Q22 | `cycle::list` 파생 status 포함 |

### E2E (CLI + MCP)

| # | 검증 항목 |
|---|----------|
| Q23 | `seogi cycle create` 겹침 시 에러 출력, exit 1 |
| Q24 | `seogi cycle list --json` 파생 status 포함 |
| Q25 | MCP `cycle_create` 겹침 시 에러 응답 |
| Q26 | MCP `cycle_list` 파생 status 포함 |

---

## Test Pyramid

### Unit (Domain) — 11건

```
test_status_planned                          → Q1
test_status_active                           → Q2
test_status_completed                        → Q3
test_status_boundary_start                   → Q4
test_status_boundary_end                     → Q5
test_dates_overlap_true                      → Q6
test_dates_overlap_false                     → Q7
test_dates_overlap_adjacent_same_day         → Q8
test_dates_overlap_adjacent_next_day         → Q9
test_cycle_new_no_status                     → Q10
test_cycle_from_row_no_status                → Q11
```

### Integration (Adapter + Workflow) — 11건

```
test_save_without_status                     → Q12
test_find_by_id_without_status               → Q13
test_list_overlapping_found                  → Q14
test_list_overlapping_not_found              → Q15
test_list_overlapping_exclude_self           → Q16
test_migration_v8_to_v9                      → Q17
test_create_no_overlap                       → Q18
test_create_overlap_error                    → Q19
test_update_no_overlap                       → Q20
test_update_overlap_error                    → Q21
test_list_with_derived_status                → Q22
```

### E2E (CLI + MCP) — 4건

```
test_cycle_create_overlap_cli                → Q23
test_cycle_list_derived_status_cli           → Q24
test_mcp_cycle_create_overlap                → Q25
test_mcp_cycle_list_derived_status           → Q26
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 26건 작성 완료
- [ ] Test Pyramid 26건 (Unit 11 + Integration 11 + E2E 4)
- [ ] 사용자 승인
