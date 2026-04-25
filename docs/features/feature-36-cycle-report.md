# Feature 36: Cycle Report

## 목적

사이클 단위 지표를 집계하여 리포트를 출력한다.
ground-truth.md **목적 2**(하니스 변경 전후 비교)에 직접 기여한다. Cycle이 비교 기준 구간이 되며, cycle report로 구간별 지표를 비교할 수 있다.

## 입력

### 사용자 입력 (CLI)

- `seogi cycle report <cycle_id>` (필수)
- cycle_id: cycles 테이블의 id (UUID hex 32글자)

### 사용자 입력 (MCP)

- `cycle_report` 도구, 파라미터: `cycle_id: String` (필수)

### 시스템 입력

- DB: cycles, cycle_tasks, tasks, task_events, statuses, tool_uses, tool_failures 테이블
- 파일 시스템: transcript JSONL (토큰 집계)
- git: diff stat (task_size)

## 출력

### 반환값

사이클 리포트 문자열. 3개 구분(계획 완료, 계획 미완료, 비계획 완료)으로 분류된 태스크별 지표 + 구분별 소계 + 사이클 전체 요약.

### 구분 분류 규칙

| 구분 | 조건 | 의미 |
|---|---|---|
| 계획 완료 (planned_done) | assigned=planned AND 현재 status category=Completed | 계획대로 완료 |
| 계획 미완료 (planned_incomplete) | assigned=planned AND 현재 status category!=Completed | 계획했지만 못 끝냄 |
| 비계획 완료 (unplanned_done) | assigned=auto AND 현재 status category=Completed | 계획에 없었지만 완료 |
| (제외) | assigned=auto AND 현재 status category!=Completed | 자동 배정되었으나 미완료. rework 등으로 발생 가능하나 사이클 성과 측정 대상에서 제외 |

### 출력 형식

```
=== Cycle Report: "Sprint 1" (2026-05-01 ~ 2026-05-14, active) ===

--- Planned Done (3/4 tasks) ---
ID       TITLE               CYCLE     LEAD      TOKENS   SIZE     REWORK
SEO-1    MCP bootstrap       2h 30m    1d 4h     45,230   +342     no
SEO-2    MCP tools           1h 20m    2d        38,100   +210     no
SEO-3    MCP registration    40m       1d        12,500   +85      yes

--- Planned Incomplete (1/4 tasks) ---
ID       TITLE               STATUS       AGE
SEO-4    MCP docs            in_progress  3d 2h

--- Unplanned Done (1 task) ---
ID       TITLE               CYCLE     LEAD      TOKENS   SIZE     REWORK
SEO-5    hotfix login         15m       2h        5,200    +12      no

--- Summary ---
completion_rate: 75% (3/4 planned)
throughput: 4 tasks (3 planned + 1 unplanned)
flow_efficiency(avg): 0.52
first_time_done: 75%
```

### 부수효과

없음 (읽기 전용).

## 성공 시나리오

1. 사용자가 `seogi cycle report <cycle_id>`를 실행한다.
2. cycle_id로 cycle을 조회한다.
3. cycle_tasks에서 해당 cycle에 배정된 태스크 목록과 assigned 값을 조회한다.
4. 각 태스크의 현재 status category를 확인하여 3개 구분으로 분류한다.
5. 완료된 태스크(planned_done, unplanned_done)에 대해 기존 TaskReport 지표를 계산한다 (cycle_time, lead_time, flow_efficiency, tokens, task_size, has_rework).
6. 미완료 태스크(planned_incomplete)에 대해 issue_age를 계산한다.
7. 요약 지표를 계산한다:
   - completion_rate: planned_done 수 / planned 전체 수
   - throughput: 완료된 태스크 총 수 (planned_done + unplanned_done)
   - flow_efficiency(avg): 완료 태스크의 flow_efficiency 평균
   - first_time_done: 완료 태스크 중 rework 없는 비율
8. 포맷된 리포트 문자열을 출력한다.

## 실패 시나리오

| 조건 | 처리 |
|---|---|
| cycle_id에 해당하는 cycle이 없음 | `Cycle not found: {cycle_id}` 에러 반환 |
| cycle에 배정된 태스크가 없음 | `No tasks assigned to this cycle.` 메시지 반환 |
| transcript JSONL 파일 없음 | tokens 컬럼에 "—" 표시 (graceful skip, Feature 25와 동일) |
| git 브랜치/repo 없음 | task_size 컬럼에 "—" 표시 (graceful skip, Feature 25와 동일) |
| DB 조회 실패 (I/O 에러) | anyhow 에러 전파, CLI exit non-zero |

## 제약 조건

- 읽기 전용 연산이므로 성능 제약 완화 (< 500ms)
- 기존 `domain::report::TaskReport` 구조체를 재사용하여 코드 중복 최소화
- 기존 `workflow::report`의 `compute_proxy`, `compute_tokens` 헬퍼를 공유

## 구현 범위

### domain 계층

- `domain/cycle_report.rs` (신규)
  - `CycleReportCategory` enum: `PlannedDone`, `PlannedIncomplete`, `UnplannedDone`
  - `classify(assigned: Assigned, status_category: StatusCategory) -> CycleReportCategory` 순수 함수
  - `CycleSummary` 구조체: completion_rate, throughput, avg_flow_eff, ftd_rate
  - `compute_summary(planned_done: usize, planned_incomplete: usize, reports: &[TaskReport]) -> CycleSummary` 순수 함수
  - `format_cycle_report(...)` 포맷팅 순수 함수 (헤더, 섹션별 테이블, 요약)

### adapter 계층

- `adapter/cycle_task_repo.rs` 기존 파일에 함수 추가
  - `list_by_cycle(conn, cycle_id) -> Vec<(String, Assigned)>`: cycle에 배정된 (task_id, assigned) 목록 조회

### workflow 계층

- `workflow/cycle_report.rs` (신규)
  - `run(conn: &Connection, cycle_id: &str) -> Result<String>`: 오케스트레이션
  - 기존 `workflow::report`의 `compute_proxy`, `compute_tokens` 헬퍼를 `pub(crate)`로 공개하여 재사용

### entrypoint 계층

- `entrypoint/cycle.rs`에 `report(conn, cycle_id)` 함수 추가
- `main.rs`에 `Cycle::Report` 서브커맨드 추가
- `entrypoint/mcp/`에 `cycle_report` 도구 추가

## 의존하는 기능

- Feature 32 (SEO-20): cycle CRUD
- Feature 35 (SEO-22): cycle-task 배정
- Feature 34 (SEO-24): cycle status 날짜 파생
- Feature 25 (SEO-9): 태스크 리포트 (TaskReport, 지표 계산 로직)

---

## QA 목록

### 정상 동작

1. cycle_id로 조회한 cycle의 name, start_date, end_date, 파생된 status가 헤더에 표시된다.
2. assigned=planned AND status_category=Completed인 태스크가 "Planned Done" 섹션에 표시된다.
3. assigned=planned AND status_category!=Completed인 태스크가 "Planned Incomplete" 섹션에 표시된다.
4. assigned=auto AND status_category=Completed인 태스크가 "Unplanned Done" 섹션에 표시된다.
5. Planned Done 태스크에 cycle_time, lead_time, tokens, task_size, rework 지표가 표시된다.
6. Planned Incomplete 태스크에 현재 status name과 issue_age가 표시된다.
7. Unplanned Done 태스크에 Planned Done과 동일한 지표가 표시된다.
8. completion_rate = planned_done / (planned_done + planned_incomplete)로 계산된다.
9. throughput = planned_done + unplanned_done으로 계산된다.
10. flow_efficiency(avg)는 완료 태스크의 flow_efficiency 평균이다.
11. first_time_done은 완료 태스크 중 rework 없는 비율이다.
12. 특정 구분에 태스크가 없으면 해당 섹션을 생략한다.
13. assigned=auto AND status_category!=Completed인 태스크는 리포트에 포함되지 않는다.

### 에러 처리

14. 존재하지 않는 cycle_id → `Cycle not found: {cycle_id}` 에러 메시지.
15. cycle에 배정된 태스크가 0개 → `No tasks assigned to this cycle.` 메시지.

### Graceful skip

16. transcript 파일 없는 태스크 → tokens 컬럼에 "—" 표시.
17. git 브랜치 없는 태스크 → task_size 컬럼에 "—" 표시.

### MCP

18. `cycle_report` MCP 도구가 CLI와 동일한 리포트 문자열을 반환한다.

---

## Test Pyramid

| QA | 레벨 | 이유 |
|---|---|---|
| Q1 | 단위 | 포맷 순수 함수 (cycle 정보 → 헤더 문자열) |
| Q2 | 단위 | 분류 순수 함수 (assigned + status_category → 구분) |
| Q3 | 단위 | 동일 |
| Q4 | 단위 | 동일 |
| Q5 | 통합 | workflow에서 DB 조회 + 지표 계산 조합 |
| Q6 | 단위 | issue_age 계산 순수 함수 (이미 domain/task_metrics에 존재) |
| Q7 | 통합 | Q5와 동일 패턴 |
| Q8 | 단위 | completion_rate 계산 순수 함수 |
| Q9 | 단위 | throughput 계산 (카운트) |
| Q10 | 단위 | avg 계산 (기존 로직 재사용) |
| Q11 | 단위 | first_time_done_rate (기존 함수 재사용) |
| Q12 | 단위 | 포맷 함수에서 빈 구분 생략 |
| Q13 | 단위 | 분류 순수 함수에서 auto+미완료 → 제외 |
| Q14 | 통합 | workflow에서 cycle 미존재 에러 |
| Q15 | 통합 | workflow에서 빈 태스크 메시지 |
| Q16 | 통합 | workflow에서 transcript/git 미존재 시 None → "—" (기존 TaskReport 로직 재사용) |
| Q17 | 통합 | Q16과 동일 |
| Q18 | E2E | MCP 도구 전체 흐름 |
