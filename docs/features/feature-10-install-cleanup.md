# Feature 10: install.sh 업데이트 + 레거시 제거

상위 문서: [Phase 1 구현 계획](../plans/2026-04-15-phase1-sqlite-migration.md)

---

## 목적

bash 훅에서 Rust CLI로 전환을 완료한다. install.sh가 `seogi hook <name>` 명령어를 등록하고, 레거시 bash 훅/설정 파일을 제거한다. 훅 에러 시 hook-errors.log에 기록하고 macOS 알림을 보낸다.

**Ground Truth 연결:**
- **정량 측정**: Rust 훅이 올바르게 등록되어야 데이터 수집이 작동한다. 설치 스크립트는 데이터 파이프라인의 진입점
- **동치 보장**: 에러 알림으로 데이터 수집 실패를 신속히 감지하여 데이터 공백을 방지

---

## 범위

### A. install.sh 업데이트
- bash 훅 복사 로직 제거
- `seogi hook <name>` 명령어를 settings.json에 등록
- config.json 복사 제거
- ~/seogi-logs 디렉토리 생성 제거

### B. uninstall.sh 업데이트
- `seogi hook` 명령어 패턴으로 필터링
- ~/.seogi/seogi.db 보존 안내 메시지

### C. 레거시 파일 삭제
- `hooks/*.sh` (5개)
- `lib/logger.sh`
- `config.json` (루트)

### D. 훅 에러 처리 (Rust)
- DB 접근 실패 시 `~/.seogi/hook-errors.log`에 에러 기록
- macOS 알림 (5분 쿨다운)
- 훅은 exit 0으로 종료 (세션 계속)

### E. 레거시 Rust 코드 정리
- `cli/src/commands/` (analyze.rs, changelog.rs, report.rs) — JSONL 기반 코드 제거
- `cli/src/log_reader.rs`, `cli/src/metrics_reader.rs`, `cli/src/models.rs` — migrate에서만 사용하므로 유지 또는 정리
- `cli/src/analyzers/session_summary.rs` — JSONL 기반, 제거 대상
- `cli/src/config.rs` — migrate에서만 사용, 유지

---

## 성공 시나리오

### install
1. `./install.sh`를 실행한다.
2. `cargo install --path ./cli`로 seogi CLI를 설치한다.
3. `~/.seogi/` 디렉토리를 생성한다.
4. `~/.claude/settings.json`에 5개 훅을 등록한다:
   - `PreToolUse`: `seogi hook pre-tool`
   - `PostToolUse`: `seogi hook post-tool`
   - `PostToolUseFailure`: `seogi hook post-tool-failure`
   - `Notification`: `seogi hook notification`
   - `Stop`: `seogi hook stop`
5. 성공 메시지를 출력한다.

### uninstall
1. `./uninstall.sh`를 실행한다.
2. settings.json에서 `seogi hook` 포함 훅을 제거한다.
3. `~/.seogi/` 디렉토리를 제거한다 (seogi.db 포함).
4. 보존 안내 메시지를 출력한다.

### 에러 처리
1. 훅 실행 중 DB 접근 실패가 발생한다.
2. `~/.seogi/hook-errors.log`에 타임스탬프 + 에러 메시지를 append한다.
3. `~/.seogi/last-notification` 파일을 확인한다.
4. 마지막 알림으로부터 5분 이상 경과했으면 macOS 알림을 보내고 타임스탬프를 갱신한다.
5. 훅은 exit 0으로 종료한다 (Claude Code 세션 계속).

---

## 실패 시나리오

| 조건 | 처리 |
|------|------|
| cargo 미설치 | 경고 메시지 출력 후 계속 |
| jq 미설치 | install.sh 실패 + 안내 메시지 |
| settings.json 없음 | 새로 생성 |
| settings.json이 유효하지 않은 JSON | install.sh 실패 + 백업 안내 |

---

## 제약 조건

- **호환성**: 기존 settings.json의 다른 훅 설정을 보존
- **멱등성**: install.sh를 여러 번 실행해도 중복 등록되지 않아야 함
- **플랫폼**: macOS 알림은 `osascript` 사용. Linux 미지원 시 알림만 건너뜀

---

## 의존 Feature

- **Feature 01~09 전체**: 모든 훅/CLI가 Rust로 전환된 상태

---

## QA 목록

### install.sh

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q1 | install.sh 실행 후 settings.json에 `seogi hook pre-tool` 명령이 등록된다 | 수동: jq로 확인 |
| Q2 | install.sh 실행 후 settings.json에 5개 훅 모두 등록된다 | 수동: jq로 확인 |
| Q3 | install.sh를 2회 실행해도 중복 등록되지 않는다 | 수동: 훅 수 확인 |
| Q4 | 기존 settings.json의 다른 설정이 보존된다 | 수동: 다른 키 확인 |

### uninstall.sh

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q5 | uninstall.sh 실행 후 settings.json에서 `seogi hook` 훅이 제거된다 | 수동: jq로 확인 |
| Q6 | uninstall.sh 실행 후 ~/.seogi/ 디렉토리가 삭제된다 | 수동: ls 확인 |

### 레거시 삭제

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q7 | `hooks/*.sh`, `lib/logger.sh`, `config.json`이 리포지토리에서 삭제된다 | git status |

### 에러 처리 (Rust)

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q8 | DB 접근 불가 시 `~/.seogi/hook-errors.log`에 에러가 기록된다 | 통합: 잘못된 DB 경로 → 파일 확인 |
| Q9 | 에러 기록 후 5분 이상 경과 시 macOS 알림이 호출된다 | 통합: last-notification 파일 확인 |
| Q10 | 에러 발생해도 훅은 exit 0으로 종료한다 | E2E: 잘못된 DB → exit 0 |
| Q11 | 5분 이내 재에러 시 알림이 중복 발생하지 않는다 | 통합: last-notification 타임스탬프 확인 |

### 레거시 코드 정리

| # | QA 항목 | 검증 방법 |
|---|---------|-----------|
| Q12 | `commands/analyze.rs`, `commands/changelog.rs`, `commands/report.rs`, `analyzers/session_summary.rs` 삭제 후 `cargo test` 통과 | cargo test |
| Q13 | `log_reader.rs`는 migrate에서 사용하므로 유지 확인 | grep 확인 |

---

## Test Pyramid

### Unit/Integration Tests (Rust — 에러 처리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_log_hook_error` | Q8 | 에러 → hook-errors.log 확인 |
| `test_notification_cooldown` | Q9, Q11 | 5분 쿨다운 확인 |

### E2E Tests (Rust — 에러 처리)

| 테스트 | QA | 설명 |
|--------|-----|------|
| `test_hook_error_exits_zero` | Q10 | 잘못된 DB → exit 0 |

### 수동 검증 (install/uninstall — 쉘 스크립트)

| 검증 | QA | 설명 |
|--------|-----|------|
| uninstall → install → settings.json 확인 | Q1~Q6 | 쉘 스크립트 통합 |
| git status | Q7 | 레거시 파일 삭제 확인 |
| cargo test | Q12, Q13 | 레거시 코드 삭제 후 회귀 |

---

## 체크리스트

- [x] Feature 문서 작성 완료
- [x] QA 항목이 성공/실패 시나리오를 모두 커버
- [x] 각 QA 항목이 Test Pyramid의 테스트에 매핑됨
- [x] 의존하는 Feature 순서 명확
- [x] `/seogi-planning-review` 통과 (인프라 Feature, 리뷰 간소화)
- [ ] 사용자 승인 완료
