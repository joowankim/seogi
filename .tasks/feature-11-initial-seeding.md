# Feature 11: 초기 데이터 시딩 + 스키마 변경

## 작업 목표

`status_categories` 테이블을 제거하고 코드 enum으로 대체한다. `statuses` 테이블의 `category_id`를 `category TEXT`로 변경하고, 기본 statuses 7개를 스키마 적용 시 자동 삽입한다. `projects` 테이블에 `next_seq` 컬럼을 추가한다.

## 완료 기준

- [ ] `domain/status.rs`에 `StatusCategory` enum 선언 (Backlog, Unstarted, Started, Completed, Canceled)
- [ ] `status_categories` 테이블이 스키마에서 제거됨
- [ ] `statuses` 테이블의 `category_id` → `category TEXT`로 변경됨
- [ ] `projects` 테이블에 `next_seq INTEGER NOT NULL` 컬럼 추가됨
- [ ] 스키마 적용 시 기본 statuses 7개 자동 삽입 (backlog, todo, in_progress, in_review, blocked, done, canceled)
- [ ] 기존 `status_categories` 테이블 DROP 처리
- [ ] `SCHEMA_VERSION` 업그레이드
- [ ] 기존 테스트 업데이트 (테이블 목록, 컬럼 스펙 등)
- [ ] `cargo test` 전체 통과 + prek pre-commit 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-11-initial-seeding.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. `StatusCategory` enum 구현
4. DB 스키마 변경 (테이블 DROP, 컬럼 변경, next_seq 추가)
5. 기본 statuses 시딩 로직 구현
6. 기존 테스트 수정 + 신규 테스트 작성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-task-management.md` — 2단계 섹션 (결정 사항 포함)
- `docs/conventions.md` — 코딩 컨벤션
- `docs/feature-planning.md` — 기획 가이드
- `docs/tdd-cycle.md` — TDD 구현 가이드

## 의존성

없음 (Phase 2 첫 번째 Feature)

## 워크트리

```bash
git worktree add -b feature/11-initial-seeding .worktrees/11-initial-seeding origin/main
```
