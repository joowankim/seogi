# Feature 10: install.sh 업데이트 + 레거시 제거

## 작업 목표

install.sh/uninstall.sh를 업데이트하여 Rust 훅 명령어를 등록하고, bash 훅과 레거시 파일을 제거한다.

## 완료 기준

- [ ] `install.sh`가 `~/.claude/settings.json`에 `seogi hook <name>` 명령어 등록
- [ ] bash 훅 복사 로직 제거 (`hooks/*.sh` 배포 중단)
- [ ] `uninstall.sh`에서 `seogi hook` 명령어 정리
- [ ] 레거시 파일 삭제: `hooks/*.sh`, `lib/logger.sh`, `config.json` (루트)
- [ ] uninstall → install 사이클 정상 동작
- [ ] 설치 후 Claude Code에서 실제 도구 호출 → DB에 저장 확인
- [ ] `seogi report` 정상 동작 확인
- [ ] DB 접근 실패 시 `~/.seogi/hook-errors.log`에 기록 + macOS 알림 (5분 쿨다운) 동작 확인

## 작업 목록

1. 기획 문서 작성 (`docs/features/feature-10-install-cleanup.md`) + QA 목록
2. 사용자 승인 대기
3. install.sh 업데이트 (훅 등록 형식 변경)
4. uninstall.sh 업데이트
5. 레거시 bash 훅/설정 파일 삭제
6. DB 에러 처리 (hook-errors.log + 알림 쿨다운) 구현
7. 실제 환경 검증 (uninstall → install → Claude Code 세션)

## 참조 문서

- `CLAUDE.md` — 프로젝트 규칙 (반드시 먼저 읽을 것)
- `docs/plans/2026-04-15-phase1-sqlite-migration.md` — Feature 10 섹션, 논의 3 (훅 에러 처리)
- 현재 `install.sh`, `uninstall.sh` — 기존 로직 참조

## 의존성

- Feature 01~09 모두 완료 필수 (전체 훅/CLI가 Rust로 전환된 상태여야 함)

## 워크트리

```bash
git worktree add -b feature/10-install-cleanup .worktrees/10-install-cleanup origin/main
```
