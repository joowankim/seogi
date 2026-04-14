# Seogi

하니스 엔지니어링을 위한 계측 도구 프레임워크.

## Ground Truth

모든 설계 결정과 구현 작업을 시작하기 전에 반드시 `docs/ground-truth.md`를 읽고, 해당 작업이 두 가지 목적에 부합하는지 확인할 것.

## Rust 개발 규칙

Rust 코드를 작성하거나 수정할 때 반드시 아래 스킬을 참조할 것:

- `.claude/skills/rust-patterns/SKILL.md` — 소유권, 에러 핸들링, 패턴 매칭, 모듈 구조 등 idiomatic Rust 패턴
- `.claude/skills/rust-testing/SKILL.md` — TDD 워크플로우, 단위/통합 테스트, 테스트 조직 패턴

### TDD 강제

Rust 코드 구현 시 반드시 TDD flow를 따를 것:
1. RED — 실패하는 테스트를 먼저 작성
2. GREEN — 테스트를 통과하는 최소한의 코드 작성
3. REFACTOR — 테스트가 통과하는 상태에서 코드 개선
