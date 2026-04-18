# Feature 14: Task 생성/조회

## 작업 목표

`seogi task create/list` 명령어를 구현한다. 태스크 ID는 `{prefix}-{seq}` 형식으로 자동 생성하고, 초기 상태는 backlog로 설정한다.

## 완료 기준

- [ ] `domain/task.rs`에 `Task` 타입, `Label` enum (feature, bug, refactor, chore, docs)
- [ ] `adapter/task_repo.rs`에 `save`, `find_all` 함수 (필터링: project, status, label)
- [ ] `workflow/task.rs`에 `create_task` workflow
- [ ] 프로젝트 존재 확인 → `next_seq` 원자적 채번 → 초기 상태 backlog 할당 → 저장 + `task_events` 기록
- [ ] `seogi task create --project "Seogi" --title "..." --description "..." --label feature` 구현
- [ ] `--project`는 프로젝트 이름으로 조회
- [ ] 태스크 ID는 `{prefix}-{seq}` 형식 (예: SEO-1, SEO-2)
- [ ] `seogi task list [--project "..."] [--status <status>] [--label <label>]` 구현 (테이블 출력 + `--json` 플래그)
- [ ] `task_events`에 초기 생성 이벤트 기록 (`from_status: NULL`, `to_status: backlog`, `session_id: CLI_SESSION_ID`)
- [ ] `CLI_SESSION_ID` 도메인 상수 선언
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-14-task-create-list.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. domain 타입 구현 (Task, Label, CLI_SESSION_ID 상수)
4. adapter 함수 구현 (task_repo, task_event_repo)
5. workflow 함수 구현 (create_task)
6. entrypoint 연결 (clap 서브커맨드)
7. 테스트 작성 및 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-task-management.md` — 2단계 섹션 (결정 사항 포함)
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

- Feature 12, 13 완료 필수

## 워크트리

```bash
git worktree add -b feature/14-task-create-list .worktrees/14-task-create-list origin/main
```
