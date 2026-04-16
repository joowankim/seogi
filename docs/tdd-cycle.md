# TDD 사이클

seogi 프로젝트의 기본 개발 워크플로우. 모든 기능 구현은 이 사이클을 따른다.

---

## 기본 원칙

| 항목 | 선택 |
|---|---|
| 접근 방향 | Hybrid (핵심 도메인은 Inside-out, 경계는 Outside-in) |
| 테스트 레벨 | Test Pyramid (단위 > 통합 > E2E) |
| Mock 전략 | Classicist (mock 대신 실제 인메모리 SQLite 사용) |
| 커밋 주기 | Safe point (모든 테스트 녹색일 때만 커밋) |
| 브랜치 커버리지 | 100% (`cargo llvm-cov` 측정) |

**용어 정의:**
- **Inside-out**: 순수 도메인(엔티티, Value Object, Domain Service)부터 시작해서 바깥으로
- **Outside-in**: 외부 인터페이스(CLI/훅)부터 시작해서 안쪽으로
- **Classicist**: 실제 구현체 사용 (예: 인메모리 SQLite 실제 Repository)
- **Mockist**: 협력자를 mock으로 대체

---

## 절차

```
[Feature 시작]
├─ 0. 기획
│   ├─ feature 문서 작성 (docs/features/feature-XX-<name>.md)
│   ├─ QA 목록 작성 (acceptance criteria)
│   └─ Test Pyramid 설계 (E2E / 통합 / 단위 분배)
├─ 1. E2E 테스트 선작성 (RED)
│   ├─ tests/ 디렉토리에 E2E 테스트 작성
│   ├─ 모든 QA 항목에 대응
│   └─ 모든 테스트 실패 상태에서 시작
├─ 2. 내부 구현
│   ├─ Inside-out: domain 단위 테스트 + 구현
│   ├─ Outside-in: application handler 테스트 + 구현
│   ├─ Entrypoint: E2E로 커버
│   └─ 각 유닛마다 RED → GREEN → REFACTOR → safe point commit
├─ 3. 통합 확인
│   ├─ Feature의 모든 E2E 테스트 녹색
│   └─ 브랜치 커버리지 100%
├─ 4. 리팩토링
│   ├─ 녹색 유지하며 개선
│   └─ 깨지면 즉시 revert
└─ 5. Feature 완료 커밋
```

---

## 0. 기획

구현 시작 전 세 가지 산출물을 만든다.

### 0-1. Feature 문서

`docs/features/feature-XX-<name>.md` 경로에 작성.

**포함할 내용:**
- **목적**: 이 기능이 왜 필요한가 (ground-truth 연결)
- **입력**: 사용자/시스템이 제공하는 데이터
- **출력**: 기능이 생성하는 데이터 또는 부수효과
- **성공 시나리오**: 정상 동작 흐름
- **실패 시나리오**: 에러 조건과 처리 방식
- **제약 조건**: 성능, 보안, 호환성 등

### 0-2. QA 목록

**acceptance criteria**. 각 항목은 테스트 가능한 검증 가능 명제여야 한다.

```
예시 (post-tool 훅):
✓ 유효한 JSON stdin 전달 시 tool_uses 테이블에 한 행 추가
✓ tool.name == "Bash"인 경우 tool.input.command이 로그에 보존
✓ duration_ms는 pre-tool 기록과의 차이로 계산
✓ session_id가 누락된 입력은 "unknown"으로 저장
✓ 잘못된 JSON stdin은 에러 반환 + DB 미변경
```

### 0-3. Test Pyramid 설계

각 QA 항목이 어느 테스트 레벨에서 검증될지 분배:

| 레벨 | 대상 | 비중 |
|---|---|---|
| 단위 (unit) | 순수 도메인 로직 (Value Object 검증, 지표 계산) | 많음 |
| 통합 (integration) | 여러 계층 조합 (Handler + Repository) | 중간 |
| E2E | 바이너리 호출, stdin/stdout, 실제 DB 파일 | 적음 |

**원칙:**
- 피드백 속도가 빠른 단위 테스트를 많이
- E2E는 QA 목록에 대응하는 핵심 경로만
- 인프라 없이 재현 가능한 설정 (임시 디렉토리 + SQLite 파일)

---

## 1. E2E 테스트 선작성 (RED)

기획된 QA 목록을 기반으로 E2E 테스트를 먼저 작성한다. 구현이 없으므로 모두 실패한다.

**위치:** `cli/tests/feature_XX_test.rs`

**환경 격리:**
- `tempfile::tempdir()`로 임시 디렉토리
- SQLite 파일은 해당 디렉토리 안에 생성
- 테스트 종료 시 자동 정리 (testcontainers 불필요)

**바이너리 호출 방식:**
- `std::process::Command::cargo_bin("seogi")` (assert_cmd 크레이트)
- 또는 `seogi::run()` 같은 lib 진입점 직접 호출

**예시:**
```rust
#[test]
fn post_tool_hook_saves_tool_use_to_db() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","tool_name":"Bash","tool_input":{"command":"ls"},"cwd":"/test"}"#;

    // Act
    let output = Command::cargo_bin("seogi").unwrap()
        .args(["hook", "post-tool"])
        .env("SEOGI_DB_PATH", &db_path)
        .write_stdin(input)
        .output().unwrap();

    // Assert
    assert!(output.status.success());
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
}
```

---

## 2. 내부 구현 (Hybrid)

### Inside-out 파트 (핵심 도메인)

순수한 부분부터 시작. mock 없이 실제 값으로 단위 테스트.

**순서:**
1. Value Object (`Prefix`, `TaskId` 등)
2. Entity (`ToolUse`, `SessionMetrics` 등)
3. 순수 도메인 함수 (`metrics::calculate` 등)

**예시:**
```rust
// domain/metrics.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_empty_session_returns_zero_metrics() {
        let metrics = metrics::calculate(&[]);
        assert_eq!(metrics.tool_call_count, 0);
    }
}
```

### Outside-in 파트 (경계)

Workflow 함수는 실제 I/O를 호출하므로, **실제 인메모리 SQLite 연결**을 사용한다 (Classicist).

**예시:**
```rust
// workflow/analyze.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_saves_metrics_to_db() {
        // Classicist: 실제 인메모리 SQLite 사용
        let mut conn = Connection::open_in_memory().unwrap();
        adapter::db::apply_schema(&mut conn);

        // 실제 adapter 함수로 데이터 준비
        log_repo::save_tool_use(&mut conn, &sample_tool_use()).unwrap();

        // workflow 실행
        let result = workflow::analyze::run(&mut conn, "session-1");

        assert!(result.is_ok());
        // 실제 DB 상태 검증
        let saved = metrics_repo::find_latest(&conn, "session-1").unwrap();
        assert_eq!(saved, result.unwrap());
    }
}
```

**Entrypoint**는 E2E 테스트로 이미 커버되므로 별도 단위 테스트 최소화.

### 각 유닛의 사이클

```
1. RED    — 실패하는 테스트 작성
2. GREEN  — 통과하는 최소 구현
3. REFACTOR — 녹색 유지하며 개선
4. COMMIT  — 전체 테스트 녹색일 때만 safe point 커밋
```

---

## 3. 통합 확인

구현이 끝나면:

1. Feature의 모든 E2E 테스트 녹색 확인
2. `cargo llvm-cov --branch`로 브랜치 커버리지 100% 확인
3. 누락된 브랜치가 있으면 단위/통합 테스트 추가

**브랜치 커버리지 예외:**
- `unreachable!()`, `panic!()` 같은 불가능 상태
- `#[derive]` 매크로 생성 코드
- main 함수 등은 E2E로 커버

---

## 4. 리팩토링

녹색 상태에서 코드 개선. **테스트가 깨지면 즉시 revert**하여 safe point로 복귀.

**개선 대상:**
- 중복 제거
- 명명 개선
- 함수 분할 (50줄 초과 시)
- 의존성 주입 명료화
- 성능 최적화 (측정 후)

**원칙:**
- 새 기능 추가 금지 (리팩토링과 기능 추가를 구분)
- 테스트 변경 최소화 (테스트도 안정성 지표)

---

## 5. Feature 완료 커밋

최종 커밋에 다음을 포함:
- 기획 문서 (`docs/features/feature-XX-<name>.md`)
- 모든 테스트
- 구현 코드

**커밋 메시지:**
```
feat(XX): <짧은 요약>

충족된 QA:
- [x] 유효한 JSON stdin → tool_uses 추가
- [x] tool.input.command 보존
- [x] duration_ms 계산
- [x] session_id 누락 시 "unknown"
- [x] 잘못된 JSON → 에러 + DB 미변경

브랜치 커버리지: 100%
```

---

## 개발 중 피드백 루프

### bacon (파일 저장 시 자동 테스트)

개발 중에는 `bacon`으로 파일 저장마다 자동 테스트 실행:

```bash
bacon test   # 기본 테스트 모드
bacon        # 기본 모드 (clippy)
```

RED → GREEN 사이클을 빠르게 돌리기 위함.

### prek (pre-commit 훅)

커밋 시점 최종 방어선:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

---

## 도구 목록

| 도구 | 역할 |
|---|---|
| `cargo test` | 테스트 실행 |
| `cargo llvm-cov --branch` | 브랜치 커버리지 측정 |
| `bacon` | 파일 저장 시 자동 테스트 |
| `prek` | pre-commit 훅 (fmt, clippy, test) |
| `assert_cmd` | 바이너리 E2E 테스트 |
| `tempfile` | 임시 디렉토리/파일 |
| `rusqlite` + `:memory:` | 인메모리 SQLite (테스트 격리) |
