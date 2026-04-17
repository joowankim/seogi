# Feature 08: 리포트 (`seogi report`)

## 작업 목표

기간/프로젝트별 메트릭 통계를 집계하여 터미널에 출력한다. 기존 `seogi report` 기능을 SQLite 기반으로 재구현한다.

## 완료 기준

- [ ] `domain/metrics.rs`에 통계 집계 순수 함수 (`aggregate`: 평균, 중앙값, σ, P25, P75)
- [ ] `adapter/metrics_repo.rs`에 `list_by_range` 함수 (날짜 범위 + 프로젝트 필터)
- [ ] `workflow/report.rs`에 Impureim Sandwich (load → aggregate → format)
- [ ] `entrypoint/cli/report.rs`에 `seogi report --from --to [--project]` 서브커맨드
- [ ] 기존 출력 포맷과 동일한 터미널 테이블 출력
- [ ] `--project` 생략 시 전체 프로젝트 합산
- [ ] boolean 지표는 %(비율)로, 수치 지표는 통계값으로 표시
- [ ] 빈 기간 → 안내 메시지
- [ ] n=1 세션 → σ 생략
- [ ] 통계 계산 순수 함수 단위 테스트
- [ ] 기존 리포트 출력과 동일 결과 회귀 테스트
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-08-report.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. 통계 집계 순수 함수 + 단위 테스트 (Inside-out)
4. metrics_repo 쿼리 함수 + 통합 테스트
5. report workflow + 통합 테스트
6. CLI 서브커맨드 연결
7. 기존 출력과 비교 회귀 테스트
8. 테스트 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 8 섹션
- 기존 `cli/src/commands/report.rs` — 현재 리포트 출력 포맷 및 통계 계산 로직

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 06 (메트릭이 DB에 있어야 집계 가능)

## 워크트리

```bash
git worktree add -b feature/08-report .worktrees/08-report origin/main
```
