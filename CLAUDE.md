# Seogi

하니스 엔지니어링을 위한 계측 도구 프레임워크.

## Ground Truth

모든 설계 결정과 구현 작업을 시작하기 전에 반드시 `docs/ground-truth.md`를 읽고, 해당 작업이 두 가지 목적에 부합하는지 확인할 것.

## Rust 개발

Rust 코드를 작성하거나 수정할 때 반드시 아래 스킬을 참조할 것:

- `.claude/skills/seogi-rust-convention/SKILL.md` — 이 프로젝트의 Rust 컨벤션 (함수형 3계층, ROP, 네이밍)
- `.claude/skills/rust-patterns/SKILL.md` — idiomatic Rust 패턴 (소유권, 에러 핸들링 등)
- `.claude/skills/rust-testing/SKILL.md` — Rust 테스트 패턴

전체 컨벤션 참조: `docs/conventions.md`

## 도메인 용어

도메인 용어는 `docs/glossary.md`를 참조할 것. 기획 문서/코드에서 glossary에 정의된 표준 용어를 사용한다. 새 도메인 용어가 등장하면 `/manage-glossary`로 먼저 등록 후 사용.

## 기능 구현 워크플로우

작업 시작 전 `seogi task list`로 태스크를 확인하고, 태스크의 description을 읽어 작업 내용을 파악할 것.

1. **태스크 구체화**: 기획 문서(`docs/features/`) 작성 후 사용자 승인 → `seogi task move <id> todo`
   - `.claude/skills/seogi-feature-planning/SKILL.md` 참조
2. **구현 시작**: `seogi task move <id> in_progress`
   - `.claude/skills/seogi-tdd-cycle/SKILL.md` 참조
3. **TDD 사이클**으로 구현
4. **PR 생성**: `seogi task move <id> in_review`
5. **머지 완료**: `seogi task move <id> done`

- 진행 불가 시: `seogi task move <id> blocked`
- 취소 시: `seogi task move <id> canceled`

기획 단계를 건너뛰고 구현에 들어가지 말 것. 사용자 승인 없이 코드 작성 금지.

## Git Workflow

기능 구현은 반드시 **워크트리**에서 진행할 것.

- **생성**: `git worktree add -b feature/<name> .worktrees/<name> origin/main`
- **정리**: 머지 후 `git worktree remove <path>` + `git branch -d feature/<name>`
- **금지**: main 디렉토리에서 `git checkout`으로 브랜치 전환 금지
