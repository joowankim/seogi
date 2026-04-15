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

### 코드 구조 (DDD + ROP)

```
representation 계층 (CLI, MCP 서버, 훅)
    ↓ 호출
service 계층 (유스케이스: create_task, start_task, log_tool_use, ...)
    ↓ 호출
domain 계층 (Task, Project, Status, LogEntry 엔티티 + FSM 로직)
    ↓ 호출
infrastructure 계층 (SQLite 리포지토리)
```

- CLI를 먼저 구현하고, MCP 서버 추가 시 representation만 추가
- ROP: Rust의 `Result<T, E>` 체인으로 자연스럽게 대응
- 기존 코드를 이 구조로 리팩토링

---

## DB 스키마

```sql
-- 프로젝트/태스크
CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    prefix      TEXT NOT NULL UNIQUE,  -- "SEO", "LOC" 등 (태스크 id 접두사)
    goal        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE status_categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL
);

CREATE TABLE statuses (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES status_categories(id),
    position    INTEGER NOT NULL
);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY,    -- "{prefix}-{sequence}" 형식 (예: SEO-1, LOC-23)
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    label       TEXT NOT NULL,       -- feature, bug, refactor, chore, docs
    status_id   TEXT NOT NULL REFERENCES statuses(id),
    project_id  TEXT NOT NULL REFERENCES projects(id),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
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

-- 세션 메트릭
CREATE TABLE session_metrics (
    id                      TEXT PRIMARY KEY,
    session_id              TEXT NOT NULL,
    project                 TEXT NOT NULL,
    read_before_edit_ratio  INTEGER NOT NULL,
    doom_loop_count         INTEGER NOT NULL,
    test_invoked            INTEGER NOT NULL,
    build_invoked           INTEGER NOT NULL,
    lint_invoked            INTEGER NOT NULL,
    typecheck_invoked       INTEGER NOT NULL,
    tool_call_count         INTEGER NOT NULL,
    session_duration_ms     INTEGER NOT NULL,
    edit_files              TEXT NOT NULL,
    bash_error_rate         REAL NOT NULL,
    timestamp               INTEGER NOT NULL
);
```

### 설계 원칙

- DEFAULT 값은 DB가 아닌 애플리케이션 레이어에서 처리
- id는 UUID v4 hex 형식 (단, 태스크 id는 `{prefix}-{sequence}` 형식)
- 모든 timestamp는 밀리초 Unix timestamp INTEGER
- `task_events.session_id`는 NOT NULL — CLI에서 생성 시 `"cli"` 값 사용
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

같은 카테고리 내 커스텀 상태 간에는 자유 전환. 메트릭에 영향 없음.

---

## 태스크 기반 성과 지표

| 지표 | 계산 | 의미 |
|---|---|---|
| triage_time | 첫 Backlog → 첫 Unstarted | 아이디어 구체화 시간 |
| cycle_time | 첫 Started → 첫 Completed | 실제 작업 소요 시간 |
| lead_time | 생성 시점 → 첫 Completed | 전체 소요 시간 |
| wait_time | lead_time - cycle_time | 대기 시간 |
| flow_efficiency | cycle_time / lead_time | 대기 비율 |
| throughput | 기간당 Completed 태스크 수 | 처리량 |
| rework_rate | Completed→Started 전환 발생 태스크 / 전체 완료 | 재작업 비율 |
| first_time_done_rate | rework 없이 완료된 비율 | 첫 구현 품질 |
| issue_age | 현재 시각 - created_at (미완료) | 방치된 백로그 감지 |
| task_size | 관련 세션의 변경 라인/파일 수 | 작업 크기 |
| cost_per_task | 관련 세션의 토큰 합산 | 비용 효율 |
| session_count | 태스크에 관여한 세션 수 | 작업 분산도 |

---

## CLI 명령어

### 프로젝트

```
seogi project create --name "..." --prefix "SEO" --goal "..."
seogi project list
```

### 태스크

```
seogi task create --project <id> --title "..." --description "..." --label feature
seogi task list [--project <id>] [--status <status>] [--label <label>]
seogi task update <task_id> [--title "..."] [--description "..."] [--label <label>]
seogi task move <task_id> <status>
seogi task start <task_id>    # 단축: move <id> in_progress
seogi task done <task_id>     # 단축: move <id> done
```

### 상태

```
seogi status create --category <category> --name "..."
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

### 기존 명령어 (유지)

```
seogi analyze <project> <session_id> [--transcript <path>] [--start-sha <sha>]
seogi report --from <date> --to <date> [--project <name>]
seogi changelog add <description>
```

### MCP 서버

```
seogi mcp-server
```

MCP 도구:
- `project_create`, `project_list`
- `task_create`, `task_list`, `task_update`, `task_move`, `task_start`, `task_done`
- `status_create`, `status_list`

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

1. 프로젝트/태스크/상태 카테고리/커스텀 상태 테이블 추가
2. 도메인 로직 (FSM, 상태 전환 유효성 검증)
3. CLI 명령어 (project, task, status)
4. DDD 구조로 기존 코드 리팩토링

### 3단계: MCP 서버

1. MCP 프로토콜 구현 (`seogi mcp-server`)
2. CLI와 동일한 서비스 인터페이스 사용
3. Claude Code MCP 설정 등록

### 4단계: 태스크 기반 성과 지표

1. task_events + session_metrics 조인으로 지표 계산
2. `seogi report`에 태스크 기반 지표 추가
3. 아웃컴 지표 (토큰, git 데이터) 통합

---

## 논의 결과

- 저장소: SQLite 단일 파일로 통합. 마이그레이션을 태스크 관리보다 먼저 수행.
- 훅: bash → Rust 전환. `seogi hook <name>` 명령어로 대체.
- 상태 시스템: Linear 방식의 2단계 구조 (고정 카테고리 5개 + 커스텀 상태).
- 이벤트: from_status/to_status 방식으로 기록. 고정 이벤트 타입 없음.
- 코드 구조: DDD + ROP. representation(CLI/MCP) → service → domain → infrastructure.
- MCP 서버: CLI와 같은 서비스 인터페이스 사용. seogi 바이너리에 포함.
- 에이전트 태스크 생성: description을 자세히 작성. title/description 업데이트 가능.
- 에이전트 상태 전환: done 포함 모든 전환 허용.
