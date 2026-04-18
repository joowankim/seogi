# Seogi 용어 사전

이 프로젝트 내부에서 특별하게 정의된 용어만 기록한다. 일반 CS/아키텍처 용어는 `docs/conventions.md` 참조.

---

## 값객체

- **Ms:** 밀리초 단위 시간 간격. 도구 호출 소요 시간(`duration`) 등에 사용. `Timestamp`(시각)와 구분하여 시각과 간격의 혼동을 방지.
- **SessionId:** Claude Code 세션의 고유 식별자. project, id 등 다른 문자열 필드와 구분.
- **Timestamp:** 밀리초 Unix timestamp. 이벤트 발생 시각을 표현. `Ms`(간격)와 구분.

## 엔티티

- **MigratedRecord:** JSONL 마이그레이션 시 `LogEntry`를 `ToolUse` 또는 `ToolFailure`로 변환한 결과.
- **MigrateSummary:** 마이그레이션 결과 요약. tool_uses, tool_failures, skipped, files 카운터.
- **SessionMetrics:** 세션의 프록시 지표 10개를 담는 타입. `calculate()` 순수 함수로 산출.
- **Stats:** 수치 배열의 통계 요약. mean, median, stddev, P25, P75.
- **SystemEvent:** Notification 또는 Stop 훅에서 수집된 시스템 이벤트 기록. `event_type`으로 구분.
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

## 프로젝트 고유 개념

- **안전 실행 (run_safely):** 훅 에러 시 `hook-errors.log`에 기록하고 exit 0으로 종료하는 동작. Claude Code 세션이 훅 에러로 중단되지 않도록 보장.
- **콘텐츠 기반 ID (Content-Based ID):** JSONL 마이그레이션 시 `SHA-256(session_id + timestamp + tool_name)`의 앞 32자 hex로 생성하는 결정론적 ID. 재실행 시 중복 방지.
- **타이밍 파일 (Timing File):** `PreToolUse` 훅이 도구 호출 시작 시각을 기록하는 임시 파일. `PostToolUse` 훅이 읽어 `duration: Ms`를 계산한 뒤 삭제.
- **하니스 (Harness):** Claude Code의 설정과 운영 환경 전체를 가리키는 seogi 고유 용어. CLAUDE.md, 스킬, 훅, 프롬프트 등을 포함.
