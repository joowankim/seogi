# Feature 15: Task 업데이트

## 작업 목표

`seogi task update` 명령어를 구현한다. 태스크의 title, description, label을 수정할 수 있다.

## 완료 기준

- [ ] `adapter/task_repo.rs`에 `update` 함수 추가
- [ ] `workflow/task.rs`에 `update_task` workflow (태스크 존재 확인 → 필드 업데이트 → `updated_at` 갱신)
- [ ] `seogi task update SEO-1 [--title "..."] [--description "..."] [--label bug]` 구현
- [ ] 존재하지 않는 태스크 ID 시 에러
- [ ] 아무 옵션도 없으면 에러 또는 안내 메시지
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-15-task-update.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. adapter 함수 구현 (update)
4. workflow 함수 구현 (update_task)
5. entrypoint 연결 (clap 서브커맨드)
6. 테스트 작성 및 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-task-management.md` — 2단계 섹션 (결정 사항 포함)
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

- Feature 14 완료 필수

## 워크트리

```bash
git worktree add -b feature/15-task-update .worktrees/15-task-update origin/main
```
