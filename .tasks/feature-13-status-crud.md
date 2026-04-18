# Feature 13: Status CRUD

## 작업 목표

`seogi status create/update/delete/list` 명령어를 구현한다. 기본 상태와 사용자 커스텀 상태를 동일하게 관리한다.

## 완료 기준

- [ ] `domain/status.rs`에 `Status` 타입 (Feature 11의 `StatusCategory` enum 활용)
- [ ] `adapter/status_repo.rs`에 `save`, `find_all`, `update`, `delete` 함수
- [ ] `workflow/status.rs`에 `create_status`, `update_status`, `delete_status` workflow
- [ ] `seogi status create --category started --name "testing"` 구현
- [ ] `seogi status update <id> --name "..."` 구현
- [ ] `seogi status delete <id>` 구현
- [ ] `seogi status list` 구현 (테이블 출력 + `--json` 플래그)
- [ ] 카테고리 유효성 검증 (`StatusCategory` enum 기반)
- [ ] position 자동 부여 (해당 카테고리 내 max + 1)
- [ ] 기본 상태와 커스텀 상태 동일하게 수정/삭제 가능
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-13-status-crud.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. domain 타입 구현 (Status)
4. adapter 함수 구현 (status_repo)
5. workflow 함수 구현 (create/update/delete_status)
6. entrypoint 연결 (clap 서브커맨드)
7. 테스트 작성 및 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-task-management.md` — 2단계 섹션 (결정 사항 포함)
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

- Feature 11 완료 필수

## 워크트리

```bash
git worktree add -b feature/13-status-crud .worktrees/13-status-crud origin/main
```
