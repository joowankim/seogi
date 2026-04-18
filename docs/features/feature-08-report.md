# Feature 08: 리포트 (`seogi report`)

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

기간별/프로젝트별 세션 메트릭을 SQLite에서 집계하여 터미널에 통계 테이블을 출력한다.

**Ground Truth 연결:**
- **정량 측정**: 수집된 프록시 지표 10개를 기간별로 집계(평균, 중앙값, σ, P25, P75)하여 하니스 성능을 정량적으로 보고
- **동치 보장**: 하니스 변경 전후 기간의 통계를 비교하여 업무 효율 동등성을 검증하는 직접적인 도구

---

## 입력

| 항목 | 설명 |
|------|------|
| CLI 인자 | `seogi report --from <YYYY-MM-DD> --to <YYYY-MM-DD> [--project <name>]` |
| 환경변수 | `SEOGI_DB_PATH` (선택, 테스트용) |
| DB 상태 | `tool_uses`, `tool_failures` 테이블에 데이터 존재 |

---

## 출력

stdout에 기존 `commands/report.rs`와 동일한 터미널 테이블 형식으로 출력한다.

### 출력 형식

```
기간: 2026-04-01 ~ 2026-04-15 (n=12 세션)
프로젝트: 전체

                           평균   중앙값        σ      P25      P75
read_before_edit          3.2      3.0      1.5      2.0      4.0
doom_loop_count           0.5      0.0      0.8      0.0      1.0
tool_call_count          42.3     38.0     12.1     30.0     52.0
session_duration_sec    180.5    165.0     45.2    140.0    210.0
bash_error_rate          10.5%    8.0%     5.2%     5.0%    15.0%
edit_files_count          4.2      3.0      2.1      2.0      6.0

test_invoked              75%
build_invoked             25%
lint_invoked              50%
typecheck_invoked         33%
```

| 항목 | 설명 |
|------|------|
| exit 0 | 성공 (데이터 유무 무관) |
| exit 1 | DB 접근 불가 등 에러 |

---

## 데이터 흐름

1. `--from`/`--to`를 밀리초 Unix timestamp로 변환
2. `tool_uses`에서 해당 timestamp 범위 + 프로젝트 필터로 고유 `session_id` 목록 조회
3. 각 session_id에 대해 `tool_uses`와 `tool_failures`를 로드하고 `metrics::calculate()`로 지표 산출
4. 전체 `SessionMetrics` 목록에 대해 통계 집계 (순수 함수)
5. 터미널 출력

---

## 성공 시나리오

1. `seogi report --from 2026-04-01 --to 2026-04-15`가 실행된다.
2. DB에서 해당 기간의 고유 session_id 목록을 조회한다.
3. 각 세션에 대해 메트릭을 계산한다.
4. 전체 메트릭을 집계하여 통계를 산출한다.
5. 터미널에 테이블을 출력하고 exit 0으로 종료한다.

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| 해당 기간에 세션이 0건 | "해당 기간에 데이터가 없습니다." 출력 후 exit 0 |
| DB 접근 불가 | exit 1 + stderr에 에러 메시지 |
| CLI 인자 누락 (from/to) | clap 자동 에러 처리 |
| 날짜 형식 잘못됨 | exit 1 + stderr에 파싱 에러 |
| `--from` > `--to` (역전 범위) | 빈 결과로 처리 (exit 0 + "데이터가 없습니다") |

---

## 제약 조건

- **출력 호환성**: 기존 `commands/report.rs`와 동일한 터미널 테이블 형식
- **통계**: n=1이면 σ=0.0 (분산 계산 시 n-1 분모)
- **boolean 지표**: 전체 세션 중 true 비율을 %로 표시
- **bash_error_rate**: 비율을 %로 변환하여 통계 표시

---

## 의존 Feature

- **Feature 01: DB 초기화** — `tool_uses`, `tool_failures` 테이블
- **Feature 02/03: 도구 사용/실패 로깅** — 원시 데이터
- **Feature 06: 세션 분석** — `metrics::calculate()` 순수 함수 재사용

---

## 구현 범위

### 수직 슬라이스

```
domain/report.rs          compute_stats, aggregate 순수 함수 [신규]
    ↓
adapter/log_repo.rs       list_session_ids_by_range [추가]
    ↓
workflow/report.rs        Impureim Sandwich: load → calculate → aggregate → format [신규]
    ↓
main.rs                   Report 서브커맨드 변경 [수정]
```

### 신규 파일

| 파일 | 내용 |
|------|------|
| `domain/report.rs` | `Stats` 구조체, `compute_stats(values) -> Stats`, `format_report(metrics, from, to, project) -> String` |
| `workflow/report.rs` | `run(conn, from, to, project) -> Result<String>` |

### 기존 파일 변경

| 파일 | 변경 내용 |
|------|-----------|
| `domain/mod.rs` | `pub mod report;` 추가 |
| `adapter/log_repo.rs` | `list_session_ids_by_range(conn, from_ts, to_ts, project) -> Vec<String>` 추가 |
| `workflow/mod.rs` | `pub mod report;` 추가 |
| `main.rs` | `Report` 서브커맨드를 새 workflow로 연결 |

### 재사용 모듈 (변경 없음)

| 파일 | 재사용 함수 |
|------|-----------|
| `domain/metrics.rs` | `calculate(session_id, tool_uses, tool_failures) -> SessionMetrics` |
| `adapter/log_repo.rs` | `list_by_session`, `list_failures_by_session` |
| `entrypoint/hooks/mod.rs` | `db_path()` |

---

## QA 목록

### 통계 계산 (순수 함수)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | `[1.0, 2.0, 3.0, 4.0, 5.0]`에서 mean이 3.0이다 | 단위 |
| Q2 | `[1.0, 2.0, 3.0, 4.0, 5.0]`에서 median이 3.0이다 | 단위 |
| Q3 | `[1.0, 2.0, 3.0, 4.0, 5.0]`에서 P25가 2.0, P75가 4.0이다 | 단위 |
| Q4 | `[1.0, 2.0, 3.0, 4.0, 5.0]`에서 σ가 약 1.581(`sqrt(2.5)`)이다 | 단위 |
| Q5 | 값이 1개일 때 σ가 0.0이다 | 단위 |
| Q6 | 빈 배열에서 모든 통계가 0.0이다 | 단위 |

### 출력 포맷

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q7 | 3개 세션 메트릭을 format_report에 전달하면 "n=3 세션"이 포함된 문자열이 반환된다 | 단위 |
| Q8 | boolean 지표 test_invoked가 3개 중 2개 true이면 "67%"가 포함된다 | 단위 |
| Q9 | 빈 메트릭 목록을 format_report에 전달하면 "데이터가 없습니다"가 포함된다 | 단위 |

### adapter

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q10 | timestamp 1000~5000 범위의 tool_uses 3건(session A, B, C)에서 `list_session_ids_by_range(1000, 5000, None)`이 3개 session_id를 반환한다 | 통합 |
| Q11 | project="proj1"으로 필터하면 해당 프로젝트의 session_id만 반환된다 | 통합 |
| Q12 | 범위 밖의 tool_uses는 반환되지 않는다 | 통합 |

### workflow

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q13 | tool_uses 3건(2개 세션)이 있는 DB에서 `workflow::report::run`이 "n=2 세션"을 포함하는 문자열을 반환한다 | 통합 |

### E2E

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q14 | `seogi report --from 2026-04-01 --to 2026-04-30` 실행 시 stdout에 통계 테이블이 출력되고 exit 0이다 | E2E |
| Q15 | 데이터 없는 기간에 대해 실행 시 "데이터가 없습니다" 출력 + exit 0이다 | E2E |
| Q16 | `--from`/`--to` 없이 실행 시 exit != 0이다 | E2E |
| Q17 | 존재하지 않는 DB 경로로 실행 시 exit 1이고 stderr에 에러 메시지가 출력된다 | E2E |
| Q18 | `--from invalid-date` 형식으로 실행 시 exit 1이고 stderr에 파싱 에러가 출력된다 | E2E |

---

## Test Pyramid

### Unit Tests (domain/report.rs)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_compute_stats_basic` | Q1, Q2, Q3 | mean, median, P25, P75 |
| `test_compute_stats_stddev` | Q4 | σ 정상 계산 |
| `test_compute_stats_single` | Q5 | n=1 → σ=0 |
| `test_compute_stats_empty` | Q6 | 빈 배열 |
| `test_format_report_with_data` | Q7, Q8 | 세션 수, boolean % |
| `test_format_report_empty` | Q9 | 빈 메트릭 → 안내 메시지 |

### Integration Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_list_session_ids_by_range` | Q10 | 범위 내 session_id 조회 |
| `test_list_session_ids_by_project` | Q11 | 프로젝트 필터 |
| `test_list_session_ids_out_of_range` | Q12 | 범위 밖 제외 |
| `test_workflow_report_run` | Q13 | 전체 흐름 → 출력 문자열 |

### E2E Tests

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_report_command_outputs_table` | Q14 | 바이너리 → stdout 테이블 |
| `test_report_command_empty_period` | Q15 | 빈 기간 → 안내 메시지 |
| `test_report_command_no_args` | Q16 | 인자 없음 → exit != 0 |
| `test_report_command_bad_db` | Q17 | 잘못된 DB → exit 1 |
| `test_report_command_invalid_date` | Q18 | 잘못된 날짜 → exit 1 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과
- [ ] 사용자 승인 완료
