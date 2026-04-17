# Feature 06: 세션 분석 (`seogi analyze`)

## 작업 목표

세션 로그(tool_uses, tool_failures, system_events)에서 프록시 지표 10개를 계산하여 session_metrics 테이블에 저장한다. stop 훅에서 백그라운드로 호출된다.

## 완료 기준

- [ ] `domain/metrics.rs`에 `SessionMetrics` 타입 + `calculate` 순수 함수
- [ ] `adapter/metrics_repo.rs`에 `save`, `find_latest` 함수
- [ ] `workflow/analyze.rs`에 Impureim Sandwich (load → calculate → save)
- [ ] `entrypoint/cli/analyze.rs`에 `seogi analyze <project> <session_id>` 서브커맨드
- [ ] 10개 지표가 모두 올바르게 계산됨 (read_before_edit, doom_loop, test/build/lint/typecheck_invoked, tool_call_count, session_duration, edit_files, bash_error_rate)
- [ ] `metrics::calculate` 순수 함수 단위 테스트 (지표별 독립 테스트)
- [ ] `workflow::analyze::run` 통합 테스트
- [ ] 기존 Rust 분석기와 동일한 결과 회귀 테스트
- [ ] Feature 4의 `hooks/stop.rs`에서 `seogi analyze`를 백그라운드로 호출하도록 연결
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-06-analyze.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. E2E 테스트 선작성 (RED)
4. `domain/metrics.rs` 순수 함수 + 단위 테스트 (Inside-out)
5. `adapter/metrics_repo.rs` + 통합 테스트
6. `workflow/analyze.rs` + 통합 테스트
7. `entrypoint/cli/analyze.rs` 연결
8. `hooks/stop.rs`에서 백그라운드 호출 연결
9. E2E 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 6 섹션
- `docs/plans/2026-04-15-task-management.md` — session_metrics 테이블 스키마
- 기존 `cli/src/analyzers/session_summary.rs` — 현재 지표 계산 로직 참조

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 02, 03, 04 (로그 데이터가 DB에 있어야 분석 가능)

## 워크트리

```bash
git worktree add -b feature/06-analyze .worktrees/06-analyze origin/main
```
