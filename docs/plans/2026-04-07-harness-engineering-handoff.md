# Seogi 하네스 엔지니어링 전환 — 핸드오프 문서

## 1. 배경: 문제 인식의 여정

### 1-1. 출발점 — 하네스 엔지니어링 플레이북

코딩 에이전트(Claude Code)를 지속적으로 개선하기 위한 체계를 만들고 싶었다. "하네스 엔지니어링 플레이북"을 작성하고 Phase 0(현재 상태 스냅샷)부터 시작했다.

- **하네스 인벤토리** 작성 완료 — CLAUDE.md, 스킬 28개(글로벌 10 + 프로젝트 18), MCP 서버 5개, 훅 3종, 퍼미션 472개 allow 규칙 등 현재 구성을 기록
- **실패 로그** 수집 시작 — 에이전트가 실수할 때마다 기록하는 failure-log.md 작성

### 1-2. 실패 로그를 쓰면서 깨달은 것

실패 로그를 3건 작성해보니, 예상과 다른 패턴이 보였다:

> "이미 기초적인 하네스가 있어서 기본적으로는 잘 지켜지는데, 가끔씩 내가 지시하진 않았는데 조금 더 바라는 게 있는 패턴이라서, 뭔가 에이전트의 실수라고 하기는 어려운 부분들이 더 많았다."

즉, **"에이전트가 틀렸다"보다 "에이전트가 내 이상적인 워크플로우와 다르게 행동한다"가 본질**이었다. 실패 수정이 아니라 **행동 정렬(behavior alignment)** 문제였다.

실제 기록된 3건도 이 패턴이다:
1. UI 컴포넌트 추가 후 Storybook에 등록되었는지 확인 안 함 (Verify 위시)
2. Storybook 컴포넌트를 사용하라고 했는데 인라인 코드로 구현 (Inform 위시)
3. 같은 패턴이 다른 컴포넌트에서도 반복 (Inform 위시)

이건 "버그"가 아니다. 에이전트는 지시받은 대로 했고, 결과물도 동작한다. 다만 **내가 원하는 수준의 행동**에 도달하지 못한 것이다.

### 1-3. "내가 원하는 에이전트 행동"이 명확해짐

메모를 정리하면서 이상적인 에이전트 행동 모델이 3가지 모드로 정리되었다:

**모드 1: 자율 실행** — 결정된 사항이 있으면 멈추지 않고 끝까지 수행. 작업 상태를 스스로 관리.

**모드 2: 중단 + 의사결정 요청** — 미결 사항을 만나면 즉시 멈추고 문서를 작성. 그 문서만 읽으면 맥락 없이도 무엇을 결정해야 하는지 알 수 있어야 함.

**모드 3: 오류 처리** — 오류 발생 → 자체 분석·해결 시도 → 해결되면 리포트 작성. 해결 과정에서 미결 사항이 생기면 모드 2로 전환.

관통 규칙:
- 테스트 커버리지(브랜치) 100%
- 문서의 문맥독립성 보장 (목표, 완료조건, 미결항목이 명확)

### 1-4. 측정 없이는 개선할 수 없다

워크플로우를 바꾸고 싶은데, 바꾼 후에 **"실제로 나아졌는가?"를 어떻게 알 수 있는가?**

수동 로그는 며칠이면 귀찮아져서 안 하게 된다. 자동으로 데이터가 쌓여야 지속 가능하다. 그래서 **"에이전트가 내 이상적인 행동에 얼마나 가까운가"를 자동으로 측정하는 시스템**이 필요해졌다.

이것이 seogi를 하네스 엔지니어링 플랫폼으로 전환하려는 이유다.

---

## 2. 왜 seogi인가

seogi는 이미:
- 매 도구 호출마다 JSONL 로그를 자동 기록하고 있다 (PostToolUse 훅)
- 120개+ 프로젝트의 히스토리가 쌓여 있다
- Bash + jq 기반으로 의존성이 최소다
- 훅 프레임워크(pre-tool → post-tool → notification)가 동작 중이다

**로그는 이미 쌓이고 있다. 분석만 없다.** 로깅 레이어 위에 분석 레이어를 얹으면 자연스럽게 측정 시스템이 된다.

---

## 3. 만들어서 얻고 싶은 것

### 3-1. 즉각적 목표: 기준선(Baseline) 자동 확보

워크플로우를 바꾸기 **전**의 에이전트 행동 패턴을 수치로 갖고 있어야 한다. 지금 seogi에 분석 기능을 추가하면, 앞으로 에이전트를 쓸 때마다 **내가 아무것도 안 해도** 기준선 데이터가 쌓인다.

예시 — 현재(변경 전):
```
세션 f9e34cc3: read_before_edit=1, test_invoked=false, doom_loops=0
세션 a29ec799: read_before_edit=4, test_invoked=true, doom_loops=1
```

워크플로우 변경 후 같은 지표가 개선되었는지 비교:
```
변경 전 평균: read_before_edit=2.5, test_invoked=50%
변경 후 평균: read_before_edit=5.2, test_invoked=90%  → 개선 확인
```

### 3-2. 핵심 목표: "에이전트가 내 이상적 행동에 얼마나 가까운가"를 프록시 지표로 근사

직접 측정이 불가능한 것(예: "멈춰야 할 곳에서 멈추는가")을 자동 수집 가능한 프록시로 대체한다.

| 내가 원하는 행동 | 프록시 지표 | 자동 측정 방법 |
|---|---|---|
| 기존 코드를 탐색한 후 구현 | `read_before_edit_ratio` | 첫 Edit/Write 전 Read/Grep/Glob 호출 수 |
| 둠 루프에 빠지지 않음 | `doom_loop_count` | 동일 파일 Edit 5회 이상 발생 횟수 |
| 테스트를 돌려서 검증 | `test_invoked` | Bash에서 test/vitest/playwright 호출 여부 |
| 빌드를 확인 | `build_invoked` | Bash에서 build/tsc 호출 여부 |
| 작업이 과도하게 복잡해지지 않음 | `tool_call_count` | 총 도구 호출 수 |
| 효율적으로 작업 | `session_duration_ms` | 첫 도구 ~ 마지막 도구 시간 차 |
| 변경 범위가 적절함 | `edit_files` | Edit/Write한 고유 파일 목록 |

이 프록시들은 완벽하지 않다. 하지만 **수동 로그 0건보다 자동 프록시 100건이 낫다.** 프록시가 실제와 일치하는지는 가끔 수동으로 검증하면 된다(주 1회, 2~3건).

### 3-3. 장기 목표: 하네스 변경의 A/B 테스트 인프라

하네스(CLAUDE.md 규칙, 스킬, 훅)를 변경할 때마다:
1. 변경 전 지표 확인 (기준선)
2. 변수 하나만 변경
3. 변경 후 지표 비교
4. 효과가 있으면 채택, 없으면 롤백

이 사이클을 돌리려면 **자동으로 쌓이는 지표 데이터**가 전제 조건이다. seogi가 이 전제 조건을 충족시키는 도구가 된다.

### 3-4. 참고: 에이전트 실패의 4가지 기둥

프록시 지표가 포착하는 영역을 분류하면:

| 기둥 | 의미 | 프록시 지표 매핑 |
|------|------|-----------------|
| **Inform** | 에이전트가 알았어야 할 걸 몰랐는가 | `read_before_edit_ratio` (탐색 부족) |
| **Constrain** | 에이전트가 하지 말았어야 할 걸 했는가 | `edit_files` (범위 이탈), `doom_loop_count` |
| **Verify** | 에이전트가 확인했어야 할 걸 안 했는가 | `test_invoked`, `build_invoked` |
| **Correct** | 에이전트가 실패에서 복구하지 못했는가 | `doom_loop_count`, `tool_call_count` (비정상 증가) |

---

## 4. seogi 현재 상태

### 프로젝트 구조

```
~/projects/seogi/                    ← git repo (main에는 설계문서만)
  .worktrees/feature-hook-impl/     ← 구현 완료, main 미머지
    hooks/pre-tool.sh               ← PreToolUse: 시작 시간 임시 저장
    hooks/post-tool.sh              ← PostToolUse: JSONL 로그 기록 (메인)
    hooks/notification.sh           ← Notification: 세션 이벤트 로깅
    lib/logger.sh                   ← 공통 로깅 (롤오버 포함)
    config.json                     ← 설정 템플릿
    install.sh / uninstall.sh       ← 설치/제거 스크립트

~/.seogi/                            ← 실제 동작 중 (워크트리에서 수동 복사)
  config.json                       ← logDir: ~/seogi-logs, maxFileSizeMB: 10
  hooks/{pre-tool,post-tool,notification}.sh
  lib/logger.sh

~/seogi-logs/                        ← 120개+ 프로젝트의 raw 로그
  locs/2026-04-07.jsonl             ← 일별 JSONL, 프로젝트별 디렉토리
```

### 로그 엔트리 포맷 (현재)

```json
{
  "timestamp": "2026-04-07T11:20:56.000Z",
  "sessionId": "f9e34cc3-f145-4721-8ed9-f38892e1d4cc",
  "project": "locs",
  "projectPath": "/Users/kimjoowan/projects/locs",
  "role": "assistant",
  "content": "",
  "tool": {
    "name": "Bash",
    "duration_ms": 0
  }
}
```

### 훅 설정 위치 (`~/.claude/settings.json`)

```json
{
  "hooks": {
    "Notification": [
      { "matcher": "permission_prompt", "hooks": [{ "type": "command", "command": "osascript ..." }] },
      { "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": "osascript ..." }] },
      { "matcher": "*", "hooks": [{ "type": "command", "command": "~/.seogi/hooks/notification.sh" }] }
    ],
    "PreToolUse": [
      { "matcher": "*", "hooks": [{ "type": "command", "command": "~/.seogi/hooks/pre-tool.sh" }] }
    ],
    "PostToolUse": [
      { "matcher": "*", "hooks": [{ "type": "command", "command": "~/.seogi/hooks/post-tool.sh" }] }
    ]
  }
}
```

**Stop 훅은 아직 없음** — 이번에 추가해야 할 핵심.

### Claude Code 훅 API 참고

- 훅은 stdin으로 JSON을 받음 (환경변수 아님)
- 공통 필드: `session_id`, `transcript_path`, `cwd`, `hook_event_name`
- PostToolUse 추가 필드: `tool_name`, `tool_use_id`, `tool_input`, `tool_response`
- **Stop 훅**: 에이전트가 전체 턴을 마쳤을 때 실행됨. `stop_reason`과 대화 컨텍스트를 받을 수 있음.

---

## 5. 해야 할 작업

### 작업 1: feature-hook-impl → main 머지

워크트리의 구현을 main에 머지한다. 이미 `~/.seogi/`에서 동작 확인됨.

### 작업 2: 메트릭 분석 레이어 추가

#### 2-1. Stop 훅 스크립트 (`hooks/stop.sh`)

세션 종료 시 실행. `analyzers/session-summary.sh`를 호출한다.

#### 2-2. 세션 요약 분석기 (`analyzers/session-summary.sh`)

현재 세션의 raw JSONL 로그에서 프록시 지표 7개를 계산한다.

| # | 지표 | 계산 방법 |
|---|------|----------|
| 1 | `read_before_edit_ratio` | 첫 Edit/Write 전 Read/Grep/Glob 호출 수 |
| 2 | `doom_loop_count` | 동일 파일 Edit 5회 이상 발생 횟수 |
| 3 | `test_invoked` | Bash에서 test/vitest/playwright 호출 여부 |
| 4 | `build_invoked` | Bash에서 build/tsc 호출 여부 |
| 5 | `tool_call_count` | 총 도구 호출 수 |
| 6 | `session_duration_ms` | 첫 도구 ~ 마지막 도구 시간 차 |
| 7 | `edit_files` | Edit/Write한 고유 파일 목록 |

#### 2-3. 메트릭 저장

```
~/seogi-logs/<project>/metrics/YYYY-MM-DD.jsonl   ← 세션별 1줄
```

```json
{
  "timestamp": "2026-04-07T15:30:00.000Z",
  "sessionId": "f9e34cc3-...",
  "project": "locs",
  "metrics": {
    "read_before_edit_ratio": 3,
    "doom_loop_count": 0,
    "test_invoked": true,
    "build_invoked": false,
    "tool_call_count": 42,
    "session_duration_ms": 180000,
    "edit_files": ["src/components/Foo.tsx", "src/utils/bar.ts"]
  }
}
```

### 작업 3: install.sh 업데이트

Stop 훅 등록과 analyzers 디렉토리를 install.sh에 반영한다.

---

## 6. 설계 원칙

- **기존 훅 동작을 깨뜨리지 않는다** — 로깅은 그대로 유지, 분석은 별도 레이어
- **Bash + jq 유지** — 의존성 최소화 원칙 계승
- **메트릭은 프록시** — 완벽한 측정이 아니라 자동 수집 가능한 근사치
- **훅 성능 주의** — Stop 훅이 세션 종료를 지연시키면 안 됨. 분석은 백그라운드 실행 고려

---

## 7. 향후 (이번 작업 범위 아님)

- 기준선 비교 CLI: `seogi compare --baseline 2026-04-07 --current 2026-04-14`
- 프록시 지표 확장: 사용자 턴 수, 미결 문서 생성 여부, 에러 자기해결률
- 주간 트렌드 대시보드
- 하네스 변경 A/B 실험 자동화
