---
name: seogi-feature-planning
description: seogi 프로젝트의 기능 기획 워크플로우. 새 기능을 시작하거나 구현 전 기획 문서를 작성할 때 참조. Feature 문서, QA 목록, Test Pyramid 설계. 기획은 사용자 승인 필요. 전체 참조는 docs/feature-planning.md.
---

# Seogi Feature 기획 (Quick Reference)

전체 가이드: `docs/feature-planning.md` 참조

## 중요 원칙

**기획은 구현 전 완료해야 하는 독립 단계**.
**사용자 승인 없이 구현에 들어가지 말 것**.
**Feature 문서 작성 시 `docs/glossary.md`의 표준 용어를 사용할 것**. glossary에 없는 새 도메인 용어가 등장하면 `/manage-glossary`로 먼저 등록 후 사용.

## 절차

```
1. Feature 문서 작성
   ↓
2. QA 목록 작성
   ↓
3. Test Pyramid 설계
   ↓
4. /seogi-planning-review 실행 → FAIL 항목 수정
   ↓
5. 사용자 검토 및 승인
   ↓
[TDD 사이클로 진입] → seogi-tdd-cycle 스킬
```

## 1. Feature 문서

경로: `docs/features/feature-XX-<name>.md`

### 필수 섹션

```markdown
# Feature XX: <이름>

## 목적
ground-truth.md의 두 가지 목적 중 어느 쪽에 기여하는지 명시.

## 입력
- 사용자 입력 (CLI 인자, stdin)
- 시스템 입력 (DB 상태, 파일 시스템)

## 출력
- 반환값
- 부수효과 (DB 변경, 파일 쓰기)

## 성공 시나리오
정상 동작 흐름

## 실패 시나리오
각 에러 조건과 처리 방식

## 제약 조건
- 성능 (훅이면 < 50ms 등)
- 호환성
- 보안

## 의존하는 기능
이미 구현된 Feature 중 의존하는 것들
```

## 2. QA 목록

**acceptance criteria**. 각 항목은 **테스트 가능한 검증 가능 명제**.

### 좋은 예
```
✓ 유효한 JSON stdin → tool_uses 테이블에 한 행 추가
✓ tool.name == "Bash"일 때 tool.input.command 보존
✓ session_id 누락 시 "unknown" 값으로 저장
✓ 훅 실행 시간 < 50ms
```

### 나쁜 예 (모호함)
```
✗ 로그가 잘 저장되어야 한다
✗ 에러가 나면 적절히 처리되어야 한다
```

### 원칙
- 각 항목이 독립적으로 검증 가능
- 구체적이고 측정 가능
- 성공/실패 조건 모두 포함

## 3. Test Pyramid 설계

각 QA 항목을 어느 레벨에서 검증할지 분배:

| 레벨 | 대상 | 비중 | 속도 |
|---|---|---|---|
| 단위 | 순수 함수, Value Object | 많음 | <1ms |
| 통합 | workflow + adapter 조합 | 중간 | <100ms |
| E2E | 바이너리 호출, 실제 DB 파일 | 적음 | 100ms~ |

### 분배 원칙
- 단위로 가능하면 단위로 (빠른 피드백)
- E2E는 핵심 경로만
- 중복 테스트 회피

### 예시

| QA 항목 | 레벨 | 이유 |
|---|---|---|
| JSON → DB 저장 | E2E | 전체 흐름 |
| command 보존 | 통합 | domain+adapter |
| duration 계산 | 단위 | 순수 계산 |
| 잘못된 JSON 처리 | E2E | 바이너리 에러 처리 |

## 4. 기획 리뷰

기획 문서 작성 후, 사용자에게 제출하기 전에 `/seogi-planning-review` 스킬을 실행한다.

- FAIL 항목이 있으면 수정 후 재실행
- 모든 항목 PASS 후 사용자에게 검토 요청

## 5. 사용자 승인

리뷰 통과 후 사용자에게 검토 요청:

- 목적이 ground-truth에 부합?
- QA 목록이 충분히 커버?
- Test Pyramid 분배가 합리적?
- 제약 조건이 현실적?

**승인 후에만 구현 착수**.

## 산출물 체크리스트

- [ ] `docs/features/feature-XX-<name>.md` 작성
- [ ] 목적, 입력/출력, 시나리오, 제약 명시
- [ ] QA 목록의 각 항목이 테스트 가능한 명제
- [ ] Test Pyramid 분배표 작성
- [ ] 의존 Feature 순서 명확
- [ ] `/seogi-planning-review` 통과
- [ ] 사용자 승인 완료

모두 충족 시 `seogi-tdd-cycle` 스킬로 넘어간다.
