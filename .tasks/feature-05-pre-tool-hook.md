# Feature 05: 도구 호출 시작 시간 기록 (`seogi hook pre-tool`)

## 작업 목표

Claude Code의 PreToolUse 훅을 Rust로 구현하여, 도구 호출 시작 시각을 기록한다. Feature 02의 post-tool 훅이 이 시작 시간을 읽어 duration_ms를 계산한다.

## 완료 기준

- [ ] `adapter/timing.rs`에 시작 시간 저장/조회 함수 (임시 파일 기반)
- [ ] `entrypoint/hooks/pre_tool.rs`에 PreToolUse 훅 핸들러
- [ ] `seogi hook pre-tool` 서브커맨드 동작
- [ ] Feature 02의 `workflow/log_tool.rs`가 시작 시간을 읽어 `duration_ms` 계산하도록 수정됨
- [ ] pre-tool → post-tool 순서로 호출 시 duration_ms가 올바르게 계산되는 통합 테스트
- [ ] pre-tool만 호출된 경우 (post-tool 없이) 시작 시간 파일이 정리되지 않아도 문제없음 확인
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-05-pre-tool-hook.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. timing adapter 구현
4. pre-tool 훅 entrypoint 구현
5. post-tool workflow에 duration 계산 연결
6. 통합 테스트 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 5 섹션
- 기존 bash `hooks/pre-tool.sh` — 현재 동작 참조

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 02 (post-tool 훅에 duration 연결)

## 워크트리

```bash
git worktree add -b feature/05-pre-tool-hook .worktrees/05-pre-tool-hook origin/main
```
