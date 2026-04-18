---
name: seogi-tdd-cycle
description: seogi 프로젝트의 TDD 구현 워크플로우. 기획 승인 완료 후 Feature를 구현할 때 참조. E2E 선작성 → Inside-out/Outside-in 하이브리드 구현 → 100% 브랜치 커버리지. 전체 참조는 docs/tdd-cycle.md.
---

# Seogi TDD 사이클 (Quick Reference)

전체 가이드: `docs/tdd-cycle.md` 참조

## 전제 조건

**기획 승인 완료 상태**여야 함:
- Feature 문서 작성 완료
- QA 목록 작성 완료
- Test Pyramid 설계 완료
- 사용자 승인 완료

승인 없으면 `seogi-feature-planning` 스킬로 돌아갈 것.

## 기본 원칙

| 항목 | 선택 |
|---|---|
| 접근 | Hybrid (도메인은 Inside-out, 경계는 Outside-in) |
| 테스트 레벨 | Test Pyramid |
| Mock 전략 | Classicist (실제 인메모리 SQLite) |
| 커밋 주기 | Safe point (모든 테스트 녹색일 때만) |
| 브랜치 커버리지 | 100% (`cargo llvm-cov --branch`) |

## 5단계 절차

```
1. E2E 테스트 선작성 (RED)
   ↓
2. 내부 구현
   ├─ Inside-out: domain 단위 테스트 + 구현
   ├─ Outside-in: workflow 테스트 + 구현
   └─ 각 유닛 RED → GREEN → REFACTOR → safe point commit
   ↓
3. 통합 확인 (E2E 녹색 + 커버리지 100%)
   ↓
4. 리팩토링 (녹색 유지)
   ↓
5. /seogi-code-review 실행 → MUST FIX 항목 수정
   ↓
6. Feature 완료 커밋
```

## 1. E2E 테스트 선작성 (RED)

- 위치: `cli/tests/feature_XX_test.rs`
- 모든 QA 항목에 대응
- 격리: `tempfile::tempdir()` + 임시 SQLite 파일
- 모든 테스트 실패 상태에서 시작

```rust
#[test]
fn post_tool_hook_saves_tool_use_to_db() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("seogi.db");
    let input = r#"{"session_id":"s1","tool_name":"Bash",...}"#;

    // Act
    let output = Command::cargo_bin("seogi").unwrap()
        .args(["hook", "post-tool"])
        .env("SEOGI_DB_PATH", &db_path)
        .write_stdin(input)
        .output().unwrap();

    // Assert
    assert!(output.status.success());
    // DB 상태 검증
}
```

## 2. 내부 구현 (Hybrid)

### Inside-out (핵심 도메인)

순수 함수/Value Object부터. mock 불필요.

**순서:**
1. Value Object (`Prefix`, `TaskId`)
2. Entity (`ToolUse`, `SessionMetrics`)
3. 순수 도메인 함수 (`metrics::calculate`)

```rust
// domain/metrics.rs
#[cfg(test)]
mod tests {
    #[test]
    fn calculate_empty_session_returns_zero() {
        let metrics = metrics::calculate(&[]);
        assert_eq!(metrics.tool_call_count, 0);
    }
}
```

### Outside-in (경계)

workflow 함수를 테스트. **실제 인메모리 SQLite 사용** (Classicist).

```rust
// workflow/analyze.rs
#[cfg(test)]
mod tests {
    #[test]
    fn run_saves_metrics() {
        let mut conn = Connection::open_in_memory().unwrap();
        adapter::db::apply_schema(&mut conn);

        // 실제 adapter 함수로 데이터 준비
        log_repo::save_tool_use(&mut conn, &sample_tool_use()).unwrap();

        // workflow 실행
        let result = workflow::analyze::run(&mut conn, "session-1");
        assert!(result.is_ok());
    }
}
```

**Entrypoint**는 E2E로 커버 (단위 테스트 최소화).

### 각 유닛의 사이클

```
RED    → 실패 테스트 작성
GREEN  → 통과하는 최소 구현
REFACTOR → 녹색 유지하며 개선
COMMIT   → 전체 테스트 녹색일 때만 safe point
```

## 3. 통합 확인

- Feature의 모든 E2E 녹색
- `cargo llvm-cov --branch` 100% 확인
- 누락 브랜치 → 단위/통합 테스트 추가

**커버리지 예외:** `unreachable!()`, `panic!()`, derive 매크로 등.

## 4. 리팩토링

- 녹색 유지하며 개선
- 깨지면 즉시 revert
- 새 기능 추가 금지 (리팩토링과 기능 추가 분리)

## 5. 코드 리뷰

커밋 전에 `/seogi-code-review` 스킬을 실행한다.

- MUST FIX 항목이 있으면 수정 후 재실행
- 모든 MUST FIX 해소 후 커밋 진행

## 6. Feature 완료 커밋

```
feat(XX): <요약>

충족된 QA:
- [x] 유효한 JSON stdin → tool_uses 추가
- [x] session_id 누락 시 "unknown"
- [x] 훅 실행 시간 < 50ms
...
브랜치 커버리지: 100%
```

## 개발 중 피드백 루프

- `bacon test` — 파일 저장 시 자동 테스트
- `prek` — pre-commit 훅 (fmt, clippy, test)

## 도구

| 용도 | 도구 |
|---|---|
| 테스트 | `cargo test` |
| 커버리지 | `cargo llvm-cov --branch` |
| 자동 테스트 | `bacon test` |
| E2E 바이너리 | `assert_cmd` 크레이트 |
| 임시 파일 | `tempfile` 크레이트 |

## 체크리스트

**시작 시:**
- [ ] 기획 승인 완료 (필수)

**구현 중:**
- [ ] E2E 테스트 선작성
- [ ] 모든 safe point에서만 커밋
- [ ] 각 유닛 RED → GREEN → REFACTOR 준수

**완료 시:**
- [ ] 모든 E2E 녹색
- [ ] 브랜치 커버리지 100%
- [ ] `/seogi-code-review` MUST FIX 없음
- [ ] 커밋 메시지에 QA 목록 명시

## 자주 실수하는 것

- ❌ 기획 없이 구현 시작
- ❌ E2E 테스트 없이 구현 시작
- ❌ mock 사용 (Classicist 원칙 위반)
- ❌ 빨간 상태에서 커밋
- ❌ Feature 중간에 새 기능 추가
- ❌ 리팩토링 중 기능 변경
