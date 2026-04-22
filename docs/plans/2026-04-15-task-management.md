# 태스크 기반 성과 측정 설계

상위 문서: [measurement-framework.md](./2026-04-08-measurement-framework.md)

---

## 배경

현재 seogi의 측정 단위는 세션이지만, 세션은 임의적인 경계라 "이 작업이 효율적이었는가?"를 알 수 없다.
애자일 팀이 벨로시티에서 사이클 타임/처리량으로 전환한 것처럼,
측정 단위를 **세션에서 태스크(단위 작업)**로 전환한다.

단위 작업으로부터 얻을 수 있는 지표:
- 정량: 사이클 타임, 리드 타임, 처리량, 작업 크기(변경 라인/파일 수), 비용(토큰)
- 정성(자동화 가능): 플로우 효율성, 한 번에 완료 비율, 작업 크기 적절성
- 정성(수동): 목표 달성 여부, 작업 정의 명확성

---

## 아키텍처

- **CLI**: `seogi task create`, `seogi task move`, `seogi hook post-tool` 등
- **MCP 서버**: 에이전트가 세션 중 태스크를 생성/관리할 수 있도록 MCP 프로토콜 제공
- **저장소**: SQLite 단일 파일 (`~/.seogi/seogi.db`)
- **seogi 바이너리 하나**에 CLI + MCP 서버 + 훅 모두 포함

```
seogi task create "..."          # CLI 모드
seogi hook post-tool             # 훅 모드 (Claude Code가 호출)
seogi mcp-server                 # MCP 서버 모드 (Claude Code 연동)
```

### 코드 구조 (함수형 3계층 + ROP)

```
entrypoint 계층 (CLI, MCP 서버, 훅)
    ↓ 호출
workflow 계층 (유스케이스: create_task, start_task, log_tool_use, ...)
    ↓ 호출
domain 계층 (순수 타입 + 순수 함수) + adapter 계층 (SQLite, 파일 I/O)
```

- CLI를 먼저 구현하고, MCP 서버 추가 시 entrypoint만 추가
- ROP: Rust의 `Result<T, E>` 체인으로 자연스럽게 대응
- Repository trait 없이 모듈+함수로 구성 (Dependency Rejection)

---

## DB 스키마

```sql
-- 프로젝트/태스크
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,  -- "SEO", "LOC" 등 (대문자 알파벳 3글자, 태스크 id 접두사)
    goal        TEXT NOT NULL,
    next_seq    INTEGER NOT NULL,     -- 다음 태스크 시퀀스 번호 (도메인에서 초기값 1 설정)
    created_at  TEXT NOT NULL,       -- ISO 8601
    updated_at  TEXT NOT NULL        -- ISO 8601
);

-- status_categories는 DB 테이블이 아닌 코드 enum (StatusCategory)으로 관리

CREATE TABLE statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,      -- StatusCategory enum 값 (backlog, unstarted, started, completed, canceled)
    position    INTEGER NOT NULL
);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY,    -- "{prefix}-{sequence}" 형식 (예: SEO-1, LOC-23)
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    label       TEXT NOT NULL,       -- feature, bug, refactor, chore, docs
    status_id   TEXT NOT NULL REFERENCES statuses(id),
    project_id  TEXT NOT NULL REFERENCES projects(id),
    created_at  TEXT NOT NULL,       -- ISO 8601
    updated_at  TEXT NOT NULL        -- ISO 8601
);

CREATE TABLE task_events (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(id),
    from_status TEXT,
    to_status   TEXT NOT NULL,
    session_id  TEXT NOT NULL,
    timestamp   INTEGER NOT NULL
);

-- 세션 로그 (종류별 분리)
CREATE TABLE tool_uses (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    tool_input      TEXT NOT NULL,
    duration_ms     INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE tool_failures (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    tool_name       TEXT NOT NULL,
    error           TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE TABLE system_events (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    project         TEXT NOT NULL,
    project_path    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    content         TEXT NOT NULL,
    timestamp       INTEGER NOT NULL
);

```

### 설계 원칙

- DEFAULT 값은 DB가 아닌 애플리케이션 레이어에서 처리
- id는 UUID v4 hex 형식 (단, 태스크 id는 `{prefix}-{sequence}` 형식)
- 이벤트/로그의 `timestamp`는 밀리초 Unix timestamp INTEGER
- 엔티티의 `created_at`/`updated_at`은 ISO 8601 TEXT (관습적 구분)
- `task_events.session_id`는 NOT NULL — CLI에서 생성 시 도메인 상수 `CLI_SESSION_ID` 사용
- `task_events.from_status`는 nullable — 최초 생성 시 이전 상태 없음
- 세션 로그는 종류별 테이블 분리 (tool_uses, tool_failures, system_events) — nullable 최소화

---

## 상태 카테고리 시스템

### 5개 고정 카테고리

| 카테고리 | 의미 |
|---|---|
| Backlog | 아이디어를 간략히 적어둔 상태 |
| Unstarted | 구체화 완료, 작업 준비됨 |
| Started | 실제 작업 중 |
| Completed | 완료 |
| Canceled | 취소 |

### 초기 커스텀 상태

| 카테고리 | 커스텀 상태 |
|---|---|
| Backlog | backlog |
| Unstarted | todo |
| Started | in_progress, in_review, blocked |
| Completed | done |
| Canceled | canceled |

### 카테고리 간 전환 규칙

| From | To | 의미 |
|---|---|---|
| Backlog | Unstarted | 구체화 완료 |
| Backlog | Canceled | 아이디어 단계에서 취소 |
| Unstarted | Started | 작업 시작 |
| Unstarted | Backlog | 다시 정리 필요 |
| Unstarted | Canceled | 시작 전 취소 |
| Started | Completed | 작업 완료 |
| Started | Canceled | 작업 중 취소 |
| Started | Unstarted | 작업 보류 |
| Completed | Started | 재작업 (rework) |
| Canceled | Backlog | 취소 복구 |

같은 카테고리 내 커스텀 상태 간에는 자유 전환. 메트릭에 영향 없음.

> 카테고리는 DB 테이블이 아닌 코드 enum (`StatusCategory`)으로 관리. 5개 고정이므로 타입 시스템으로 보장.

---

## 태스크 기반 성과 지표

### 태스크 고유 지표

| 지표 | 계산 | 데이터 소스 |
|---|---|---|
| triage_time | 첫 Backlog → 첫 Unstarted | task_events |
| cycle_time | 첫 Started → 첫 Completed | task_events |
| lead_time | 생성 시점 → 첫 Completed | task_events |
| wait_time | lead_time - cycle_time | 파생 |
| flow_efficiency | cycle_time / lead_time | 파생 |
| throughput | 기간당 Completed 태스크 수 | task_events |
| rework_rate | Completed→Started 전환 발생 태스크 / 전체 완료 | task_events |
| first_time_done_rate | rework 없이 완료된 비율 | task_events |
| issue_age | 현재 시각 - created_at (미완료) | tasks |
| task_size | 변경 라인/파일 수 | git diff (태스크 ID = 브랜치 이름) |
| cost_per_task | 시간 범위 내 토큰 합산 | transcript 파싱 (message.usage) |

### 세션 프록시 지표 (태스크 단위로 집계)

태스크의 Started~Completed 시간 범위 내 tool_uses/tool_failures에서 계산.

| 지표 | 의미 |
|---|---|
| read_before_edit_ratio | 편집 전 파일을 읽었는가 |
| doom_loop_count | 같은 파일을 5회 이상 수정했는가 |
| test_invoked | 테스트를 실행했는가 |
| build_invoked | 빌드를 실행했는가 |
| lint_invoked | 린트를 실행했는가 |
| typecheck_invoked | 타입체크를 실행했는가 |
| tool_call_count | 총 도구 호출 수 |
| bash_error_rate | bash 에러 비율 |

### 제외

| 지표 | 제외 사유 |
|---|---|
| session_count | 세션-태스크 매핑 없이 의미 없음 |

### 데이터 소스별 연결 방식

- **task_events**: 태스크 상태 전환 timestamp로 직접 계산
- **tool_uses/tool_failures**: 태스크의 Started~Completed 시간 범위 내 tool_uses에서 고유 session_id를 추출하여 태스크별로 분리. 같은 세션에서 여러 태스크 작업 시에만 겹침 발생 (실제로는 한 번에 하나의 태스크를 작업하는 워크플로우)
- **git diff**: 브랜치 이름 = 태스크 ID (`SEO-1`). 매칭 브랜치 없으면 task_size 생략
- **transcript 파싱**: tool_uses에서 추출한 session_id로 transcript 파일 경로 특정 (`~/.claude/projects/<hash>/<session_id>.jsonl`). 전체 스캔 불필요. `message.usage.input_tokens` + `output_tokens` 합산. `usage` 필드는 Anthropic API 스펙으로 안정적 (출시 이후 삭제/변경 없음, 추가만 발생)

---

## CLI 명령어

### 프로젝트

```
seogi project create --name "..." --prefix "SEO" --goal "..."
seogi project list
```

### 태스크

```
seogi task create --project <name> --title "..." --description "..." --label feature
seogi task list [--project <name>] [--status <status>] [--label <label>]
seogi task update <task_id> [--title "..."] [--description "..."] [--label <label>]
seogi task move <task_id> <status>
```

### 상태

```
seogi status create --category <category> --name "..."
seogi status update <id> --name "..."
seogi status delete <id>
seogi status list
```

### 훅 (Claude Code가 호출)

```
seogi hook pre-tool
seogi hook post-tool
seogi hook post-tool-failure
seogi hook notification
seogi hook stop
```

stdin으로 Claude Code 훅 데이터를 받아 SQLite에 저장.

### 마이그레이션

```
seogi migrate
```

기존 `~/seogi-logs/**/*.jsonl`을 SQLite로 변환.

### 리포트 (태스크 중심)

```
seogi report --from <date> --to <date> [--project <name>] [--detail]
```

기본 출력: 요약 테이블

```
ID      TITLE              CYCLE    LEAD     TOKENS   SIZE    REWORK
SEO-1   MCP 부트스트랩       2h30m    1d4h     45,230   +342    no
SEO-2   MCP 도구 구현        3h15m    2d1h     62,100   +580    no
SEO-3   MCP 등록+README     1h10m    6h       18,400   +120    no
---
throughput: 3 tasks    flow_efficiency(avg): 0.48    first_time_done: 100%
```

`--detail` 플래그: 태스크별 상세 출력

```
=== SEO-1: MCP 서버 부트스트랩 ===
cycle_time: 2h 30m    lead_time: 1d 4h    flow_efficiency: 0.52
tokens: 45,230 (input: 38,120 / output: 7,110)
task_size: +342 -28 (5 files)
test_invoked: true    doom_loop: 0    bash_error_rate: 0.02
```

### 기존 명령어 (유지)

```
seogi changelog add <description>
```

### MCP 서버

```
seogi mcp-server
```

MCP 도구:
- `project_create`, `project_list`
- `task_create`, `task_list`, `task_update`, `task_move`
- `status_create`, `status_update`, `status_delete`, `status_list`

---

## 구현 순서

### 1단계: SQLite 마이그레이션 + 훅 Rust 전환

1. SQLite 스키마 생성 (session_logs, session_metrics 테이블)
2. `seogi migrate` — 기존 JSONL → SQLite 변환
3. 훅 5개를 Rust로 전환 (`seogi hook pre-tool/post-tool/post-tool-failure/notification/stop`)
4. `~/.claude/settings.json` 훅 등록을 `seogi hook ...` 명령어로 변경
5. `seogi analyze`, `seogi report` → SQLite 기반으로 변경
6. install.sh/uninstall.sh 업데이트
7. bash 훅, lib/logger.sh, config.json 삭제

### 2단계: 태스크 관리

> 테이블은 Phase 1에서 이미 생성됨. 리팩토링도 Phase 1에서 함수형 3계층으로 완료.

| Feature | 내용 | 의존성 |
|---------|------|--------|
| 11 | 초기 데이터 시딩 — `StatusCategory` enum, `status_categories` 테이블 DROP, `statuses.category TEXT` 변경, `projects.next_seq` 추가, 기본 statuses 7개 시딩 | 없음 |
| 12 | Project CRUD — `project create/list`, `Prefix` newtype (대문자 3글자), 출력 테이블+`--json` | 11 |
| 13 | Status CRUD — `status create/update/delete/list`, 카테고리 유효성 검증, 출력 테이블+`--json` | 11 |
| 14 | Task 생성/조회 — `task create/list`, `Label` enum, `{prefix}-{seq}` ID, 초기상태 backlog, 필터링 | 12, 13 |
| 15 | Task 업데이트 — `task update` (title, description, label 수정) | 14 |
| 16 | FSM + Task 상태 전환 — `task move`, 카테고리 간 전환 규칙, `Canceled→Backlog` 허용, `task_events` 기록 | 14 |

**결정 사항:**
- 카테고리는 DB 테이블이 아닌 코드 enum (`StatusCategory`)
- 기본 statuses 7개는 스키마 적용 시 자동 삽입
- `projects.next_seq`로 시퀀스 채번 (DEFAULT 없이 도메인에서 초기값 설정)
- `--project`는 프로젝트 이름, prefix는 기본값 이름 앞 3글자 대문자
- 출력은 테이블 + `--json` 플래그
- `start`/`done` 단축 없이 `move`로 통일
- `Canceled → Backlog` 복구 허용
- CLI session_id는 도메인 상수

### 3단계: MCP 서버

> entrypoint 계층에서만 변경. workflow/domain/adapter는 CLI와 동일하게 재사용.

| Feature | 내용 | 의존성 |
|---------|------|--------|
| 17 (SEO-1) | MCP 서버 부트스트랩 — `rmcp` 크레이트, `seogi mcp-server` 명령어, stdio transport | 없음 |
| 18 (SEO-2) | MCP 도구 구현 — project/status/task 도구 10개, 기존 workflow 재사용, `spawn_blocking` 래핑 | 17 |
| 19 (SEO-3) | Claude Code MCP 등록 + README — install.sh/uninstall.sh에 MCP 설정 추가/제거, CLI/MCP 사용법 README 작성 | 18 |

**결정 사항:**
- MCP 크레이트: `rmcp` (공식 SDK, `modelcontextprotocol/rust-sdk`)
- Transport: stdio (Claude Code 기본)
- async 래핑: workflow는 sync(rusqlite), MCP 서버에서 `spawn_blocking`으로 감싸 호출
- 도구 10개를 하나의 feature로 통합 (동일 패턴)

### 4단계: 태스크 기반 성과 지표

> `seogi report`를 세션 중심에서 태스크 중심으로 교체. `session_metrics` 테이블 제거.

| Feature | 내용 | 의존성 |
|---------|------|--------|
| 20 (SEO-4) | `session_metrics` 테이블 DROP + `seogi analyze` 제거 — 스키마에서 테이블 삭제, analyze 서브커맨드/워크플로우/테스트 삭제, 세션 기반 report 워크플로우 삭제 | 없음 |
| 21 (SEO-5) | 태스크 지표 도메인 — task_events 기반 지표 계산 순수 함수 9개 (cycle_time, lead_time, throughput 등) | 없음 |
| 22 (SEO-6) | 프록시 지표의 태스크 단위 집계 — `SessionMetrics`를 시간 범위 기반 계산으로 변경, 태스크의 Started~Completed 구간 내 tool_uses/tool_failures에서 계산 | 20 |
| 23 (SEO-7) | transcript 파싱 + cost_per_task — Claude Code transcript JSONL에서 `message.usage` 추출, 시간 범위 내 input_tokens + output_tokens 합산 | 없음 |
| 24 (SEO-8) | task_size (git diff 기반) — 브랜치 이름 = 태스크 ID로 매칭, `git diff main...<task-id>`로 변경량 계산, 매칭 브랜치 없으면 생략 | 없음 |
| 25 (SEO-9) | `seogi report` 태스크 중심 교체 — 모든 지표 통합 출력 (태스크 고유 + 프록시 + cost + size), 기존 세션 기반 report 대체 | 21, 22, 23, 24 |

**결정 사항:**
- 세션-태스크 매핑 불필요 — 시간 범위 기반으로 tool_uses/tool_failures 필터링
- `session_metrics` 테이블 제거 — on-the-fly 계산으로 충분 (개인 도구, 세션 수 적음)
- 토큰 데이터는 transcript 파싱으로 획득 — `message.usage.input_tokens` + `output_tokens` (API 스펙, 안정적)
- `task_size`는 git diff 기반 — 브랜치 이름 = 태스크 ID (`SEO-1`), 매칭 브랜치 없으면 생략
- `session_count` 제외 — 세션-태스크 매핑 없이 의미 없음
- `seogi report`를 태스크 중심으로 완전 교체 — 세션 프록시 지표도 태스크 시간 범위로 집계

### 5단계: 태스크 관리 고도화

> 단일 조회, 의존성, 서브태스크를 추가하여 태스크 관리 기능의 완성도를 높인다.

| Feature | 내용 | 의존성 |
|---------|------|--------|
| 26 (SEO-11) | 단일 태스크 조회 — `seogi task get <id>` CLI + `task_get` MCP 도구. description 포함 상세 출력 | 없음 |
| 27 (SEO-12) | 의존성 라벨링 — `task_dependencies` 테이블, `seogi task depend <id> --on <id>`, `task list`에서 blocked 표시, MCP 도구 추가 | 없음 |
| 28 (SEO-13) | 서브태스크 — `tasks.parent_id` 컬럼, `seogi task create --parent <id>`, 부모-자식 관계 표시, 서브태스크 전체 완료 시 부모 완료 가능 로직 | 27 |

**보류 사항:**
- 코멘트: PR description으로 충분. 별도 목적이 생기면 재검토
- GitHub PR 훅: 외부 서버 필요. PR 생성/머지 이벤트로 자동 상태 전환 (토큰 절약 효과)
- 스프린트/사이클: 데이터가 충분히 쌓인 후 도입. ground-truth 목적 2 (변경 전후 비교)와 연결
- 하니스별 메트릭 구분: 하니스 관리 도구 선정/개발 후 연동. task에 harness_id 입력

**참조:**
- [Picrew/awesome-agent-harness](https://github.com/Picrew/awesome-agent-harness) — 에이전트 하니스 도구 목록
- [AutoJunjie/awesome-agent-harness](https://github.com/AutoJunjie/awesome-agent-harness) — 태스크 라이프사이클, 비용 추적, 하니스 버전 관리 도구 목록

---

## 논의 결과

- 저장소: SQLite 단일 파일로 통합. 마이그레이션을 태스크 관리보다 먼저 수행.
- 훅: bash → Rust 전환. `seogi hook <name>` 명령어로 대체.
- 상태 시스템: Linear 방식의 2단계 구조 (고정 카테고리 5개 + 커스텀 상태).
- 이벤트: from_status/to_status 방식으로 기록. 고정 이벤트 타입 없음.
- 코드 구조: 함수형 3계층 + ROP. entrypoint(CLI/MCP) → workflow → (domain + adapter).
- MCP 서버: CLI와 같은 서비스 인터페이스 사용. seogi 바이너리에 포함.
- 에이전트 태스크 생성: description을 자세히 작성. title/description 업데이트 가능.
- 에이전트 상태 전환: done 포함 모든 전환 허용.
- 세션-태스크 매핑: 불필요. 시간 범위 기반으로 추적. Claude Code가 MCP/훅에 session_id를 제공하지 않으며, 세션 ≠ 태스크 (1:1 대응 아님).
- session_metrics 테이블: 제거. on-the-fly 계산으로 충분.
- 토큰 메트릭: transcript JSONL의 `message.usage` 파싱. `usage` 필드는 Anthropic API 스펙으로 안정적.
- task_size: git diff 기반. 브랜치 이름 = 태스크 ID. 매칭 브랜치 없으면 생략.
- 리포트: `seogi report`를 세션 중심에서 태스크 중심으로 완전 교체.
- 코멘트 기능: PR description으로 충분. 별도 목적이 생기면 재검토.
- 하니스별 메트릭: 하니스 관리 도구 선정/개발 후 연동 예정. seogi 내부에서는 만들지 않음.
