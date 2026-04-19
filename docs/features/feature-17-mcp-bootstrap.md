# Feature 17: MCP 서버 부트스트랩

## 목적

rmcp 크레이트를 사용하여 `seogi mcp-server` 서브커맨드를 추가한다. stdio transport로 MCP 서버를 구동하고, 도구 없이 서버가 정상 기동/종료되는 것을 확인한다.

ground-truth 기여: 목적 1 (측정 수단 확보). MCP 서버가 동작해야 Claude Code 에이전트가 세션 중 태스크를 직접 생성/관리할 수 있고, 이를 통해 자동 계측 범위가 넓어진다.

## 입력

- 사용자 입력: `seogi mcp-server` CLI 명령어 실행
- 시스템 입력: stdin (MCP JSON-RPC 메시지)

## 출력

- stdout: MCP JSON-RPC 응답 메시지
- 부수효과: 없음 (도구 미등록 상태이므로 DB 접근 없음)

## 성공 시나리오

1. 사용자가 `seogi mcp-server`를 실행한다
2. 서버가 stdio transport로 기동된다
3. 클라이언트가 `initialize` 요청을 보낸다
4. 서버가 서버 정보(name: "seogi", version)와 빈 capabilities를 응답한다
5. 클라이언트가 `tools/list` 요청을 보낸다
6. 서버가 빈 도구 목록 `[]`을 응답한다
7. 클라이언트가 `shutdown` 요청을 보낸다
8. 서버가 정상 종료된다

## 실패 시나리오

- **잘못된 JSON-RPC 메시지**: rmcp가 프로토콜 에러를 자동 응답
- **stdin EOF (클라이언트 연결 끊김)**: 서버 정상 종료
- **tokio 런타임 초기화 실패**: anyhow 에러 메시지 출력 후 exit 1

## 제약 조건

- rmcp 크레이트 사용 (features = ["server", "transport-stdio"])
- tokio 런타임 필요 (async 예외: MCP 서버는 rmcp가 async를 요구하므로 conventions.md의 "sync 지향" 예외)
- workflow/domain/adapter 계층 변경 없음 — entrypoint에만 추가
- 도구 등록 없음 (SEO-2에서 구현)

## 의존하는 기능

없음 (Phase 3 첫 번째 Feature)

---

## QA 목록

1. `seogi mcp-server` 실행 시 MCP 서버가 stdio transport로 기동된다
2. `initialize` 요청에 서버 정보(name: "seogi", version: Cargo.toml 버전)를 응답한다
3. `tools/list` 요청에 빈 배열을 응답한다
4. stdin EOF 시 서버가 정상 종료된다 (exit 0)
5. `Commands` enum에 `McpServer` variant가 추가된다
6. `entrypoint/mcp/` 모듈이 생성된다
7. Cargo.toml에 rmcp(server, transport-stdio)와 tokio 의존성이 추가된다
8. workflow/domain/adapter 계층에 변경이 없다
9. `cargo test` 전체 통과
10. `cargo clippy` 경고 없음

---

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---|---|---|
| 1. stdio 기동 | E2E | 바이너리 실행 + 프로세스 I/O 검증 |
| 2. initialize 응답 | E2E | 프로토콜 핸드셰이크는 전체 흐름 검증 |
| 3. tools/list 빈 배열 | E2E | 프로토콜 응답 검증 |
| 4. stdin EOF 정상 종료 | E2E | 프로세스 종료 코드 검증 |
| 5. McpServer variant | 컴파일 | clap derive — 컴파일 시 검증 |
| 6. mcp 모듈 존재 | 컴파일 | mod 선언 — 컴파일 시 검증 |
| 7. 의존성 추가 | 컴파일 | Cargo.toml — 빌드 시 검증 |
| 8. 계층 미변경 | 코드 리뷰 | diff 확인 |
| 9. cargo test 통과 | CI | 기존 테스트 회귀 없음 |
| 10. clippy 통과 | CI | 린트 검증 |

### E2E 테스트 전략

MCP 프로토콜은 JSON-RPC over stdio이므로, E2E 테스트에서 자식 프로세스로 `seogi mcp-server`를 실행하고 stdin/stdout으로 JSON-RPC 메시지를 주고받아 검증한다.

- `tests/mcp_bootstrap.rs`에 통합 E2E 테스트 작성
- `std::process::Command`로 바이너리 실행, stdin에 JSON-RPC 쓰기, stdout에서 응답 읽기
- 각 테스트는 독립적으로 프로세스를 기동/종료
