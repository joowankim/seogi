# Feature 22: 프록시 지표의 태스크 단위 집계

## 목적

ground-truth 목적 1(하니스 성능을 정량적으로 측정하는 수단과 지표를 다수 확보하기)에 직접 기여.
기존 프록시 지표 8개를 태스크의 Started~Completed 시간 범위 내 tool_uses/tool_failures에서 계산하여, 태스크 단위 성과 측정의 기반을 마련한다.

## 입력

- adapter 계층: `from_ts: i64`, `to_ts: i64` (밀리초 Unix timestamp) → DB에서 시간 범위 내 tool_uses/tool_failures 조회
- domain 계층: `&[ToolUse]`, `&[ToolFailure]` (adapter가 조회한 슬라이스) → 순수 계산

## 출력

- 반환값: `TaskProxyMetrics` 구조체 (프록시 지표 8개, `session_duration`과 `edit_files` 제외 — 태스크 단위에서 의미가 다르므로 SEO-9에서 필요 시 추가)
- 부수효과: 없음 (on-the-fly 계산, DB 저장 없음)

## 성공 시나리오

1. 태스크 ID로 task_events를 조회하여 첫 Started~첫 Completed 타임스탬프를 결정
2. 해당 시간 범위 내 tool_uses/tool_failures를 조회
3. 기존 `calc_*` 순수 함수들로 8개 프록시 지표를 계산
4. `TaskProxyMetrics` 구조체로 반환

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| 태스크에 Started 이벤트가 없음 | None 반환 (미시작 태스크) |
| 태스크에 Completed 이벤트가 없음 | None 반환 (진행 중 태스크) |
| Started/Completed가 여러 개 (rework 등) | **첫 Started ~ 첫 Completed** 범위만 사용 (설계 문서 정의와 일치, cycle_time과 동일 구간) |
| 시간 범위 내 tool_uses가 없음 | 모든 지표 기본값 (0, false, 0.0) |
| DB 조회 실패 | `Result<_, AdapterError>` 반환, 호출자(workflow)가 에러 전파 |

## 제약 조건

- SEO-4에서 `pub`으로 변경된 `calc_*` 함수를 그대로 재사용
- domain의 `calculate`는 `&[ToolUse]`, `&[ToolFailure]` 슬라이스를 받는 순수 함수. 시간 범위 조회는 adapter 계층이 담당
- on-the-fly 계산 (DB 저장 없음, 개인 도구이므로 데이터량 적음)

## 의존하는 기능

- Feature 20 (SEO-4) — SessionMetrics 제거, calc_* pub 유지 (완료)

## 구현 범위

### domain 계층

| 파일 | 변경 |
|------|------|
| `domain/metrics.rs` | `TaskProxyMetrics` 구조체 추가, `calculate` 함수 재구현 (tool_uses/tool_failures 슬라이스 → TaskProxyMetrics) |

`TaskProxyMetrics` 필드 8개:
- `read_before_edit_ratio: u32`
- `doom_loop_count: u32`
- `test_invoked: bool`
- `build_invoked: bool`
- `lint_invoked: bool`
- `typecheck_invoked: bool`
- `tool_call_count: u32`
- `bash_error_rate: f64`

> `session_duration`, `edit_files`는 태스크 단위에서 의미가 다르므로 제외. SEO-9에서 필요 시 추가.

### adapter 계층

| 파일 | 변경 |
|------|------|
| `adapter/log_repo.rs` | `list_by_time_range(conn, from_ts, to_ts)` 함수 추가 (tool_uses) |
| `adapter/log_repo.rs` | `list_failures_by_time_range(conn, from_ts, to_ts)` 함수 추가 (tool_failures) |

### workflow 계층

변경 없음. SEO-9(report)에서 통합 시 workflow에서 조합.

---

## QA 목록

1. `TaskProxyMetrics` 구조체가 8개 필드를 가짐
2. `calculate(&[ToolUse], &[ToolFailure])` → `TaskProxyMetrics` 반환
3. 빈 tool_uses/tool_failures → 모든 지표 기본값 (0, false, 0.0)
4. tool_uses에 Read→Edit 순서 → `read_before_edit_ratio` 정확히 계산
5. 동일 파일 Edit 5회 이상 → `doom_loop_count` 정확히 계산
6. Bash command에 test 패턴 → `test_invoked: true`
7. Bash command에 build 패턴 → `build_invoked: true`
8. Bash command에 lint 패턴 → `lint_invoked: true`
9. Bash command에 typecheck 패턴 → `typecheck_invoked: true`
10. tool_uses 개수 → `tool_call_count` 일치
11. Bash 실패/전체 비율 → `bash_error_rate` 정확히 계산
12. `list_by_time_range` — 범위 내 tool_uses만 반환
13. `list_by_time_range` — 범위 밖 tool_uses 제외
14. `list_failures_by_time_range` — 범위 내 tool_failures만 반환
15. `list_failures_by_time_range` — 범위 밖 tool_failures 제외
16. `cargo test` 통과
17. `cargo clippy` 통과

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---------|------|------|
| 1. 구조체 필드 | 단위 | 컴파일 시 검증 |
| 2. calculate 반환 | 단위 | 순수 함수 |
| 3. 빈 입력 기본값 | 단위 | 순수 함수 |
| 4-11. 개별 지표 | 단위 | 기존 calc_* 테스트 + calculate 통합 테스트 |
| 12-13. time_range tool_uses | 통합 | DB 조회 |
| 14-15. time_range tool_failures | 통합 | DB 조회 |
| 16. cargo test | E2E | CI 검증 |
| 17. cargo clippy | E2E | CI 검증 |

| 레벨 | 항목 수 |
|------|---------|
| 단위 | 11 |
| 통합 | 4 |
| E2E | 2 |
