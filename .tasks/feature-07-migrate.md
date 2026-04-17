# Feature 07: 마이그레이션 (`seogi migrate`)

## 작업 목표

기존 `~/seogi-logs/` 디렉토리의 JSONL 로그/메트릭 파일을 SQLite로 마이그레이션한다. pretty-printed와 compact JSON 형식을 모두 지원하며, 재실행 시 중복이 발생하지 않아야 한다.

## 완료 기준

- [ ] `adapter/jsonl_reader.rs`에 기존 JSONL 파서 구현 (pretty-printed + compact 호환)
- [ ] `workflow/migrate.rs`에 JSONL → 도메인 타입 → DB 저장 workflow
- [ ] `entrypoint/cli/migrate.rs`에 `seogi migrate` 서브커맨드
- [ ] 컨텐츠 기반 id 생성으로 재실행 시 중복 방지 (`INSERT OR IGNORE`)
- [ ] 파싱 실패 엔트리는 건너뛰고 경고 출력
- [ ] 원본 JSONL 파일은 보존 (삭제하지 않음)
- [ ] 샘플 JSONL → DB 변환 테스트
- [ ] 재실행 시 중복 없음 테스트
- [ ] 실제 `~/seogi-logs/` 데이터로 검증
- [ ] `cargo test` 전체 통과 + prek 훅 통과
- [ ] 브랜치 커버리지 100%

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-07-migrate.md`) + QA 목록 + Test Pyramid 설계
2. 사용자 승인 대기
3. JSONL 파서 구현 (기존 `cli/src/log_reader.rs` 로직 재사용)
4. 마이그레이션 workflow 구현
5. CLI 서브커맨드 연결
6. 실제 데이터 검증
7. 테스트 녹색 + 커버리지 달성

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 7 섹션, 리스크 2 (마이그레이션 중복), 리스크 3 (혼합 JSONL)
- 기존 `cli/src/log_reader.rs` — 현재 JSONL 파서 코드

## 의존성

- Feature 01 (DB 초기화) 완료 필수
- Feature 02, 03, 04 (저장 대상 테이블이 존재해야 함)

## 워크트리

```bash
git worktree add -b feature/07-migrate .worktrees/07-migrate origin/main
```
