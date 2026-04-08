# Seogi (서기)

하니스 엔지니어링을 위한 계측 도구 프레임워크. Claude Code 세션의 도구 사용 패턴을 자동으로 수집하고 분석한다.

## 기능

- 실시간 도구 사용 로깅 (JSONL 형식)
- 도구 실패 로깅 (PostToolUseFailure)
- 세션 종료 시 프록시 지표 10개 자동 산출
- 프로젝트별 로그 파일 분리
- 자동 파일 롤오버 (기본 10MB)

## 설치

### 사전 요구사항

- `jq` — macOS: `brew install jq` / Ubuntu: `apt install jq`

### 설치

```bash
git clone git@github.com:joowankim/seogi.git
cd seogi
./install.sh
```

### 설치 확인

```bash
# 1. 파일 배포 확인
ls ~/.seogi/hooks/
# pre-tool.sh  post-tool.sh  post-tool-failure.sh  notification.sh  stop.sh

# 2. 훅 등록 확인
jq '.hooks | keys' ~/.claude/settings.json
# ["Notification", "PostToolUse", "PostToolUseFailure", "PreToolUse", "Stop"]

# 3. 로그 디렉토리 확인
ls ~/seogi-logs/
```

## 설정

`~/.seogi/config.json` 파일을 편집하세요:

```json
{
  "logDir": "~/seogi-logs",
  "maxFileSizeMB": 10
}
```

| 설정 | 설명 | 기본값 |
|------|------|--------|
| `logDir` | 로그 저장 디렉토리 | `~/seogi-logs` |
| `maxFileSizeMB` | 파일 롤오버 크기 (MB) | `10` |

## 로그 형식

로그는 JSONL (JSON Lines) 형식으로 저장됩니다:

```
~/seogi-logs/
  └── {프로젝트명}/
      ├── 2026-01-30.jsonl
      ├── 2026-01-30_001.jsonl  (롤오버)
      └── ...
```

각 로그 엔트리:

```json
{
  "timestamp": "2026-01-30T14:23:45.000Z",
  "sessionId": "abc123",
  "project": "my-project",
  "projectPath": "/path/to/my-project",
  "role": "assistant",
  "content": "메시지 내용...",
  "tool": {
    "name": "Edit",
    "duration_ms": 1523
  }
}
```

## 제거

```bash
cd seogi
./uninstall.sh
```

로그 파일은 보존됩니다. 완전 삭제:

```bash
rm -rf ~/seogi-logs
```

## 라이선스

MIT
