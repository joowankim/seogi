# Feature 16: FSM + Task 상태 전환

## 작업 목표

`seogi task move` 명령어를 구현한다. 카테고리 간 상태 전환 규칙(FSM)을 검증하고, 전환 이력을 `task_events`에 기록한다.

## 완료 기준

- [ ] `domain/status.rs`에 FSM 검증 함수 (카테고리 간 허용된 전환인지 확인)
- [ ] 전환 규칙: Backlog→{Unstarted,Canceled}, Unstarted→{Started,Backlog,Canceled}, Started→{Completed,Canceled,Unstarted}, Completed→Started, Canceled→Backlog
- [ ] 같은 카테고리 내 커스텀 상태 간 자유 전환 허용
- [ ] `adapter/task_event_repo.rs`에 `save` 함수
- [ ] `workflow/task.rs`에 `move_task` workflow (태스크 조회 → FSM 검증 → status_id 변경 → task_event 기록)
- [ ] `seogi task move SEO-1 in_review` 구현
- [ ] 허용되지 않은 전환 시 에러 메시지 (현재 상태, 목표 상태, 허용 가능한 전환 목록 표시)
- [ ] `task_events`에 `from_status`, `to_status`, `session_id` (CLI에서는 도메인 상수 `CLI_SESSION_ID`) 기록
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-16-task-fsm.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. domain FSM 검증 함수 구현
4. adapter 함수 구현 (task_event_repo)
5. workflow 함수 구현 (move_task)
6. entrypoint 연결 (clap 서브커맨드)
7. 테스트 작성 및 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-task-management.md` — 2단계 섹션 (결정 사항 포함, 전환 규칙 포함)
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

- Feature 14 완료 필수

## 워크트리

```bash
git worktree add -b feature/16-task-fsm .worktrees/16-task-fsm origin/main
```
