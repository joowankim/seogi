# Feature 14: Task 생성/조회

## 목적

태스크를 생성하고 조회하는 CLI 명령어를 제공한다. 태스크는 프로젝트에 속하며, `{prefix}-{seq}` 형식의 ID로 식별된다. 생성 시 초기 상태는 backlog이고, `task_events`에 생성 이벤트가 기록된다.

## 입력

### `seogi task create`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `--project` | String | O | 프로젝트 이름 |
| `--title` | String | O | 태스크 제목 |
| `--description` | String | O | 태스크 설명 |
| `--label` | String | O | 라벨 (feature, bug, refactor, chore, docs) |

### `seogi task list`

| 인자 | 타입 | 필수 | 설명 |
|------|------|------|------|
| `--project` | String | X | 프로젝트 이름으로 필터 |
| `--status` | String | X | 상태 이름으로 필터 |
| `--label` | String | X | 라벨로 필터 |
| `--json` | flag | X | JSON 형식 출력 |

## 출력

### `seogi task create` 성공

```
Created task SEO-1 "태스크 제목"
```

### `seogi task list` 테이블 출력

```
ID       TITLE                STATUS           LABEL
SEO-1    태스크 제목           backlog          feature
SEO-2    두번째 태스크         todo             bug
```

### `seogi task list --json`

```json
[
  {
    "id": "SEO-1",
    "title": "태스크 제목",
    "description": "설명",
    "label": "feature",
    "status_name": "backlog",
    "project_name": "Seogi",
    "created_at": "2026-04-18T12:00:00+00:00",
    "updated_at": "2026-04-18T12:00:00+00:00"
  }
]
```

## 시나리오

### 성공

1. **태스크 생성**: 유효한 프로젝트, 제목, 설명, 라벨로 태스크를 생성하면 `{prefix}-{next_seq}` 형식의 ID가 할당되고, `next_seq`이 1 증가하며, 초기 상태 backlog가 설정되고, `task_events`에 생성 이벤트(`from_status: NULL`, `to_status: backlog`)가 기록된다.
2. **태스크 목록 조회**: 전체 태스크를 테이블 형식으로 출력한다.
3. **프로젝트 필터**: `--project`로 특정 프로젝트의 태스크만 조회한다.
4. **상태 필터**: `--status`로 특정 상태의 태스크만 조회한다.
5. **라벨 필터**: `--label`로 특정 라벨의 태스크만 조회한다.
6. **복합 필터**: 여러 필터를 동시에 적용한다.
7. **JSON 출력**: `--json` 플래그로 JSON 형식 출력한다.

### 실패

1. **존재하지 않는 프로젝트**: `--project`에 없는 프로젝트 이름 → 에러.
2. **유효하지 않은 라벨**: `--label`에 정의되지 않은 값 → 에러.
3. **빈 제목**: `--title`이 빈 문자열 → 에러.
4. **빈 설명**: `--description`이 빈 문자열 → 에러.
5. **backlog 상태 부재**: DB에 backlog 카테고리 상태가 없는 경우 → 에러.

## 제약 조건

- **Task ID 형식**: `{ProjectPrefix}-{seq}`. seq는 프로젝트의 `next_seq`에서 채번하고 원자적으로 증가시킨다.
- **Label**: 5개 고정 값 (`feature`, `bug`, `refactor`, `chore`, `docs`). 코드 enum으로 관리.
- **초기 상태**: backlog 카테고리의 첫 번째 상태 (position 순). 시딩된 "backlog" 상태가 사용된다.
- **CLI_SESSION_ID**: CLI에서 생성한 이벤트의 `session_id`로 사용되는 도메인 상수 (`"CLI"`).
- **task_events 기록**: 생성 시 `from_status: NULL`, `to_status`는 초기 상태의 이름.
- **next_seq 원자적 채번**: workflow에서 트랜잭션으로 `next_seq` 읽기 → 태스크 저장 → `next_seq` 업데이트를 보장한다.
- **list 필터**: `--status`는 상태 이름으로 매칭. `--project`는 프로젝트 이름으로 매칭. `--label`은 라벨 값으로 매칭.
- **JSON 출력**: 목록 조회 시 `status_name`과 `project_name`을 포함하여 ID 대신 읽기 쉬운 이름을 제공한다.

## 의존성

- Feature 12 (Project CRUD): 프로젝트 존재 확인, `next_seq` 채번.
- Feature 13 (Status CRUD): 초기 상태 backlog 할당.

---

## QA 목록

### Domain (Unit)

| # | 검증 항목 |
|---|----------|
| Q1 | `Label` enum 5개 variant 존재 (feature, bug, refactor, chore, docs) |
| Q2 | `Label::from_str` 유효값 → 해당 variant |
| Q3 | `Label::from_str` 무효값 → `DomainError::Validation` |
| Q4 | `Label::as_str` 소문자 문자열 반환 |
| Q5 | `Task::new` 유효 입력 → id가 `{prefix}-{seq}` 형식 |
| Q6 | `Task::new` 유효 입력 → 필드값 보존 |
| Q7 | `Task::new` 빈 title → `DomainError::Validation` |
| Q8 | `Task::new` 빈 description → `DomainError::Validation` |
| Q9 | `TaskEvent::new` 필드값 보존 |
| Q10 | `CLI_SESSION_ID` 상수값 `"CLI"` |

### Adapter (Integration)

| # | 검증 항목 |
|---|----------|
| Q11 | `task_repo::save` 후 DB에서 조회 시 필드 일치 |
| Q12 | `task_repo::list_all` 필터 없이 전체 반환, created_at DESC 정렬 |
| Q13 | `task_repo::list_all` project 필터 적용 |
| Q14 | `task_repo::list_all` status 필터 적용 |
| Q15 | `task_repo::list_all` label 필터 적용 |
| Q16 | `task_repo::list_all` 복합 필터 적용 |
| Q17 | `task_event_repo::save` 후 DB에서 조회 시 필드 일치 |
| Q18 | `project_repo::increment_next_seq` 호출 후 next_seq 1 증가 |

### Workflow (Integration)

| # | 검증 항목 |
|---|----------|
| Q19 | `create_task` 성공 시 Task 반환, next_seq 증가, task_events 1건 |
| Q20 | `create_task` 존재하지 않는 프로젝트 → 에러 |
| Q21 | `create_task` 무효 라벨 → 에러 |
| Q22 | `create_task` backlog 상태 부재 → 에러 |
| Q23 | `list_tasks` 필터 적용 결과 반환 |

### E2E (CLI)

| # | 검증 항목 |
|---|----------|
| Q24 | `seogi task create` 성공 메시지에 task ID 포함 |
| Q25 | `seogi task create` 존재하지 않는 프로젝트 → 에러 출력 |
| Q26 | `seogi task list` 테이블 형식 출력 |
| Q27 | `seogi task list --json` JSON 형식 출력 |
| Q28 | `seogi task list --project "..." --label feature` 필터링 |

---

## Test Pyramid

### Unit (Domain) — 10건

```
test_label_variant_count                    → Q1
test_label_from_str_valid                   → Q2
test_label_from_str_invalid                 → Q3
test_label_as_str                           → Q4
test_task_new_id_format                     → Q5
test_task_new_fields                        → Q6
test_task_new_empty_title                   → Q7
test_task_new_empty_description             → Q8
test_task_event_new_fields                  → Q9
test_cli_session_id_constant                → Q10
```

### Integration (Adapter + Workflow) — 13건

```
test_task_repo_save_and_find                → Q11
test_task_repo_list_all                     → Q12
test_task_repo_list_filter_project          → Q13
test_task_repo_list_filter_status           → Q14
test_task_repo_list_filter_label            → Q15
test_task_repo_list_filter_combined         → Q16
test_task_event_repo_save                   → Q17
test_project_repo_increment_next_seq        → Q18
test_create_task_success                    → Q19
test_create_task_unknown_project            → Q20
test_create_task_invalid_label              → Q21
test_create_task_no_backlog_status          → Q22
test_list_tasks_with_filter                 → Q23
```

### E2E (CLI) — 5건

```
test_task_create_success                    → Q24
test_task_create_unknown_project            → Q25
test_task_list_table                        → Q26
test_task_list_json                         → Q27
test_task_list_filtered                     → Q28
```

---

## 완료 체크리스트

- [ ] Feature 문서 작성 완료
- [ ] QA 목록 28건 작성 완료
- [ ] Test Pyramid 28건 (Unit 10 + Integration 13 + E2E 5)
- [ ] 사용자 승인

승인일: 2026-04-18
