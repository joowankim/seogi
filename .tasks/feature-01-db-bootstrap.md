# Feature 01: 프로젝트 부트스트랩 + DB 초기화

## 작업 목표

기존 Rust CLI 프로젝트를 함수형 3계층(entrypoint/workflow/domain+adapter) 구조로 리팩토링하고, SQLite 연결과 스키마 초기화를 구현한다.

## 완료 기준

- [ ] Cargo.toml에 rusqlite, uuid, thiserror 의존성 추가됨
- [ ] 기존 코드가 함수형 3계층 디렉토리 구조로 재배치됨 (`domain/`, `adapter/`, `workflow/`, `entrypoint/`)
- [ ] `adapter/db.rs`에서 `~/.seogi/seogi.db` 파일을 자동 생성하고 전체 스키마를 적용함
- [ ] `domain/error.rs`에 `DomainError`가 thiserror로 정의됨
- [ ] 빈 DB 초기화 → 스키마 존재 확인 테스트 통과
- [ ] 재실행 시 기존 스키마 유지 테스트 통과
- [ ] 인메모리 DB (`:memory:`)로 테스트 격리됨
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-01-db-bootstrap.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. 기존 코드를 함수형 3계층 구조로 리팩토링
4. SQLite 연결 관리 및 스키마 초기화 구현
5. DomainError 정의
6. 테스트 작성 및 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — 전체 계획 (Feature 1 섹션)
- `docs/plans/2026-04-15-task-management.md` — DB 스키마 정의
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

없음 (첫 번째 Feature)

## 워크트리

```bash
git worktree add -b feature/01-db-bootstrap .worktrees/01-db-bootstrap origin/main
```
