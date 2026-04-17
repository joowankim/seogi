# Feature 04: 시스템 이벤트 로깅 (`seogi hook notification`, `seogi hook stop`)

## 작업 목표

Claude Code의 Notification과 Stop 훅을 Rust로 구현하여, 알림과 세션 종료를 SQLite에 기록한다.

## 완료 기준

- [ ] `domain/log.rs`에 `SystemEvent` 타입 추가
- [ ] `adapter/log_repo.rs`에 `save_system_event` 함수 추가
- [ ] `workflow/log_system.rs`에 workflow 함수
- [ ] `entrypoint/hooks/notification.rs`에 Notification 훅 핸들러
- [ ] `entrypoint/hooks/stop.rs`에 Stop 훅 핸들러
- [ ] `seogi hook notification`, `seogi hook stop` 서브커맨드 동작
- [ ] stop 훅에서 `seogi analyze`를 백그라운드로 호출하는 코드 자리 마련 (Feature 6 이후 연결)
- [ ] Feature 2와 동일 패턴의 단위/통합/E2E 테스트
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-04-system-event-hooks.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. E2E → domain → adapter → workflow → entrypoint 구현
4. stop 훅의 분석기 호출 플레이스홀더 구현
5. 테스트 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 4 섹션
- `docs/plans/2026-04-15-task-management.md` — system_events 테이블 스키마

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 02 (패턴 참조)

## 워크트리

```bash
git worktree add -b feature/04-system-event-hooks .worktrees/04-system-event-hooks origin/main
```
