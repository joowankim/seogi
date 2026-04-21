# Feature 20: session_metrics 테이블 DROP + seogi analyze 제거

## 목적

직접적으로 새 지표를 확보하지는 않으나, ground-truth 목적 1의 후속 작업(SEO-6: 태스크 단위 지표 산출)을 위한 **선행 리팩토링**. 세션 중심 인프라(`session_metrics` 테이블, `seogi analyze`, `seogi report`)를 제거하여 태스크 중심 측정 구조로의 전환 경로를 확보한다.

## 입력

- 시스템 입력: 기존 DB 스키마 (`session_metrics` 테이블), 기존 코드 (analyze/report 워크플로우)

## 출력

- 부수효과: `session_metrics` 테이블 스키마 제거, `seogi analyze` 서브커맨드 제거, 세션 기반 `seogi report` 제거, 관련 코드 삭제

## 성공 시나리오

1. `seogi migrate` 또는 DB 초기화 시 `session_metrics` 테이블이 생성되지 않음
2. 기존 DB에 `session_metrics` 데이터가 있어도 `DROP TABLE IF EXISTS`로 안전 제거
3. `seogi analyze` 명령어 실행 시 clap이 "unrecognized subcommand" 에러 반환
4. `seogi report` 명령어 실행 시 clap이 "unrecognized subcommand" 에러 반환
5. 프록시 지표 계산 순수 함수(`calculate` 등)는 그대로 유지됨
6. `cargo test` 전체 통과, `cargo clippy` 통과

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| 기존 DB에 session_metrics 데이터 존재 | `DROP TABLE IF EXISTS`로 제거 — 데이터 유실 허용 (태스크 중심으로 대체 예정) |
| 삭제 대상 코드가 예상 외 모듈에서 참조됨 | `cargo check` 실패로 조기 감지 — 해당 참조를 추적하여 삭제/수정 |
| DROP TABLE 시 다른 테이블의 FK 참조 존재 | 현재 스키마에 session_metrics를 참조하는 FK 없음 — 해당 없음 |

## 제약 조건

- `domain/metrics.rs`의 `calculate` 함수와 프록시 지표 계산 로직(`calc_read_before_edit`, `calc_doom_loop_count` 등)은 SEO-6에서 태스크 단위로 재사용하므로 삭제 금지
- `SessionMetrics` 구조체만 삭제하고, 계산 함수들은 유지

## 의존하는 기능

- Feature 10 (SQLite 스키마) — 이미 구현됨

## 삭제 대상 목록

### 파일 삭제 (5개)

| 파일 | 이유 |
|------|------|
| `cli/src/workflow/analyze.rs` | analyze 워크플로우 전체 |
| `cli/src/workflow/report.rs` | 세션 기반 report 워크플로우 |
| `cli/src/domain/report.rs` | 세션 기반 report 도메인 (Stats, format_report 등) |
| `cli/tests/analyze_command_test.rs` | analyze E2E 테스트 |
| `cli/tests/report_command_test.rs` | report E2E 테스트 |

### 파일 수정 (5개)

| 파일 | 변경 내용 |
|------|-----------|
| `cli/src/adapter/sql/schema.sql` | `session_metrics` 테이블 정의 제거 + `DROP TABLE IF EXISTS session_metrics` 추가 |
| `cli/src/main.rs` | `Analyze`, `Report` 서브커맨드 및 핸들러 제거 |
| `cli/src/domain/metrics.rs` | `SessionMetrics` 구조체 삭제 (calculate 함수 및 calc_* 함수 유지) |
| `cli/src/adapter/log_repo.rs` | `list_session_ids_by_range` 함수 및 관련 테스트 삭제 |
| `cli/src/adapter/db.rs` | `EXPECTED_TABLES`에서 `session_metrics` 제거 (9→8), `test_schema_columns_session_metrics` 테스트 삭제 |

### 모듈 선언 제거

| 파일 | 변경 |
|------|------|
| `cli/src/workflow/mod.rs` | `pub mod analyze;` 제거 |
| `cli/src/workflow/mod.rs` | `pub mod report;` 제거 |
| `cli/src/domain/mod.rs` | `pub mod report;` 제거 |

---

## QA 목록

1. `schema.sql`에 `session_metrics` CREATE TABLE 문이 없음
2. `schema.sql`에 `DROP TABLE IF EXISTS session_metrics` 문이 존재함
3. `seogi analyze x` 실행 시 clap 에러 (unrecognized subcommand)
4. `seogi report --from 2026-01-01 --to 2026-01-31` 실행 시 clap 에러
5. `domain/metrics.rs`의 `calculate` 함수가 존재하고 컴파일됨
6. `domain/metrics.rs`의 `calc_read_before_edit`, `calc_doom_loop_count`, `calc_invoked`, `calc_session_duration`, `calc_edit_files`, `calc_bash_error_rate` 함수가 존재함
7. `domain/metrics.rs`에 `SessionMetrics` 구조체가 없음
8. `adapter/log_repo.rs`에 `list_session_ids_by_range` 함수가 없음
9. `adapter/db.rs`의 `EXPECTED_TABLES`에 `session_metrics`가 없음
10. `cargo test` 전체 통과
11. `cargo clippy` 경고 없음

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---------|------|------|
| 1. schema에 session_metrics 없음 | 통합 | DB 초기화 후 테이블 목록 확인 |
| 2. DROP TABLE IF EXISTS 존재 | 단위 | schema.sql 텍스트 검증 (기존 db.rs 테스트로 커버) |
| 3. analyze 명령어 제거 | E2E | 바이너리 실행 결과 확인 |
| 4. report 명령어 제거 | E2E | 바이너리 실행 결과 확인 |
| 5-6. calculate 및 calc_* 함수 유지 | 단위 | 기존 테스트가 계속 통과하면 충족 |
| 7. SessionMetrics 구조체 없음 | 단위 | 컴파일 시 검증 (grep으로 확인) |
| 8. list_session_ids_by_range 없음 | 단위 | 컴파일 시 검증 |
| 9. EXPECTED_TABLES 업데이트 | 통합 | 기존 db.rs 테스트가 통과하면 충족 |
| 10. cargo test 통과 | E2E | CI 수준 검증 |
| 11. cargo clippy 통과 | E2E | CI 수준 검증 |

### 분배 요약

| 레벨 | 항목 수 | 비고 |
|------|---------|------|
| 단위 | 4 | 기존 테스트 유지로 커버 |
| 통합 | 2 | 기존 db.rs 테스트로 커버 |
| E2E | 5 | 바이너리 실행 확인 (analyze/report 제거 확인은 기존 테스트 삭제로 대응, cargo test/clippy로 검증) |

> 리팩토링 태스크 특성상 **새 테스트 작성보다 기존 테스트 삭제 + 나머지 테스트 통과 확인**이 핵심.
