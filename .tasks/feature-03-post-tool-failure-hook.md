# Feature 03: 도구 실패 로깅 (`seogi hook post-tool-failure`)

## 작업 목표

Claude Code의 PostToolUseFailure 훅을 Rust로 구현하여, 도구 호출이 실패했을 때 SQLite에 기록한다.

## 완료 기준

- [ ] `domain/log.rs`에 `ToolFailure` 타입 추가
- [ ] `adapter/log_repo.rs`에 `save_tool_failure` 함수 추가
- [ ] `workflow/log_failure.rs`에 workflow 함수
- [ ] `entrypoint/hooks/post_tool_failure.rs`에 stdin 파싱 → workflow 호출
- [ ] `seogi hook post-tool-failure` 서브커맨드 동작
- [ ] Feature 2와 동일 패턴의 단위/통합/E2E 테스트
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-03-post-tool-failure-hook.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. Feature 2의 패턴을 따라 E2E → domain → adapter → workflow → entrypoint 구현
4. 테스트 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 3 섹션
- `docs/plans/2026-04-15-task-management.md` — tool_failures 테이블 스키마
- Feature 02의 구현 코드를 패턴 참조로 사용

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 02 (패턴 참조)

## 워크트리

```bash
git worktree add -b feature/03-post-tool-failure-hook .worktrees/03-post-tool-failure-hook origin/main
```
