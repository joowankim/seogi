# Seogi

하니스 엔지니어링을 위한 계측 도구 프레임워크.

Claude Code의 하니스(CLAUDE.md, 스킬, 훅, MCP 서버 등) 성능을 정량적으로 측정하고, 하니스 변경 전후의 업무 효율을 비교할 수 있는 데이터를 자동으로 수집한다.

## 설치

```bash
git clone git@github.com:joowankim/seogi.git
cd seogi
./install.sh
```

install.sh는 다음을 수행한다:

1. `~/.seogi/` 디렉토리 생성
2. `cargo install`로 seogi 바이너리 설치
3. `~/.claude/settings.json`에 Claude Code 훅 등록
4. `~/.claude.json`에 MCP 서버 등록

### 요구 사항

- Rust 툴체인 (`cargo`)
- `jq` — macOS: `brew install jq` / Ubuntu: `apt install jq`

## CLI 명령어

### 워크스페이스 관리

```bash
seogi workspace create --name "MyWorkspace" --goal "workspace goal"
seogi workspace list [--json]
```

### 상태 관리

```bash
seogi status create --category <category> --name <name>
seogi status list [--json]
seogi status update --id <id> --name <new_name>
seogi status delete --id <id>
```

### 태스크 관리

```bash
seogi task create --workspace <name> --title <title> --description <desc> --label <label>
seogi task list [--workspace <name>] [--status <name>] [--label <label>] [--json]
seogi task update --task-id <id> [--title <title>] [--description <desc>] [--label <label>]
seogi task move --task-id <id> --status <name>
```

### 세션 분석

```bash
seogi analyze <session_id>           # 세션 프록시 지표 계산
seogi report --from <date> --to <date> [--workspace <name>]  # 기간별 리포트
```

### 기타

```bash
seogi changelog add --description <text>   # 하니스 변경 이력 기록
seogi migrate                              # JSONL 로그를 SQLite로 마이그레이션
seogi hook <name>                          # Claude Code 훅 (자동 호출)
seogi mcp-server                           # MCP 서버 (stdio transport)
```

## MCP 서버

seogi는 MCP(Model Context Protocol) 서버로 동작하여 Claude Code 에이전트가 세션 중 태스크를 직접 관리할 수 있다.

### 제공 도구

| 도구 | 설명 |
|---|---|
| `workspace_create` | 워크스페이스 생성 |
| `workspace_list` | 워크스페이스 목록 조회 |
| `status_create` | 상태 생성 |
| `status_list` | 상태 목록 조회 |
| `status_update` | 상태 이름 변경 |
| `status_delete` | 상태 삭제 |
| `task_create` | 태스크 생성 |
| `task_list` | 태스크 목록 조회 (필터 지원) |
| `task_update` | 태스크 수정 |
| `task_move` | 태스크 상태 전환 |

### 수동 등록

install.sh를 사용하지 않고 직접 등록하려면 `~/.claude.json`에 추가:

```json
{
  "mcpServers": {
    "seogi": {
      "command": "seogi",
      "args": ["mcp-server"]
    }
  }
}
```

## 제거

```bash
./uninstall.sh
```

uninstall.sh는 다음을 수행한다:

1. `~/.claude/settings.json`에서 seogi 훅 제거
2. `~/.claude.json`에서 MCP 서버 설정 제거
3. `~/.seogi/` 디렉토리 삭제

로그 파일(`~/seogi-logs/`)은 보존된다. 완전 삭제: `rm -rf ~/seogi-logs`

## 라이선스

MIT
