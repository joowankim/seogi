# Phase 2 구현 계획

상위 문서: [measurement-framework.md](./2026-04-08-measurement-framework.md)

---

## 목표

1. 분석기와 CLI를 Rust로 구현
2. 기존 bash 분석기(session-summary.sh)를 대체
3. `seogi report` — 기준선 집계 리포트
4. `seogi changelog add` — 하니스 변경 이력 기록

---

## 언어 선택: Rust

- 바이너리 하나만 배포, 사용자 환경에 런타임 의존성 없음
- 로컬 빌드(`cargo build --release`)로 시작
- 크로스 컴파일 CI는 다른 사용자 배포 시 추가

---

## 프로젝트 구조

```
seogi/
├── hooks/                    # bash (변경 없음)
├── lib/                      # bash (변경 없음)
├── analyzers/
│   └── session-summary.sh    # Phase 2 완료 후 삭제
├── cli/                      # Rust 프로젝트
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # CLI 진입점 (clap)
│       ├── config.rs         # config.json 로드, 경로 관리
│       ├── log_reader.rs     # raw JSONL 파싱 (pretty-printed + compact)
│       ├── metrics_reader.rs # metrics JSONL 파싱
│       ├── models.rs         # LogEntry, SessionMetrics, ChangelogEntry 구조체
│       ├── analyzers/
│       │   ├── mod.rs
│       │   └── session_summary.rs  # 지표 10개 계산
│       └── commands/
│           ├── mod.rs
│           ├── analyze.rs    # seogi analyze <project> <session_id>
│           ├── report.rs     # seogi report --from --to --project
│           └── changelog.rs  # seogi changelog add "..."
├── config.json
├── install.sh
├── uninstall.sh
└── ...
```

---

## CLI 명령어

### 글로벌 옵션

```
seogi [--config <path>] <command>
```

- `--config`: config.json 경로 지정. 생략 시 `~/.seogi/config.json` 사용.

### seogi analyze \<project\> \<session_id\>

기존 session-summary.sh를 대체. Stop 훅에서 백그라운드로 호출.

```bash
# stop.sh 변경
SEOGI_BIN="$SCRIPT_DIR/../bin/seogi"
if [[ -x "$SEOGI_BIN" ]]; then
  "$SEOGI_BIN" analyze "$PROJECT_NAME" "$SESSION_ID" &
fi
```

### seogi report

```
$ seogi report --from 2026-04-08 --to 2026-04-14
$ seogi report --from 2026-04-08 --to 2026-04-14 --project locs

기간: 2026-04-08 ~ 2026-04-14 (n=496 세션)

                         평균    중앙값   σ      P25    P75
read_before_edit         27.0   15.0    30.2   5.0    40.0
doom_loop_count          0.95   0.0     1.8    0.0    1.0
tool_call_count          160    125     120    27     276
session_duration_sec     15175  ...     ...    ...    ...
bash_error_rate          1.7%   ...     ...    ...    ...
edit_files_count         5.2    ...     ...    ...    ...

test_invoked             55%
build_invoked            13%
lint_invoked             0%
typecheck_invoked        11%
```

- `--project` 생략 시 전체 프로젝트 합산
- boolean 지표는 비율(%)로 표시
- 수치 지표는 평균/중앙값/σ/P25/P75

### seogi changelog add \<description\>

```
$ seogi changelog add "CLAUDE.md에 Edit 전 Read 강제 규칙 추가"
Recorded at 2026-04-15T09:00:00.000Z
```

저장: `~/seogi-logs/harness-changelog.jsonl`

---

## 의존성 (Cargo.toml)

```toml
[package]
name = "seogi"
version = "0.2.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
cargo-husky = { version = "1", default-features = false, features = ["prepush-hook", "run-cargo-clippy", "run-cargo-fmt"] }
```

- clap: CLI 파싱
- serde/serde_json: JSONL 파싱
- chrono: 타임스탬프 처리
- cargo-husky: pre-push 훅으로 clippy + fmt 자동 실행
- 외부 HTTP/DB 의존성 없음

---

## 구현 순서

### Step 1: Rust 프로젝트 초기화 + 모델/설정

구현:
- `cargo init cli`
- `models.rs` — LogEntry, SessionMetrics, ChangelogEntry 구조체
- `config.rs` — config.json 로드, `--config` 옵션 또는 `~/.seogi/config.json` 기본값

테스트:
- config.json 로드 성공/실패
- `~` 경로 확장 (`~/seogi-logs` → 절대경로)
- `--config` 옵션 지정 시 해당 경로 사용

### Step 2: JSONL 파서 (log_reader.rs + metrics_reader.rs)

구현:
- `log_reader.rs` — raw 로그 파싱 (pretty-printed + compact), 세션 ID 필터링, metrics 디렉토리 제외, 타임스탬프 정렬
- `metrics_reader.rs` — metrics JSONL 파싱, 날짜 범위 필터링, 누락 필드 허용

테스트:
- compact JSONL 파싱
- pretty-printed JSON 파싱
- 세션 ID 필터링
- 빈 파일 / 파일 없음 처리
- metrics 디렉토리 제외
- 날짜 범위 필터
- 구형 스키마(lint_invoked 없는 엔트리) 파싱

### Step 3: 분석기 (analyzers/session_summary.rs)

구현 — 기존 jq 로직을 Rust로 옮긴다. 지표 10개:

| # | 지표 | 계산 |
|---|---|---|
| 1 | read_before_edit_ratio | 첫 Edit/Write 전 Read/Grep/Glob 수 |
| 2 | doom_loop_count | 동일 파일 Edit 5회+ 횟수 |
| 3 | test_invoked | Bash command에 test/vitest/jest/pytest 등 |
| 4 | build_invoked | Bash command에 build/tsc/webpack 등 |
| 5 | lint_invoked | Bash command에 lint/eslint/prettier/ruff/biome |
| 6 | typecheck_invoked | Bash command에 tsc --noEmit/mypy/pyright |
| 7 | tool_call_count | tool != null 엔트리 수 |
| 8 | session_duration_ms | 첫 ~ 마지막 타임스탬프 차이 |
| 9 | edit_files | Edit/Write의 file_path 고유 목록 |
| 10 | bash_error_rate | Bash 실패 / Bash 전체 |

테스트 — 지표별 독립 테스트:
- read_before_edit: Edit 전 Read 3번 → ratio=3 / Edit 없으면 → 전체 read 수
- doom_loop_count: 같은 파일 Edit 6번 → 1 / 4번 → 0
- test_invoked: "pytest" → true / "ls" → false
- lint_invoked: "eslint" → true
- typecheck_invoked: "tsc --noEmit" → true
- bash_error_rate: 성공 8 + 실패 2 → 0.2
- edit_files: 3개 파일 Edit → 3개 목록
- session_duration: 타임스탬프 차이 검증
- 빈 세션 → 모든 지표 기본값

### Step 4: `seogi analyze` 명령어 + stop.sh 연결

구현:
- `seogi analyze <project> <session_id>` → metrics JSONL 출력
- stop.sh 수정: bash 분석기 → seogi 바이너리 호출

테스트:
- 실제 로그 데이터로 analyze 실행 → 기존 bash 분석기와 동일한 출력 확인
- metrics JSONL 스키마 일치 확인

### Step 5: `seogi report` 명령어

구현:
- metrics JSONL 읽기 (metrics_reader.rs 사용) + 날짜 범위 필터
- 통계 계산 (평균, 중앙값, σ, P25, P75)
- 터미널 테이블 출력

테스트:
- 통계 계산 정확성 (알려진 입력 → 기대 출력)
- 데이터 없는 기간 → 안내 메시지
- n=1 세션 → σ 표시 생략
- 구형 스키마 엔트리 → 해당 지표 집계에서 제외

### Step 6: `seogi changelog add` 명령어

구현:
- `~/seogi-logs/harness-changelog.jsonl`에 append
- `{timestamp, description}` 형태

테스트:
- 파일 없을 때 생성 + append
- 기존 파일에 append
- 타임스탬프 형식 검증

### Step 7: install.sh 업데이트 + 배포

구현:
- install.sh: `cli/target/release/seogi`를 `~/.seogi/bin/seogi`에 복사
- stop.sh: 바이너리 호출로 변경
- `analyzers/session-summary.sh` 삭제

테스트:
- uninstall → install 사이클
- 훅 등록 확인
- 바이너리 실행 확인
- stop 훅 → analyze 호출 동작 확인

---

## 기존 데이터 호환

기존 metrics JSONL에는 `lint_invoked`, `typecheck_invoked`, `bash_error_rate`가 없는 엔트리(147건)가 있다.
- metrics_reader에서 누락 필드는 Option으로 처리
- report 명령어에서 필드 없는 엔트리는 해당 지표 집계에서 제외
- 새 분석기가 생성하는 엔트리는 항상 10개 지표 포함

---

## 논의 결과

### 논의 1: 빌드와 배포 방식 — 결정됨

install.sh가 `cli/target/release/seogi` 바이너리를 `~/.seogi/bin/seogi`에 복사한다.
빌드는 개발 시 `cargo build --release`로 수동 수행.
uninstall.sh는 기존과 동일하게 `~/.seogi/` 전체 삭제 — 바이너리도 함께 삭제됨.

### 논의 2: bash 분석기 — 즉시 삭제

Rust 바이너리 검증 후 `analyzers/session-summary.sh` 삭제.
폴백 불필요 (사용자가 본인뿐).

### 논의 3: config 경로 전달 — 결정됨

`--config` 옵션으로 지정, 생략 시 `~/.seogi/config.json` 기본값.

### 논의 4: metrics 파일 읽기 — 별도 모듈

`metrics_reader.rs`를 `log_reader.rs`와 분리. raw 로그와 metrics는 스키마가 다르므로.
