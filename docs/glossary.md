# Seogi 용어 사전

이 프로젝트 내부에서 특별하게 정의된 용어만 기록한다. 일반 CS/아키텍처 용어는 `docs/conventions.md` 참조.

---

## 값객체

- **Ms:** 밀리초 단위 시간 간격. 도구 호출 소요 시간(`duration`) 등에 사용. `Timestamp`(시각)와 구분하여 시각과 간격의 혼동을 방지.
- **ProjectPrefix:** 프로젝트의 대문자 알파벳 3글자 식별자 newtype. 태스크 ID의 접두사로 사용 (e.g., `"SEO"` → `SEO-1`). 정확히 3글자 대문자 알파벳만 허용하며, 프로젝트 간 중복 불가.
- **SessionId:** Claude Code 세션의 고유 식별자. project, id 등 다른 문자열 필드와 구분.
- **Timestamp:** 밀리초 Unix timestamp. 이벤트/로그의 발생 시각(`timestamp` 컬럼)에만 사용. 엔티티의 `created_at`/`updated_at`은 ISO 8601 TEXT로 표현하며 `Timestamp` 타입을 사용하지 않는다.

## 엔티티

- **Label:** 태스크의 분류 라벨. 5개 고정 값(feature, bug, refactor, chore, docs)을 코드 enum으로 관리.
- **MigratedRecord:** JSONL 마이그레이션 시 `LogEntry`를 `ToolUse` 또는 `ToolFailure`로 변환한 결과.
- **Project:** 태스크를 묶는 관리 단위. name, `Prefix`, goal, next_seq을 포함하며 `projects` 테이블에 대응. next_seq은 태스크 시퀀스 채번에 사용되며 도메인에서 초기값 1로 설정.
- **MigrateSummary:** 마이그레이션 결과 요약. tool_uses, tool_failures, skipped, files 카운터.
- **Status:** 태스크의 상태를 나타내는 엔티티. name, category(`StatusCategory`), position을 포함하며 `statuses` 테이블에 대응. 기본 7개가 스키마 적용 시 시딩되고, 사용자 커스텀 상태 추가/수정/삭제 가능.
- **SystemEvent:** Notification 또는 Stop 훅에서 수집된 시스템 이벤트 기록. `event_type`으로 구분.
- **Task:** 단위 작업. id는 `{ProjectPrefix}-{seq}` 형식(예: SEO-1). title, description, `Label`, `Status`, `Project`를 포함하며 `tasks` 테이블에 대응. 생성 시 초기 상태는 backlog.
- **TaskEvent:** 태스크 상태 변경 이벤트. from_status(nullable), to_status, session_id, timestamp를 포함하며 `task_events` 테이블에 대응. 최초 생성 시 from_status는 NULL.
- **ToolFailure:** 도구 호출 실패 기록. `PostToolUseFailure` 훅에서 수집. `tool_failures` 테이블에 대응.
- **ToolUse:** 도구 호출 성공 기록. `PostToolUse` 훅에서 수집. `tool_uses` 테이블에 대응.

## 프록시 지표

하니스 성능을 간접 측정하는 10개 수치. ground-truth.md의 두 가지 목적에 기여하는 원천 데이터.

- **bash_error_rate:** Bash 도구 실패 비율. `tool_failures`의 Bash 건수 / `tool_uses`의 Bash 건수. Bash 호출이 없으면 0.0.
- **build_invoked:** Bash 명령어에 빌드 도구(webpack, tsc, esbuild 등) 패턴이 존재하는지 여부.
- **doom_loop_count:** 같은 파일을 5회 이상 Edit한 파일의 수. 반복 수정 징후.
- **edit_files:** Edit/Write 도구로 수정한 고유 파일 경로 목록. 중복 제거 후 알파벳 정렬.
- **lint_invoked:** Bash 명령어에 린트 도구(eslint, prettier, ruff 등) 패턴이 존재하는지 여부.
- **read_before_edit_ratio:** 첫 Edit/Write 전 Read/Grep/Glob 호출 수. 이름에 ratio가 있지만 실제로는 횟수(count).
- **session_duration:** 첫 도구 호출부터 마지막 도구 호출까지의 시간 간격. `Ms` 값객체로 표현.
- **test_invoked:** Bash 명령어에 테스트 도구(pytest, jest, vitest 등) 패턴이 존재하는지 여부.
- **tool_call_count:** 세션 내 전체 도구 호출 수.
- **typecheck_invoked:** Bash 명령어에 타입체크 도구(tsc --noEmit, mypy, pyright) 패턴이 존재하는지 여부.

## 태스크 지표

task_events 기반으로 태스크 단위 성과를 측정하는 9개 지표. `domain/task_metrics.rs`의 순수 함수로 계산.

- **triage_time:** 첫 Backlog 카테고리 도착 → 첫 Unstarted 카테고리 도착까지의 시간 간격. 태스크 구체화에 소요된 시간을 나타낸다. `Option<Ms>`.
- **cycle_time:** 첫 Started 카테고리 도착 → 첫 Completed 카테고리 도착까지의 시간 간격. 실제 작업에 소요된 시간. `Option<Ms>`.
- **lead_time:** 태스크 생성 시점(`created_at`) → 첫 Completed 도착까지의 전체 경과 시간. `Option<Ms>`.
- **wait_time:** `lead_time - cycle_time`. 작업 시작 전 대기 시간. `Option<Ms>`, 파생 지표.
- **flow_efficiency:** `cycle_time / lead_time`. 전체 리드 타임 중 실제 작업 비율. 0.0~1.0 범위. `Option<f64>`, 파생 지표.
- **throughput:** 지정 기간 내 Completed 카테고리로 전환된 고유 태스크 수. `u32`.
- **rework_rate:** Completed→Started 전환(재작업)이 발생한 태스크 수 / 전체 완료 태스크 수. `f64`.
- **first_time_done_rate:** 재작업 없이 한 번에 완료된 태스크 비율. `1.0 - rework_rate`과 동치. `f64`.
- **issue_age:** 현재 시각 - 태스크 생성 시점. 미완료(Completed/Canceled 아닌) 태스크에만 적용. `Option<Ms>`.

## 프로젝트 고유 개념

- **CLI_SESSION_ID:** CLI에서 생성한 `TaskEvent`의 `session_id`로 사용되는 도메인 상수. 값은 `"CLI"`.
- **상태 전환 규칙 (FSM):** `StatusCategory` 간 허용된 전환을 정의하는 유한 상태 기계. `can_transition_to`/`allowed_transitions` 순수 함수로 구현. 같은 카테고리 내 커스텀 상태 간에는 자유 전환 허용.
- **안전 실행 (run_safely):** 훅 에러 시 `hook-errors.log`에 기록하고 exit 0으로 종료하는 동작. Claude Code 세션이 훅 에러로 중단되지 않도록 보장.
- **콘텐츠 기반 ID (Content-Based ID):** JSONL 마이그레이션 시 `SHA-256(session_id + timestamp + tool_name)`의 앞 32자 hex로 생성하는 결정론적 ID. 재실행 시 중복 방지.
- **타이밍 파일 (Timing File):** `PreToolUse` 훅이 도구 호출 시작 시각을 기록하는 임시 파일. `PostToolUse` 훅이 읽어 `duration: Ms`를 계산한 뒤 삭제.
- **하니스 (Harness):** Claude Code의 설정과 운영 환경 전체를 가리키는 seogi 고유 용어. CLAUDE.md, 스킬, 훅, 프롬프트 등을 포함.
