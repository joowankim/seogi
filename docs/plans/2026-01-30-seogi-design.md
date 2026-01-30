# Seogi (서기) - LLM 대화 로깅 도구 설계

## 개요

Claude Code 또는 OpenCode 사용 시 LLM과의 대화 내용을 실시간으로 파일에 저장하는 Hook 기반 도구.

**목적 (우선순위):**
1. 작업 기록 (audit trail)
2. 지식베이스 구축
3. 학습/복습
4. 디버깅/분석

**저장소:** git@github.com:joowankim/seogi.git

## 전체 아키텍처

**핵심 컴포넌트:**
- Claude Code Hook (`seogi-hook`) - 실시간 로그 캡처 및 저장

**동작 흐름:**
```
User/Assistant 메시지 발생
       ↓
PostToolUse / Notification Hook 트리거
       ↓
메시지 데이터 추출 (role, content, tool, timestamp 등)
       ↓
JSON 객체로 직렬화
       ↓
로그 파일에 append (JSONL 형식)
       ↓
파일 크기 체크 → 기본 10MB 초과 시 롤오버
```

**로그 저장 경로:**
```
{사용자 지정 디렉토리}/
  └── {프로젝트명}/
      ├── 2026-01-30.jsonl      (첫 번째 파일)
      ├── 2026-01-30_001.jsonl  (롤오버 시)
      └── 2026-01-30_002.jsonl
```

**설정 파일:** `~/.seogi/config.json`
```json
{
  "logDir": "~/seogi-logs",
  "maxFileSizeMB": 10
}
```

## 데이터 스키마

**각 로그 엔트리 (JSONL 한 줄):**
```json
{
  "timestamp": "2026-01-30T14:23:45.123Z",
  "sessionId": "abc123-def456",
  "project": "seogi",
  "projectPath": "/Users/kimjoowan/projects/seogi",
  "role": "assistant",
  "content": "로그 저장 기능을 구현할게요...",
  "tool": {
    "name": "Edit",
    "duration_ms": 1523
  }
}
```

**필드 설명:**
| 필드 | 타입 | 설명 |
|------|------|------|
| `timestamp` | ISO8601 | 메시지 발생 시각 |
| `sessionId` | string | Claude Code 세션 고유 ID |
| `project` | string | 프로젝트 디렉토리명 |
| `projectPath` | string | 프로젝트 전체 경로 |
| `role` | "user" \| "assistant" | 발화자 |
| `content` | string | 메시지 내용 |
| `tool` | object \| null | 도구 사용 시 이름과 소요 시간 |

## Hook 구현

**사용할 Hook 이벤트:**

| Hook | 용도 |
|------|------|
| `PreToolUse` | 도구 호출 시작 시간 기록 |
| `PostToolUse` | 도구 호출 완료, 소요 시간 계산 후 로깅 |
| `Notification` | Stop, 에러 등 세션 이벤트 캡처 |

**Hook 설정 (`~/.claude/settings.json`):**
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "*",
      "hooks": ["~/.seogi/hooks/pre-tool.sh"]
    }],
    "PostToolUse": [{
      "matcher": "*",
      "hooks": ["~/.seogi/hooks/post-tool.sh"]
    }],
    "Notification": [{
      "matcher": "*",
      "hooks": ["~/.seogi/hooks/notification.sh"]
    }]
  }
}
```

**Hook 스크립트 구조:**
```
~/.seogi/
├── config.json          # 사용자 설정
├── hooks/
│   ├── pre-tool.sh      # 시작 시간 임시 저장
│   ├── post-tool.sh     # 로그 기록 (메인 로직)
│   └── notification.sh  # 세션 이벤트 로깅
└── lib/
    └── logger.sh        # 공통 로깅 함수 (롤오버 포함)
```

**구현 언어:** Bash + jq (의존성 최소화, 설치 간편)

## 배포

**패키지 구조:**
```
seogi/
├── README.md           # 사용법, 설치 가이드
├── LICENSE             # MIT 또는 Apache 2.0
├── install.sh          # 자동 설치 스크립트
├── uninstall.sh        # 제거 스크립트
├── config.json         # 기본 설정 템플릿
├── hooks/
│   ├── pre-tool.sh
│   ├── post-tool.sh
│   └── notification.sh
└── lib/
    └── logger.sh
```

**설치 흐름:**
1. 사용자가 마켓플레이스에서 `seogi` 설치
2. `install.sh` 실행 → `~/.seogi/` 디렉토리 생성
3. Claude Code settings.json에 hook 설정 자동 추가
4. 기본 로그 디렉토리 `~/seogi-logs/` 생성
5. 설치 완료 메시지 출력

## 향후 확장 (v2)

- OpenCode 지원 (Go 기반 hook)
- Skill 추가: `/seogi summary` → Markdown 요약 생성
- 레벨별 로깅 설정 (최소/중간/상세)
