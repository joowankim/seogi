# Seogi (서기)

Claude Code에서 LLM과의 대화를 실시간으로 로깅하는 Hook 플러그인.

## 기능

- 실시간 대화 로깅 (JSONL 형식)
- 프로젝트별 로그 파일 분리
- 자동 파일 롤오버 (기본 10MB)
- 도구 사용 시간 측정

## 설치

```bash
git clone git@github.com:joowankim/seogi.git
cd seogi
./install.sh
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

## 의존성

- `jq` - JSON 처리

macOS: `brew install jq`
Ubuntu: `apt install jq`

## 라이선스

MIT
