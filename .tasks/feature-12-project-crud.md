# Feature 12: Project CRUD

## 작업 목표

`seogi project create/list` 명령어를 구현한다. 프로젝트 이름, 대문자 3글자 prefix, 목표를 관리한다.

## 완료 기준

- [ ] `domain/project.rs`에 `Project` 타입, `Prefix` newtype (대문자 알파벳 3글자 검증)
- [ ] `adapter/project_repo.rs`에 `save`, `find_all` 함수
- [ ] `workflow/project.rs`에 `create_project` workflow (prefix 중복 검증 → 저장)
- [ ] `seogi project create --name "..." --prefix "SEO" --goal "..."` 구현
- [ ] `--prefix` 미지정 시 이름 앞 3글자 대문자를 기본값으로 사용
- [ ] `seogi project list` 구현 (테이블 출력 + `--json` 플래그)
- [ ] `project.id`는 UUID hex, `next_seq` 초기값은 도메인에서 1로 설정
- [ ] Prefix가 대문자 알파벳 3글자가 아니면 에러
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-12-project-crud.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. domain 타입 구현 (Project, Prefix)
4. adapter 함수 구현 (project_repo)
5. workflow 함수 구현 (create_project)
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
git worktree add -b feature/12-project-crud .worktrees/12-project-crud origin/main
```
