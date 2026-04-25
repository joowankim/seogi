# Phase 1 구현 계획: SQLite 마이그레이션 + 훅 Rust 전환

상위 문서: [task-management.md](./2026-04-15-task-management.md)

---

## 목표

1. 저장소를 JSONL에서 SQLite 단일 파일로 통합
2. bash 훅 5개를 Rust로 전환 (`seogi hook <name>`)
3. 기존 세션 로그/메트릭을 SQLite로 마이그레이션
4. 코드를 DDD + ROP 구조로 리팩토링

이 단계가 완료되면 태스크 관리(2단계)를 저장소 위에서 바로 올릴 수 있다.

---

## 의존성 추가 (Cargo.toml)

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
regex = "1"
rusqlite = { version = "0.32", features = ["bundled"] }  # 신규
uuid = { version = "1", features = ["v4"] }              # 신규
thiserror = "1"                                          # 신규 (도메인 에러용)
```

- `rusqlite` + `bundled`: SQLite를 바이너리에 포함, 외부 의존성 없음
- `uuid`: UUID v4 hex 생성
- `thiserror`: 도메인 레이어의 에러 타입 정의

---

## 코드 구조 (DDD + ROP)

```
app/
├── src/
│   ├── main.rs                        # 진입점 (clap 파싱)
│   ├── lib.rs                         # 모듈 선언
│   ├── domain/                        # 순수 데이터 + 순수 함수
│   │   ├── mod.rs
│   │   ├── log.rs                     # ToolUse, ToolFailure, SystemEvent 타입
│   │   ├── metrics.rs                 # SessionMetrics 타입 + calculate 함수
│   │   ├── command.rs                 # Command DTO
│   │   ├── query.rs                   # Query DTO
│   │   └── error.rs                   # DomainError (thiserror)
│   ├── adapter/                       # I/O 함수 (DB 액세스)
│   │   ├── mod.rs
│   │   ├── db.rs                      # Connection 관리, 스키마 초기화
│   │   ├── log_repo.rs                # 로그 저장/조회 함수
│   │   ├── metrics_repo.rs            # 메트릭 저장/조회 함수
│   │   ├── changelog_repo.rs
│   │   └── mapper.rs                  # 도메인 타입 ↔ Row 변환
│   ├── workflow/                      # 샌드위치 조립 함수
│   │   ├── mod.rs
│   │   ├── log_tool.rs                # 훅에서 호출되는 로깅 workflow
│   │   ├── log_failure.rs
│   │   ├── log_system.rs
│   │   ├── analyze.rs                 # load → calculate → save
│   │   ├── report.rs
│   │   ├── migrate.rs
│   │   └── changelog.rs
│   └── entrypoint/                    # 외부 인터페이스
│       ├── mod.rs
│       ├── app/                       # CLI 명령어
│       │   ├── mod.rs
│       │   ├── analyze.rs
│       │   ├── report.rs
│       │   ├── changelog.rs
│       │   └── migrate.rs
│       └── hooks/                     # Claude Code 훅
│           ├── mod.rs
│           ├── pre_tool.rs
│           ├── post_tool.rs
│           ├── post_tool_failure.rs
│           ├── notification.rs
│           └── stop.rs
└── tests/                             # 통합 테스트 (E2E)
    ├── migrate_test.rs
    ├── hook_test.rs
    ├── analyze_test.rs
    └── report_test.rs
```

**적용한 컨벤션** (`docs/conventions.md`):
- 함수형 3계층 (`entrypoint → workflow → domain + adapter`)
- Repository trait/Handler struct 없음, 모듈 + 함수로 조직
- domain은 순수 데이터 타입 + 순수 함수 (I/O 없음)
- adapter는 I/O 함수들 (DB 액세스)
- workflow는 Impureim Sandwich: 불순 I/O → 순수 계산 → 불순 I/O
- domain 허용 외부 크레이트: serde, thiserror, chrono, uuid
```

---

## 구현 순서 (Feature-first)

계층별이 아닌 **기능별 증분 구현**. 각 기능은 domain → adapter → workflow → entrypoint의 수직 슬라이스로 완성되며, 완성 후 다음 기능으로 넘어간다.

각 Feature는 [TDD 사이클](../tdd-cycle.md)을 따른다.


### Feature 1: 프로젝트 부트스트랩 + DB 초기화

**목표:** Rust 프로젝트에 SQLite를 연결하고 스키마를 준비한다.

**구현:**
- `Cargo.toml` 의존성 업데이트 (rusqlite, uuid, thiserror 등 추가)
- `adapter/db.rs`: 연결 생성, 스키마 적용 (embedded SQL)
- `domain/error.rs`: `DomainError` 정의 (thiserror)
- `~/.seogi/seogi.db` 자동 생성

**테스트:**
- 빈 DB 초기화 후 스키마 존재 확인
- 재실행 시 기존 스키마 유지
- 인메모리 DB (`:memory:`)로 테스트 격리

**산출물:** 첫 실행 시 DB 파일 자동 생성

---

### Feature 2: 도구 사용 로깅 (`seogi hook post-tool`)

**목표:** Claude Code가 도구를 성공적으로 호출했을 때 SQLite에 기록.

**수직 슬라이스:**
- `domain/log.rs`: `ToolUse` 타입 정의
- `adapter/log_repo.rs`: `save_tool_use` 함수
- `adapter/mapper.rs`: `ToolUse` ↔ Row 변환
- `workflow/log_tool.rs`: 샌드위치 조립 (파싱 → 저장)
- `entrypoint/hooks/post_tool.rs`: stdin 파싱 → workflow 호출

**테스트:**
- `ToolUse::new` Value Object 검증 단위 테스트
- `adapter::log_repo::save_tool_use` 통합 테스트 (인메모리 SQLite)
- `workflow::log_tool::run` 통합 테스트 (실제 adapter + domain)
- E2E: 바이너리 stdin → DB 저장 확인

---

### Feature 3: 도구 실패 로깅 (`seogi hook post-tool-failure`)

**목표:** 도구 호출 실패를 기록.

**수직 슬라이스:**
- `domain/log.rs`: `ToolFailure` 타입 추가
- `adapter/log_repo.rs`: `save_tool_failure` 함수 추가
- `workflow/log_failure.rs`
- `entrypoint/hooks/post_tool_failure.rs`

**테스트:** Feature 2와 동일한 패턴

---

### Feature 4: 시스템 이벤트 로깅 (`seogi hook notification`, `seogi hook stop`)

**목표:** 알림과 세션 종료를 기록.

**수직 슬라이스:**
- `domain/log.rs`: `SystemEvent` 타입 추가
- `adapter/log_repo.rs`: `save_system_event` 함수 추가
- `workflow/log_system.rs`
- `entrypoint/hooks/notification.rs`, `hooks/stop.rs`

**테스트:**
- Feature 2와 동일한 패턴
- `stop.rs`는 추가로 분석기를 백그라운드로 호출 (Feature 6 이후 연결)

---

### Feature 5: 도구 호출 시작 시간 기록 (`seogi hook pre-tool`)

**목표:** 도구 호출 시작 시각을 기록해서 나중에 duration 계산.

**수직 슬라이스:**
- `adapter/timing.rs`: 세션별 시작 시간 저장/조회 함수 (임시 파일)
- `entrypoint/hooks/pre_tool.rs`
- Feature 2의 `workflow/log_tool.rs`에서 시작 시간을 읽어 duration 계산

**테스트:**
- 동일 세션+도구 조합으로 pre → post 호출 시 duration 계산 확인

---

### Feature 6: 세션 분석 (`seogi analyze`)

**목표:** 세션 로그에서 메트릭 10개를 계산하여 저장.

**수직 슬라이스:**
- `domain/metrics.rs`: `SessionMetrics` 타입 + `calculate` 순수 함수
- `adapter/metrics_repo.rs`: `save`, `find_latest` 함수
- `workflow/analyze.rs`: 샌드위치 (load → calculate → save)
- `entrypoint/app/analyze.rs`: `seogi analyze <project> <session_id>`

**테스트:**
- `metrics::calculate` 순수 함수 단위 테스트 (입력 → 출력)
- `workflow::analyze::run` 통합 테스트
- 기존 Rust 분석기 결과와 동일한 값 나오는지 회귀 테스트

**stop 훅 연결:**
- `hooks/stop.rs`에서 `analyze` 명령어를 백그라운드로 호출

---

### Feature 7: 마이그레이션 (`seogi migrate`)

**목표:** 기존 JSONL 로그/메트릭을 SQLite로 옮김.

**수직 슬라이스:**
- `adapter/jsonl_reader.rs`: 기존 JSONL 파서 (pretty-printed + compact)
- `workflow/migrate.rs`: JSONL 읽기 → 도메인 타입 변환 → adapter 함수로 저장
- `entrypoint/app/migrate.rs`: `seogi migrate`

**테스트:**
- 샘플 JSONL 디렉토리 → DB 변환
- 재실행 시 중복 없음 (컨텐츠 기반 id + `INSERT OR IGNORE`)
- 파싱 실패 엔트리는 건너뛰고 경고
- 실제 `~/seogi-logs/` 데이터로 dry-run

---

### Feature 8: 리포트 (`seogi report`)

**목표:** 기간/프로젝트별 메트릭 통계 출력.

**수직 슬라이스:**
- `domain/metrics.rs`: 통계 집계 순수 함수 (`aggregate`)
- `adapter/metrics_repo.rs`: `list_by_range` 함수
- `workflow/report.rs`: 샌드위치 (load → aggregate → print 포매팅)
- `entrypoint/app/report.rs`: `seogi report --from --to --project`

**테스트:**
- 알려진 데이터셋으로 통계 계산 정확성 (순수 함수 단위 테스트)
- 기존 출력 포맷과 동일
- 빈 기간/단일 세션 엣지 케이스

---

### Feature 9: Changelog (`seogi changelog add`)

**목표:** 하니스 변경 이력을 SQLite에 기록.

**수직 슬라이스:**
- `domain/changelog.rs`: `ChangelogEntry` 타입
- `adapter/changelog_repo.rs`: `save` 함수
- `workflow/changelog.rs`: 이력 추가 workflow
- `entrypoint/app/changelog.rs`

**테스트:**
- 단순 추가 케이스
- 재실행 시 중복 방지 불필요 (append-only 이력)

---

### Feature 10: install.sh / uninstall.sh 업데이트 + 기존 코드 제거

**목표:** 배포 통합과 레거시 제거.

**구현:**
- `install.sh`: `~/.claude/settings.json`에 `seogi hook <name>` 명령어 등록
- bash 훅 복사 제거
- `lib/logger.sh`, `config.json`, `hooks/*.sh` 삭제
- `uninstall.sh`: `seogi hook` 명령어 정리 로직 업데이트

**검증:**
- uninstall → install 사이클
- Claude Code에서 실제 도구 호출 → DB 저장 확인
- `seogi report` 정상 동작 확인

---

## 설정 관리

기존 `~/.seogi/config.json`의 두 필드는 SQLite 전환으로 의미를 잃는다:
- `logDir` (`~/seogi-logs`): SQLite 단일 파일로 대체
- `maxFileSizeMB`: SQLite에 롤오버 개념 없음

따라서 **설정 파일을 완전히 삭제**하고 **DB 경로를 하드코딩**한다.

**DB 경로:** `~/.seogi/seogi.db` (고정)

사용자 변경 가능성을 배제하는 이유:
- 로컬 CLI 도구 성격상 다른 디스크 저장 필요성 없음
- 외부 서버/다른 DB는 단순 경로 변경이 아닌 코드 변경 필요
- 설정 표면적을 최소화해 사용자 마찰 감소

---

## 리스크 및 대응

### 리스크 1: 훅 레이턴시

훅은 매 도구 호출마다 실행되므로 실행 시간이 중요. 현재 bash 훅은 jq 파싱 포함 수십~수백 ms. Rust로 전환하면 프로세스 기동 + SQLite 쓰기 오버헤드가 있음.

**대응:**
- 벤치마크: 훅당 < 50ms 목표
- 필요 시 SQLite WAL 모드 활성화:
  - 기본 rollback journal 모드는 쓰기 중 다른 프로세스 읽기 차단
  - WAL(Write-Ahead Logging)은 append-only 로그로 동시성 향상
  - 설정: `conn.pragma_update(None, "journal_mode", "WAL")?;`
  - 훅 빈도가 낮으면 기본 모드로도 충분, 벤치마크 후 판단

### 리스크 2: 마이그레이션 재실행 시 중복 삽입

`seogi migrate`를 여러 번 실행할 때(실패 후 재시도, 새 JSONL 추가 후 재실행 등) 같은 로그 엔트리가 중복 삽입되면 안 됨.

**대응 — 컨텐츠 기반 id 생성:**
- 로그 엔트리의 고유 식별자를 `hash(session_id + timestamp + tool_name)`으로 생성
- 또는 `(session_id, timestamp, tool_name)` 복합 유니크 제약
- 삽입 시 `INSERT OR IGNORE`로 중복 자동 회피
- 같은 내용은 같은 id → 재실행해도 안전

UUID를 매번 새로 생성하면 중복 감지가 불가능하므로 컨텐츠 기반 해싱 필요.

### 리스크 3: 혼합 JSONL 형식 파싱

기존 seogi 초기 버전은 pretty-printed로 로그를 저장했고, 이후 compact 형식으로 전환됨. 같은 세션의 로그가 두 형식이 섞여 있을 수 있음.

```
# pretty-printed (초기)
{
  "timestamp": "...",
  "tool": { "name": "Bash", ... }
}

# compact (현재)
{"timestamp":"...","tool":{"name":"Bash",...}}
```

**대응:**
- 기존 Rust 분석기에 이미 구현된 stream 파서(`serde_json::Deserializer::from_str().into_iter()`) 재사용
- 한 줄에 완전한 JSON이면 compact로 처리, 아니면 stream으로
- 파싱 실패 엔트리는 건너뛰고 경고 출력 (데이터 일부 손실 감수)

---

## 논의 필요 사항

### 논의 1: 마이그레이션 후 원본 JSONL 처리 — 결정됨

**A안: 원본 보존**. 사용자가 마이그레이션 검증 후 직접 삭제.

근거:
- 현재 `~/seogi-logs/` 크기는 27MB — 디스크 부담 없음
- 마이그레이션 버그로 인한 데이터 손실 방지가 우선
- 검증 완료 후 `rm -rf ~/seogi-logs/`로 수동 정리 가능

### 논의 2: DB 파일 위치 — 결정됨

**`~/.seogi/seogi.db`**. seogi 전용 디렉토리 하나에 통합.

uninstall 시 DB도 함께 삭제됨. 데이터 보존이 필요하면 uninstall 전에 백업.
단순한 구조 유지가 우선.

### 논의 3: 훅에서 DB 쓰기 실패 처리 — 결정됨

**B안 + macOS 알림**. 로그 파일에 기록하고 쿨다운을 두어 알림도 띄움.

**절차:**
1. 훅에서 DB 쓰기 실패 발생
2. `~/.seogi/hook-errors.log`에 에러 append (항상 기록)
3. `~/.seogi/last-notification` 파일에서 마지막 알림 시각 읽기
4. 5분 이상 경과했으면:
   - `osascript -e 'display notification "..." with title "seogi"'`로 macOS 알림 발송
   - `last-notification`에 현재 시각 기록
5. 훅은 exit 0으로 종료 (세션 계속 진행)

**알림 내용:**
- 제목: `seogi`
- 메시지: `훅 에러 발생. ~/.seogi/hook-errors.log 확인 필요`

**쿨다운 이유:**
- DB가 지속적으로 잠긴 상태면 훅이 세션당 수십~수백 번 실패할 수 있음
- 매번 알림이 뜨면 오히려 세션을 방해
- 5분 쿨다운으로 noise 제어

**플랫폼:**
- macOS: `osascript` (우선 구현)
- Linux: `notify-send` (필요 시 확장)
- 플랫폼 감지 후 적절한 명령 사용
