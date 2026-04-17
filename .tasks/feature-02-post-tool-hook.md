# Feature 02: 도구 사용 로깅 (`seogi hook post-tool`)

## 작업 목표

Claude Code의 PostToolUse 훅을 Rust로 구현하여, 도구가 성공적으로 호출됐을 때 SQLite에 기록한다.

## 완료 기준

- [ ] `domain/log.rs`에 `ToolUse` 타입 정의 (Newtype 패턴, 필수 derive)
- [ ] `adapter/log_repo.rs`에 `save_tool_use`, `find_by_session` 함수 구현
- [ ] `adapter/mapper.rs`에 `ToolUse` ↔ Row 변환 함수
- [ ] `workflow/log_tool.rs`에 Impureim Sandwich workflow 함수
- [ ] `entrypoint/hooks/post_tool.rs`에 stdin 파싱 → workflow 호출
- [ ] `seogi hook post-tool` 서브커맨드가 stdin으로 JSON을 받아 DB에 저장
- [ ] 단위 테스트: ToolUse 생성/검증
- [ ] 통합 테스트: adapter 함수 (인메모리 SQLite)
- [ ] 통합 테스트: workflow 함수 (실제 adapter + domain)
- [ ] E2E 테스트: 바이너리 stdin → DB 저장 확인
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-02-post-tool-hook.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. E2E 테스트 선작성 (RED)
4. domain 타입 + 단위 테스트 구현
5. adapter 함수 + 통합 테스트 구현
6. workflow 함수 + 통합 테스트 구현
7. entrypoint 훅 연결
8. E2E 테스트 녹색 확인 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 2 섹션
- `docs/plans/2026-04-15-task-management.md` — tool_uses 테이블 스키마
- `docs/conventions.md` — 코딩 컨벤션 (Impureim Sandwich, Dependency Rejection)

## 의존성

- Feature 01 (DB 초기화) 완료 필수

## 워크트리

```bash
git worktree add -b feature/02-post-tool-hook .worktrees/02-post-tool-hook origin/main
```
