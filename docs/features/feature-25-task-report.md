# Feature 25: seogi report 태스크 중심 교체

## 목적

ground-truth 목적 1(하니스 성능을 정량적으로 측정하는 수단과 지표를 다수 확보하기)에 기여: SEO-5~8에서 구현한 개별 지표를 하나의 리포트로 통합하여 태스크 단위 성과를 한눈에 파악할 수 있는 수단을 제공한다.

ground-truth 목적 2(하니스 변경 전후의 동등한 업무 효율을 측정하고 보장하기)에 기여: 리포트의 기간 필터를 통해 하니스 변경 전후 태스크 지표를 비교할 수 있어 baseline 비교의 기반이 된다.

## 입력

- CLI 인자:
  - `--from` (필수): `String` (YYYY-MM-DD 형식). 해당 날짜 00:00:00 UTC 밀리초 timestamp로 변환
  - `--to` (필수): `String` (YYYY-MM-DD 형식). 해당 날짜 23:59:59.999 UTC 밀리초 timestamp로 변환
  - `--project`: `Option<String>`. 프로젝트 이름 필터
  - `--detail`: `bool` 플래그 (기본값 false). 태스크별 상세 출력
- 시스템 입력: tasks, task_events, statuses, tool_uses, tool_failures 테이블, transcript 파일, git repo

## 출력

- stdout: 요약 테이블 또는 --detail 상세 텍스트
- exit code: 성공 시 0, 에러 시 non-zero
- stderr: anyhow 에러 메시지 (에러 시)

### 기본 출력 (요약 테이블)

```
ID      TITLE              CYCLE    LEAD     TOKENS   SIZE    REWORK
SEO-1   MCP 부트스트랩       2h30m    1d4h     45,230   +342    no
SEO-2   MCP 도구 구현        3h15m    2d1h     62,100   +580    no
---
throughput: 3 tasks    flow_efficiency(avg): 0.48    first_time_done: 100%
```

### --detail 출력 (태스크별 상세)

```
=== SEO-1: MCP 서버 부트스트랩 ===
cycle_time: 2h 30m    lead_time: 1d 4h    flow_efficiency: 0.52
tokens: 45,230 (input: 38,120 / output: 7,110)
task_size: +342 -28 (5 files)
test_invoked: true    doom_loop: 0    bash_error_rate: 0.02
```

## 성공 시나리오

1. `--from`/`--to` 날짜를 밀리초 timestamp로 변환
2. `task_events`에서 해당 기간 내 Completed 카테고리(시딩된 상태명: `done`)로 전환된 태스크 목록 추출
3. `--project` 필터 적용 (옵션)
4. 각 태스크에 대해:
   a. task_events로 cycle_time, lead_time, flow_efficiency 등 계산 (SEO-5)
   b. Started~Completed 시간 범위로 tool_uses/tool_failures 조회 → proxy metrics 계산 (SEO-6)
   c. tool_uses에서 session_id 추출 → transcript 파싱으로 토큰 사용량 계산 (SEO-7)
   d. git diff로 task_size 계산 (SEO-8)
5. 요약 테이블 또는 --detail 상세 출력

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| 기간 내 Completed 태스크 없음 | "No completed tasks in the given period." 출력, exit 0 |
| 날짜 형식 오류 (YYYY-MM-DD 아님) | 파싱 에러 메시지, exit non-zero |
| `--from` > `--to` | "Invalid date range: --from must be before --to" 에러 |
| transcript 파일 없음 | 토큰 "—" 표시 (graceful skip) |
| 브랜치 없음 (task_size) | SIZE "—" 표시 (graceful skip) |
| DB 조회 실패 | anyhow 에러 전파, exit non-zero |

## 제약 조건

- workflow 계층에서 adapter/domain 함수를 조합 (Impureim Sandwich)
- 기존 함수를 최대한 재사용, 새 domain/adapter 함수 최소화
- 날짜→timestamp 변환은 workflow에서 처리

## 의존하는 기능

- Feature 21 (SEO-5): 태스크 지표 도메인 — 완료
- Feature 22 (SEO-6): 프록시 지표 태스크 단위 집계 — 완료
- Feature 23 (SEO-7): transcript 파싱 — 완료
- Feature 24 (SEO-8): task_size — 완료

## 구현 범위

### entrypoint 계층

| 파일 | 변경 |
|------|------|
| `main.rs` | `Report` 서브커맨드 추가 (--from, --to, --project, --detail) |

### workflow 계층

| 파일 | 변경 |
|------|------|
| `workflow/report.rs` (새 파일) | `run` 함수 — 날짜 파싱, 태스크 조회, 지표 계산, 포맷팅 조합 |

### domain 계층

| 파일 | 변경 |
|------|------|
| `domain/report.rs` (새 파일) | `TaskReport` 구조체 (태스크별 통합 지표), `format_summary`/`format_detail` 순수 포맷팅 함수 |

### adapter 계층

| 파일 | 변경 |
|------|------|
| `adapter/task_repo.rs` | `list_completed_tasks_in_range` 함수 추가 (기간 내 완료된 태스크 목록 + 프로젝트 필터) |

---

## QA 목록

1. `seogi report --from 2026-04-01 --to 2026-04-30` → 기간 내 완료 태스크 요약 테이블 출력
2. `--project` 필터 적용 시 해당 프로젝트의 태스크만 출력
3. `--detail` 플래그 시 태스크별 상세 출력 (cycle_time, lead_time, tokens, task_size, proxy metrics 포함)
4. 기간 내 완료 태스크 없으면 "No completed tasks in the given period." 출력
5. 요약 테이블의 CYCLE 컬럼이 `task_metrics::cycle_time()` 결과를 사람이 읽을 수 있는 형식으로 표시
6. 요약 테이블의 TOKENS 컬럼이 `transcript::read_token_usage()` 결과의 `total()`과 일치
7. 요약 테이블의 SIZE 컬럼이 `git::diff_stat()` 결과의 `additions`과 일치 ("+N" 형식)
8. --detail 출력에 test_invoked, doom_loop_count, bash_error_rate 필드가 `metrics::calculate()` 결과와 일치
9. 요약 테이블의 REWORK 컬럼이 rework_rate 기반으로 "yes"/"no" 표시 (Completed→Started 전환 발생 시 "yes")
10. 하단 throughput = 완료 태스크 수, flow_efficiency(avg) = 각 태스크 flow_efficiency의 산술 평균(소수점 2자리), first_time_done = rework 없는 비율(%)
11. transcript 없으면 TOKENS 컬럼에 "—" 표시
12. 브랜치 없으면 SIZE 컬럼에 "—" 표시
13. `--from 2026-04-30 --to 2026-04-01` → "Invalid date range" 에러

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---------|------|------|
| 1-3. CLI 출력 | E2E | 전체 흐름 검증 |
| 4. 빈 결과 메시지 | 통합 | workflow 함수 반환값 검증 |
| 5-8. 지표 정확성 | 단위 | TaskReport → format 순수 함수 + 계산 로직 |
| 9-10. 집계 정확성 | 단위 | 순수 집계 함수 |
| 11-12. graceful skip | 통합 | workflow에서 None 처리 |
| 13. 날짜 에러 | 단위 | 날짜 파싱 함수 테스트 |

| 레벨 | 항목 수 |
|------|---------|
| 단위 | 7 |
| 통합 | 3 |
| E2E | 3 |
