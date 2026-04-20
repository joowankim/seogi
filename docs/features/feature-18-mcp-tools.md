# Feature 18: MCP 도구 구현

## 목적

MCP 서버에 project/status/task 도구 10개를 등록하여, Claude Code 에이전트가 세션 중 태스크를 직접 생성·조회·관리할 수 있게 한다.

ground-truth 기여: 목적 1 (측정 수단 확보). 에이전트가 MCP 도구로 태스크를 생성·전환하면 `TaskEvent`가 자동 기록되어 사이클 타임·처리량 등 태스크 기반 프록시 지표의 원천 데이터가 세션 중 즉시 쌓인다. 또한 MCP 도구 호출 자체가 Claude Code 훅을 통해 `tool_uses`에 기록되므로 `tool_call_count` 지표에도 반영된다.

## 입력

- 사용자 입력: MCP 클라이언트가 `tools/call` JSON-RPC 요청으로 도구명과 파라미터를 전달
- 시스템 입력: `~/.seogi/seogi.db` (SQLite DB)

### 도구별 파라미터

| 도구 | 파라미터 | 필수/선택 | 비고 |
|---|---|---|---|
| `project_create` | `name: string` | 필수 | 프로젝트 이름 |
| | `prefix: string` | 선택 | `ProjectPrefix`. 3글자 대문자. 미지정 시 기존 workflow 규칙(이름 앞 3글자 대문자 변환)으로 자동 생성 |
| | `goal: string` | 필수 | 프로젝트 목표 |
| `project_list` | (없음) | — | |
| `status_create` | `category: string` | 필수 | `StatusCategory` enum. 허용 값: `backlog`, `unstarted`, `started`, `completed`, `canceled` |
| | `name: string` | 필수 | 상태 이름. position은 workflow가 카테고리 내 마지막 위치+1로 자동 할당 |
| `status_list` | (없음) | — | |
| `status_update` | `id: string` | 필수 | 상태 UUID |
| | `name: string` | 필수 | 변경할 이름 |
| `status_delete` | `id: string` | 필수 | 상태 UUID |
| `task_create` | `project: string` | 필수 | 프로젝트 이름 |
| | `title: string` | 필수 | 태스크 제목 |
| | `description: string` | 필수 | 태스크 설명 |
| | `label: string` | 필수 | 허용 값: `feature`, `bug`, `refactor`, `chore`, `docs` |
| `task_list` | `project: string` | 선택 | 프로젝트 이름 필터. 미지정 시 전체 |
| | `status: string` | 선택 | 상태 이름 필터. 미지정 시 전체 |
| | `label: string` | 선택 | Label 필터. 미지정 시 전체 |
| `task_update` | `task_id: string` | 필수 | 태스크 ID (e.g., SEO-1) |
| | `title: string` | 선택 | 변경할 제목 |
| | `description: string` | 선택 | 변경할 설명 |
| | `label: string` | 선택 | 변경할 라벨. 허용 값: `feature`, `bug`, `refactor`, `chore`, `docs`. title/description/label 중 최소 1개 필수 |
| `task_move` | `task_id: string` | 필수 | 태스크 ID |
| | `status: string` | 필수 | 전환할 상태 이름. 기본 시딩 상태 및 커스텀 상태 이름 모두 가능. FSM 규칙 적용 |

## 출력

- 반환값: 각 도구의 MCP `CallToolResult` (JSON 텍스트 콘텐츠)
  - 성공: `is_error: false` + JSON 직렬화된 결과 데이터
  - 실패: `is_error: true` + 에러 메시지 문자열
- 부수효과: DB 변경 (create/update/delete/move 도구)

### 도구별 응답 데이터

| 도구 | 성공 응답 |
|---|---|
| `project_create` | Project JSON: `{ name, prefix, goal }` |
| `project_list` | Project 배열 JSON: `[{ name, prefix, goal }, ...]` |
| `status_create` | Status JSON: `{ id, name, category, position }` |
| `status_list` | Status 배열 JSON: `[{ id, name, category, position }, ...]` |
| `status_update` | `"Updated status {id}"` |
| `status_delete` | `"Deleted status {id}"` |
| `task_create` | Task JSON: `{ id, title, description, label }` |
| `task_list` | TaskListRow 배열 JSON: `[{ id, title, description, label, status_name, project_name, created_at, updated_at }, ...]` |
| `task_update` | `"Updated task {task_id}"` |
| `task_move` | `"Moved task {task_id}: {from} → {to}"` |

## 성공 시나리오

1. Claude Code가 `seogi mcp-server`를 MCP 서버로 구동한다
2. 클라이언트가 `initialize` → `tools/list` 요청을 보낸다
3. 서버가 10개 도구 목록을 응답한다
4. 클라이언트가 `tools/call`로 `task_create`를 호출한다
5. 서버가 `spawn_blocking`으로 sync workflow를 실행하고 결과를 JSON으로 반환한다
6. 클라이언트가 `task_list`로 생성된 태스크를 확인한다

## 실패 시나리오

- **필수 파라미터 누락**: rmcp가 스키마 검증 실패를 자동 응답
- **enum 값 불일치 (category, label)**: rmcp 스키마에는 string으로 정의. 도메인 레벨(`StatusCategory::from_str`, `Label::from_str`)에서 검증하여 `is_error: true` + 에러 메시지 반환
- **미존재 엔티티 (프로젝트, 태스크, 상태)**: `CallToolResult`에 `is_error: true` + 에러 메시지
- **도메인 에러 (FSM 위반, 중복 prefix 등)**: `CallToolResult`에 `is_error: true` + 에러 메시지
- **DB 에러**: `CallToolResult`에 `is_error: true` + 에러 메시지
- **Mutex poison**: `lock()` 실패 시 `CallToolResult`에 `is_error: true` + 에러 메시지. 의도적 재현이 어려워 테스트 제외 — 런타임 안전망으로만 보장
- **spawn_blocking 패닉**: tokio가 JoinError 반환 → MCP 내부 에러 응답. 의도적 재현이 어려워 테스트 제외 — 런타임 안전망으로만 보장

## 제약 조건

- entrypoint 계층(mcp.rs)에서만 변경. workflow/domain/adapter 계층 변경 없음
- sync workflow 호출을 `tokio::task::spawn_blocking`으로 래핑
- rmcp의 `#[tool]` 매크로를 사용하여 도구 등록
- DB 연결(`Connection`)은 MCP 서버 기동 시 한 번 생성, `Arc<Mutex<Connection>>`으로 공유
- 각 도구의 입력 파라미터와 출력 형식이 CLI와 동일한 데이터 제공
- `TaskEvent.session_id`: workflow가 `CLI_SESSION_ID`("CLI")를 하드코딩하므로, MCP 호출 시에도 동일하게 "CLI"로 기록됨 (workflow 변경 없음 제약에 의해)
- `status_create`의 `position` 자동 할당: 기존 workflow(`status::create`)에 이미 구현된 동작 (카테고리 내 max position + 1). 신규 구현 불필요

## 의존하는 기능

- Feature 17 (SEO-1): MCP 서버 부트스트랩 — 완료

---

## QA 목록

### tools/list

1. `tools/list` 요청에 10개 도구가 응답된다
2. 10개 도구 각각의 `inputSchema`에서, 필수 파라미터가 `required` 배열에 포함되고 선택 파라미터는 미포함이다

### project 도구

3. `project_create` 호출 시 프로젝트가 생성되고, 생성된 프로젝트 JSON이 반환된다
4. `project_create`에서 prefix 미지정 시 이름에서 자동 생성된 prefix가 반환된다
5. `project_create`에서 중복 prefix 시 `is_error: true`와 "already exists" 키워드를 포함한 에러 메시지가 반환된다
6. `project_list` 호출 시 전체 프로젝트 배열 JSON이 반환된다

### status 도구

7. `status_create` 호출 시 상태가 생성되고, 생성된 상태 JSON이 반환된다
8. `status_create`에서 잘못된 category 문자열 시 `is_error: true`가 반환된다
9. `status_list` 호출 시 전체 상태 배열 JSON이 반환된다
10. `status_update` 호출 시 상태 이름이 변경되고, 확인 메시지가 반환된다
11. `status_update`에서 존재하지 않는 id 시 `is_error: true`가 반환된다
12. `status_delete` 호출 시 상태가 삭제되고, 확인 메시지가 반환된다
13. `status_delete`에서 태스크가 참조 중인 상태 삭제 시 `is_error: true`와 해당 status id를 포함한 에러 메시지가 반환된다

### task 도구

14. `task_create` 호출 시 태스크가 생성되고, 생성된 태스크 JSON이 반환된다
15. `task_create`에서 미존재 프로젝트명 시 `is_error: true`가 반환된다
16. `task_create`에서 잘못된 label 문자열 시 `is_error: true`가 반환된다
17. `task_list` 호출 시 전체 태스크 배열 JSON이 반환된다
18. `task_list`에서 project 필터 지정 시 해당 프로젝트의 태스크만 반환된다
19. `task_list`에서 status 필터 지정 시 해당 상태의 태스크만 반환된다
20. `task_list`에서 label 필터 지정 시 해당 라벨의 태스크만 반환된다
21. `task_update` 호출 시 태스크가 수정되고, 확인 메시지가 반환된다
22. `task_update`에서 미존재 task_id 시 `is_error: true`가 반환된다
23. `task_update`에서 title/description/label 모두 미지정 시 `is_error: true`가 반환된다
24. `task_move` 호출 시 상태가 전환되고, `"Moved task {id}: {from} → {to}"` 메시지가 반환된다
25. `task_move`에서 미존재 task_id 시 `is_error: true`가 반환된다
26. `task_move`에서 FSM 위반 시 `is_error: true`가 반환된다

### 코드 리뷰 체크리스트

- [ ] 모든 도구가 기존 workflow 함수를 직접 호출한다 (entrypoint에서만 변경)
- [ ] sync workflow 호출이 `spawn_blocking`으로 래핑된다
- [ ] `cargo test` 전체 통과
- [ ] `cargo clippy` 경고 없음

---

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---|---|---|
| 1. tools/list 10개 | E2E | 프로토콜 응답 검증, 프로세스 I/O |
| 2. inputSchema 일치 | E2E | 스키마 구조는 프로토콜 레벨 검증 |
| 3. project_create 성공 | E2E | 전체 흐름 (MCP → workflow → DB) |
| 4. project_create prefix 자동 생성 | E2E | 전체 흐름 |
| 5. project_create 중복 prefix | E2E | 에러 응답 형식 검증 |
| 6. project_list | E2E | 전체 흐름 |
| 7. status_create 성공 | E2E | 전체 흐름 |
| 8. status_create 잘못된 category | E2E | 에러 응답 형식 검증 |
| 9. status_list | E2E | 전체 흐름 |
| 10. status_update 성공 | E2E | 전체 흐름 |
| 11. status_update 미존재 | E2E | 에러 응답 형식 검증 |
| 12. status_delete 성공 | E2E | 전체 흐름 |
| 13. status_delete 참조 중 | E2E | 에러 응답 형식 검증 |
| 14. task_create 성공 | E2E | 전체 흐름 |
| 15. task_create 미존재 프로젝트 | E2E | 에러 응답 형식 검증 |
| 16. task_create 잘못된 label | E2E | 에러 응답 형식 검증 |
| 17. task_list | E2E | 전체 흐름 |
| 18. task_list project 필터 | E2E | 필터 파라미터 전달 검증 |
| 19. task_list status 필터 | E2E | 필터 파라미터 전달 검증 |
| 20. task_list label 필터 | E2E | 필터 파라미터 전달 검증 |
| 21. task_update 성공 | E2E | 전체 흐름 |
| 22. task_update 미존재 ID | E2E | 에러 응답 형식 검증 |
| 23. task_update 옵션 미지정 | E2E | 에러 응답 형식 검증 |
| 24. task_move 성공 | E2E | 전체 흐름 |
| 25. task_move 미존재 ID | E2E | 에러 응답 형식 검증 |
| 26. task_move FSM 위반 | E2E | 에러 응답 형식 검증 |

### E2E 테스트 전략

Feature 17과 동일한 방식: `std::process::Command`로 `seogi mcp-server` 프로세스를 실행하고, stdin/stdout으로 JSON-RPC 메시지를 주고받아 검증한다.

- `tests/mcp_tools_test.rs`에 E2E 테스트 작성
- 각 테스트는 독립적 프로세스 기동 + 임시 DB 사용
- 성공/에러 케이스 모두 `CallToolResult`의 `isError` 필드와 `content[0].text`로 검증
- 도구 간 의존이 있는 케이스 (예: project_create → task_create)는 하나의 테스트에서 순차 호출

### 테스트가 E2E에 집중되는 이유 (분배 원칙 예외)

이 Feature는 분배 원칙("단위로 가능하면 단위로, E2E는 핵심 경로만")의 예외이며, 그 사유는 다음과 같다:

1. **코드 변경이 entrypoint 계층에 한정**: workflow 이하 계층은 이미 기존 단위/통합 테스트로 커버됨
2. **rmcp `#[tool]` 매크로 제약**: 매크로가 핸들러 함수를 MCP 프로토콜 디스패치용 trait impl으로 변환하여, 함수를 직접 호출하는 통합 테스트가 불가능. 반드시 JSON-RPC 프로토콜을 통해 호출해야 함
3. **에러 변환 로직의 단순성**: workflow의 `DomainError` → `CallToolResult(is_error: true)` 변환은 `.map_err()` 한 줄이므로, 별도 헬퍼로 추출하여 단위 테스트를 작성하는 것은 과도한 추상화
4. **단위 테스트 대상 순수 함수 추가 없음**: 새로운 도메인 로직이나 계산 로직이 없음
