# Feature 09: Changelog (`seogi changelog add`)

## 작업 목표

하니스 변경 이력을 SQLite에 기록한다. 기존 JSONL 기반 changelog를 SQLite 기반으로 재구현한다.

## 완료 기준

- [ ] `domain/changelog.rs`에 `ChangelogEntry` 타입
- [ ] `adapter/changelog_repo.rs`에 `save` 함수
- [ ] `workflow/changelog.rs`에 이력 추가 workflow
- [ ] `entrypoint/cli/changelog.rs`에 `seogi changelog add <description>` 서브커맨드
- [ ] 타임스탬프 자동 생성 + 기록 확인 메시지 출력
- [ ] 단순 추가 테스트
- [ ] append-only 이력 (중복 방지 불필요)
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-09-changelog.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. domain 타입 + adapter 함수 구현
4. workflow + entrypoint 연결
5. 테스트 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 9 섹션
- 기존 `cli/src/commands/changelog.rs` — 현재 changelog 로직

## 의존성

- Feature 01 (DB 초기화) 완료 필수

## 워크트리

```bash
git worktree add -b feature/09-changelog .worktrees/09-changelog origin/main
```
