# Feature 29: project→workspace DB 마이그레이션 + 훅 입력 매핑

## 목적

도메인 용어 "project"를 "workspace"로 교체하는 6단계의 첫 번째 작업.
DB 스키마와 adapter SQL 쿼리에서 project를 workspace로 리네이밍하고,
훅 입력의 `project`/`project_path` 필드를 내부적으로 `workspace`/`workspace_path`로 매핑한다.

이 Feature는 리팩토링이며 ground-truth의 측정 지표에 직접 기여하지 않는다.
Phase 6(project→workspace 리네이밍) 완료는 향후 workspace 단위 지표 집계(목적 1)와
workspace별 baseline 비교(목적 2)를 가능하게 하는 전제 조건이다.

## 입력

- 시스템 입력: 기존 SQLite DB (schema version 6, `projects`/`project_id`/`project`/`project_path` 컬럼 사용)
- 훅 stdin: Claude Code가 보내는 JSON (`cwd` 필드 → 내부적으로 `workspace_path`로 매핑)

## 출력

- DB 스키마 변경:
  - `projects` 테이블 → `workspaces`
  - `tasks.project_id` → `workspace_id`
  - `tool_uses.project` → `workspace`, `project_path` → `workspace_path`
  - `tool_failures.project` → `workspace`, `project_path` → `workspace_path`
  - `system_events.project` → `workspace`, `project_path` → `workspace_path`
- schema version: 6 → 7
- adapter SQL 쿼리: 새 테이블/컬럼명 사용
- mapper: `row.get("project")` → `row.get("workspace")` 등
- 도메인 타입의 필드명은 이 Feature에서 변경하지 않음 (SEO-16 범위)

## 성공 시나리오

1. **신규 DB**: version 0에서 시작 → schema.sql이 `workspaces` 테이블로 생성
2. **기존 DB (v6)**: `migration_v6_to_v7.sql` 실행 → ALTER TABLE RENAME으로 테이블/컬럼 변경, 기존 데이터 보존
3. **훅 데이터 저장**: Claude Code 훅이 `cwd` 필드를 보내면 → `workspace`/`workspace_path` 컬럼에 저장
4. **기존 기능 정상 동작**: task list, task get, report 등 모든 조회가 새 컬럼명으로 동작

## 실패 시나리오

1. **마이그레이션 실패**: ALTER TABLE RENAME 실패 시 → `AdapterError::Database` 반환, DB 변경 없음 (SQLite 트랜잭션 롤백)
2. **이전 버전 바이너리**: v7 스키마에서 v6 바이너리 실행 → 쿼리 실패 (컬럼명 불일치). 이는 정상적 제약이며 별도 대응 불요.
3. **이미 v7 이상인 DB**: `setup_connection`의 `if version < 7` 조건에 의해 마이그레이션이 스킵되어 영향 없음.

## 제약 조건

- **무손실 마이그레이션**: 기존 데이터 100% 보존 (ALTER TABLE RENAME 사용, DROP/재생성 금지)
- **Claude Code 훅 스펙 불변**: 훅 stdin의 `cwd` 필드는 외부 스펙이므로 변경 불가. serde 구조체의 필드명은 유지.
- **도메인 타입 필드명 유지**: `ToolUse`, `ToolFailure`, `SystemEvent`의 `project`/`project_path` 필드명은 이 Feature에서 변경하지 않음 (SEO-16 범위). 이 Feature에서는 DB 컬럼명만 변경하고, mapper에서 새 컬럼명 → 기존 도메인 필드로 매핑.

## 변경 범위 구분

| 항목 | 이 Feature에서 변경 | SEO-16에서 변경 |
|------|:---:|:---:|
| DB 테이블명 (`projects` → `workspaces`) | O | - |
| DB 컬럼명 (`project`/`project_path` → `workspace`/`workspace_path`) | O | - |
| adapter SQL 쿼리 (테이블/컬럼명) | O | - |
| mapper (`row.get` 컬럼명) | O | - |
| 도메인 타입 필드명 (`ToolUse.project` 등) | - | O |
| 도메인 함수명 (`extract_project_from_cwd` 등) | - | O |
| adapter 파일명 (`project_repo.rs` 등) | - | O |
| glossary.md 용어 등록 | - | SEO-18 |

## 작업 상세

### 1. migration_v6_to_v7.sql

```sql
-- 1) projects → workspaces
ALTER TABLE projects RENAME TO workspaces;

-- 2) tasks.project_id → workspace_id
ALTER TABLE tasks RENAME COLUMN project_id TO workspace_id;

-- 3) tool_uses
ALTER TABLE tool_uses RENAME COLUMN project TO workspace;
ALTER TABLE tool_uses RENAME COLUMN project_path TO workspace_path;

-- 4) tool_failures
ALTER TABLE tool_failures RENAME COLUMN project TO workspace;
ALTER TABLE tool_failures RENAME COLUMN project_path TO workspace_path;

-- 5) system_events
ALTER TABLE system_events RENAME COLUMN project TO workspace;
ALTER TABLE system_events RENAME COLUMN project_path TO workspace_path;
```

### 2. schema.sql 변경

모든 `projects` → `workspaces`, `project_id` → `workspace_id`, `project` → `workspace`, `project_path` → `workspace_path`.

### 3. adapter SQL 쿼리 변경

- `log_repo.rs`: INSERT/SELECT 문의 컬럼명 변경
- `task_repo.rs`: `JOIN projects` → `JOIN workspaces`, `project_id` → `workspace_id`
- `project_repo.rs`: `projects` → `workspaces` 테이블명 변경
- `mapper.rs`: `row.get("project")` → `row.get("workspace")`, `row.get("project_path")` → `row.get("workspace_path")`

### 4. db.rs 마이그레이션 로직

- `SCHEMA_VERSION`: 6 → 7
- `MIGRATION_V6_TO_V7` 상수 추가
- `setup_connection`에 `if version < 7` 분기 추가

### 5. 테스트 변경

- `db.rs` 테스트: 테이블명 `projects` → `workspaces`, 컬럼명 변경 반영
- `EXPECTED_TABLES`: `"projects"` → `"workspaces"`
- `insert_test_tool_use`: 컬럼명 변경
- 마이그레이션 v6→v7 테스트 추가

## 의존하는 기능

없음 (6단계 첫 번째 Feature)

---

## QA 목록

### 마이그레이션

1. v6 DB에 `migration_v6_to_v7.sql` 적용 후 `workspaces` 테이블이 존재하고 `projects` 테이블이 존재하지 않음
2. v6 DB의 `projects` 테이블 데이터가 마이그레이션 후 `workspaces` 테이블에 동일하게 존재
3. v6 DB의 `tasks.project_id` 데이터가 마이그레이션 후 `tasks.workspace_id`로 보존
4. v6 DB의 `tool_uses.project`/`project_path` 데이터가 마이그레이션 후 `workspace`/`workspace_path`로 보존
5. v6 DB의 `tool_failures.project`/`project_path` 데이터가 마이그레이션 후 `workspace`/`workspace_path`로 보존
6. v6 DB의 `system_events.project`/`project_path` 데이터가 마이그레이션 후 `workspace`/`workspace_path`로 보존
7. 신규 DB(version 0) 초기화 시 `workspaces` 테이블로 생성됨

### adapter SQL

8. `tool_uses` INSERT가 `workspace`/`workspace_path` 컬럼에 데이터를 저장함
9. `tool_failures` INSERT가 `workspace`/`workspace_path` 컬럼에 데이터를 저장함
10. `system_events` INSERT가 `workspace`/`workspace_path` 컬럼에 데이터를 저장함
11. `task_repo`의 TASK_DETAIL_SELECT가 `workspaces` 테이블을 JOIN하여 정상 조회됨
12. `project_repo`의 CRUD가 `workspaces` 테이블에서 정상 동작함

### 훅 매핑

13. 훅 stdin의 `cwd` 필드 값이 `workspace_path` 컬럼에 저장됨
14. `extract_project_from_cwd` 함수의 반환값이 adapter에서 `workspace` 컬럼명으로 INSERT됨 (함수명 자체는 SEO-16에서 변경)

### 마이그레이션 안전성

15. v6 DB에 대해 `setup_connection` 호출 시 v7로 정상 승격되고, `pragma user_version`이 7임

---

## Test Pyramid

| # | QA 항목 | 레벨 | 이유 |
|---|---------|------|------|
| 1 | workspaces 테이블 존재 확인 | 통합 | DB 마이그레이션 + 스키마 검증 |
| 2 | projects 데이터 보존 | 통합 | v6 시뮬레이션 → v7 마이그레이션 |
| 3 | tasks.workspace_id 보존 | 통합 | v6 시뮬레이션 → v7 마이그레이션 |
| 4 | tool_uses 컬럼 보존 | 통합 | v6 시뮬레이션 → v7 마이그레이션 |
| 5 | tool_failures 컬럼 보존 | 통합 | v6 시뮬레이션 → v7 마이그레이션 |
| 6 | system_events 컬럼 보존 | 통합 | v6 시뮬레이션 → v7 마이그레이션 |
| 7 | 신규 DB 테이블명 | 통합 | 기존 test_schema_creates_all_tables 수정 |
| 8-10 | INSERT 컬럼명 | 통합 | 기존 log_repo 테스트 수정 |
| 11 | task 조회 JOIN | 통합 | 기존 task_repo 테스트 수정 |
| 12 | project_repo CRUD | 통합 | 기존 project_repo 테스트 수정 |
| 13-14 | 훅 매핑 | 통합 | 기존 workflow 테스트 수정 (log_tool, log_failure, log_system) |
| 15 | v6→v7 승격 + user_version | 통합 | 마이그레이션 로직 검증 |

단위 테스트 대상 없음 (순수 함수 변경 없이 SQL/매핑만 변경).
마이그레이션 검증은 모두 통합 테스트 (인메모리 SQLite).
