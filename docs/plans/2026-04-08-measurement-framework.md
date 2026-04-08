# 하니스 성능 측정 프레임워크 설계

## Ground Truth 참조

- 목적 1: 하니스 성능을 정량적으로 측정하는 수단과 지표를 다수 확보하기
- 목적 2: 하니스 변경 전후의 동등한 업무 효율을 측정하고 보장하기

---

## 레퍼런스

| 출처 | 핵심 차용 |
|---|---|
| Martin Fowler — Harness Engineering | Guides/Sensors 분류 체계 |
| Philipp Schmid — Agent Harness 2026 | Durability(내구성) 개념 |
| Arize — Prompt Learning for Claude Code | LLM 지시 용량 ~150-200개 제한 |
| METR — 무작위 대조 실험 | 주관/객관 괴리 (체감 +24% vs 실제 -19%) |
| ConfFLARE 논문 | 통계적 회귀 판정 공식 |

---

## Phase 1: 지표 체계 확보 (즉시)

### 1-A. 로그 보강

현재 로그에 빠진 데이터를 추가 수집한다.

| 항목 | 변경 대상 | 내용 |
|---|---|---|
| 도구 실패 로그 | 신규: post-tool-failure.sh | PostToolUseFailure 훅 추가. 실패한 도구명, 에러 메시지 기록 |
| Stop reason 상세 | stop.sh | stop_reason을 그대로 기록 (현재도 하고 있음 — 확인 필요) |

### 1-B. 지표 확장

session-summary의 프록시 지표를 Guides/Sensors 분류로 재설계한다.
(session-summary.sh → Python 모듈로 전환, Phase 1에서 수행)

**Sensors 작동 (검증 도구 실행 여부)**
- `test_invoked` — 기존 유지
- `build_invoked` — 기존 유지
- `lint_invoked` — Bash에서 lint/eslint/prettier/ruff 호출 여부
- `typecheck_invoked` — Bash에서 tsc --noEmit/mypy/pyright 호출 여부

**Guides 준수 (규칙대로 행동했는가)**
- `read_before_edit_ratio` — 기존 유지

**비효율 감지 (삽질 징후)**
- `doom_loop_count` — 기존 유지
- `bash_error_rate` — Bash 호출 중 실패(exit code != 0) 비율

**산출물**
- `tool_call_count` — 기존 유지
- `session_duration_ms` — 기존 유지
- `edit_files` — 기존 유지

### 1-C. Python/Typer CLI + pytest 도입

분석기와 CLI를 Python으로 전환한다.

**구조:**

| 레이어 | 언어 | 이유 |
|---|---|---|
| 훅 (hooks/) | bash | 매 호출 실행, 지연 민감 |
| 분석기 | Python | 통계, 집계, 비교 로직 |
| CLI (`seogi`) | Python/Typer | report, compare, changelog |
| 테스트 | pytest | 분석기/CLI 안정성 보장 |

**CLI 명령어:**

```
$ seogi report --from 2026-04-08 --to 2026-04-14 --project locs

기간: 2026-04-08 ~ 2026-04-14 (n=23 세션)
프로젝트: locs

                        평균    중앙값   σ      P25    P75
read_before_edit        3.2    3.0     1.4    2.0    4.0
doom_loop_count         0.8    0.0     1.1    0.0    1.0
test_invoked            34%    —       —      —      —
bash_error_rate         12%    10%     8%     5%     18%
tool_call_count         42     38      15     30     52
session_duration_ms     180k   165k    90k    120k   220k
```

**Stop 훅과의 연결:**
- stop.sh → `python -m seogi.analyzers.session_summary <project> <session_id>` 백그라운드 호출
- 기존 analyzers/session-summary.sh는 Python 모듈로 대체

**테스트 범위:**
- 분석기 로직 (지표 계산, 통계 집계)
- CLI 명령어 (입출력 검증)
- 비교 판정 로직 (ConfFLARE 공식)

---

## Phase 2: 하니스 변경 이력 (1주 후)

### 2-D. Changelog

하니스 변경 시점을 기록하는 CLI.

```
$ seogi changelog add "CLAUDE.md에 Edit 전 Read 강제 규칙 추가"
```

저장: `~/seogi-logs/harness-changelog.jsonl`

```json
{"timestamp":"2026-04-15T09:00:00.000Z","description":"CLAUDE.md에 Edit 전 Read 강제 규칙 추가"}
```

비교 도구가 이 시점을 자동으로 구간 분리에 활용한다.

---

## Phase 3: 비교 도구 (2주 후)

### 3-E. 통계적 회귀 판정

`seogi compare` — ConfFLARE 공식 적용

```
회귀 = |평균_B - 평균_A| >= max(ε × 평균_A, k × σ_A)
       ε = 0.1 (10% 민감도)
       k = 3 (99.7% 신뢰도)
```

```
$ seogi compare --before 2026-04-08:2026-04-14 --after 2026-04-15:2026-04-21

기간 A (04-08 ~ 04-14, n=23) vs B (04-15 ~ 04-21, n=19)

                        A평균   B평균   변화    판정
read_before_edit        3.2    5.1    +59%   개선
doom_loop_count         0.8    0.3    -63%   개선
test_invoked            34%    41%    +7pp   동등
bash_error_rate         12%    9%     -3pp   동등
tool_call_count         42     38     -10%   동등
```

### 3-F. 주관/객관 괴리 검증

비교 결과 출력 시 아래 문구를 항상 포함:

```
⚠ METR 연구: 개발자는 24% 빠르다고 체감했지만 실제로는 19% 느렸음.
  위 데이터가 체감과 일치하는지 확인하세요. 불일치 시 프록시 지표 재검토 필요.
```

---

## 논의 필요 사항

### 논의 1: durability 지표 — Phase 3 이후로 보류

Schmid의 "수백 번 도구 호출에 걸친 지시 준수율"을 측정하려면,
산출물에 대해 lint/typecheck/test 같은 센서를 직접 실행해야 한다.
이를 위해서는 프로젝트별 seogi 설정 파일(`.seogi.json`)이 필요하며,
seogi의 성격이 수동 관찰자에서 능동 검증자로 바뀌는 설계 전환이 수반된다.
Phase 3 이후, 기존 지표만으로 부족하다는 게 확인되면 재검토한다.

### 논의 2: Bash exit code 수집 — 해결됨

PostToolUse는 성공 시에만, PostToolUseFailure는 실패 시에만 실행된다.
따라서 `bash_error_rate` = 실패 횟수 / (성공 + 실패) 로 계산 가능.
Phase 1에서 PostToolUseFailure 훅(`post-tool-failure.sh`)을 추가한다.

### 논의 3: ConfFLARE 공식의 파라미터 — 결정됨

ε=0.1, k=3 기본값으로 시작한다.
기준선 2주 후 실제 σ를 확인하고, 오탐이 많으면 파라미터를 조정한다.

### 논의 4: `seogi` CLI의 형태 — 결정됨

Python/Typer로 CLI를 구현하고 pytest로 안정성을 확보한다.
훅은 bash로 유지 (지연 민감), 분석기와 CLI만 Python으로 전환.
Phase 1의 1-C에서 프로젝트 구조 셋업과 함께 진행한다.

### 논의 5: 프록시 지표의 검증 — Phase 3 이후로 보류

수동 평가는 지속 불가능하므로, 자동화된 방법이 필요하다.
Phase 3 이후 비교 도구를 사용하면서 프록시 지표와 실제 결과의 괴리가 체감되면
그때 검증 방법을 결정한다.
