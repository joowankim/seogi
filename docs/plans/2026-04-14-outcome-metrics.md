# 아웃컴 지표 확장 계획

상위 문서: [measurement-framework.md](./2026-04-08-measurement-framework.md)

---

## 배경

현재 seogi의 지표 10개는 모두 프로세스 프록시(에이전트가 뭘 했는가)이며,
아웃컴(결과가 얼마나 좋은가)을 측정하지 못한다.

이 계획은 토큰 소비량과 git 산출물을 연결해서 **비용 대비 산출물 효율**을 측정한다.

---

## 추가 지표

### 토큰 데이터 (transcript에서 수집)

| 지표 | 타입 | 출처 |
|---|---|---|
| `total_input_tokens` | u64 | transcript `message.usage.input_tokens` 합산 |
| `total_output_tokens` | u64 | transcript `message.usage.output_tokens` 합산 |
| `total_cache_read_tokens` | u64 | transcript `message.usage.cache_read_input_tokens` 합산 |
| `total_cache_creation_tokens` | u64 | transcript `message.usage.cache_creation_input_tokens` 합산 |

### git 데이터 (Stop 시점에 수집)

| 지표 | 타입 | 수집 방법 |
|---|---|---|
| `commit_count` | u32 | `git log <start_sha>..HEAD --oneline \| wc -l` |
| `lines_added` | u32 | `git diff --stat <start_sha>..HEAD`에서 추출 |
| `lines_deleted` | u32 | `git diff --stat <start_sha>..HEAD`에서 추출 |

### 비율 지표 (위 데이터 조합)

| 지표 | 타입 | 공식 |
|---|---|---|
| `tokens_per_commit` | f64 | (input + output + cache_read + cache_creation) / commit_count |
| `tokens_per_line_changed` | f64 | 전체 토큰 / (lines_added + lines_deleted) |

---

## 데이터 수집 흐름

### 1. 세션 시작 시 HEAD 기록 (pre-tool.sh)

```
pre-tool.sh:
  임시 파일 /tmp/seogi/{SESSION_ID}_head_sha 가 없으면
    → cwd에서 git rev-parse HEAD 실행
    → 결과를 임시 파일에 기록
  이미 있으면 무시 (세션 내 첫 호출만)
```

### 2. 세션 종료 시 데이터 전달 (stop.sh)

```
stop.sh:
  1. 기존 stop 로그 기록
  2. /tmp/seogi/{SESSION_ID}_head_sha 읽기
  3. transcript_path는 Stop 훅의 stdin에서 추출
  4. seogi analyze 호출:
     seogi analyze <project> <session_id> \
       --transcript <transcript_path> \
       --start-sha <start_sha>
```

### 3. seogi analyze 확장

```
seogi analyze <project> <session_id> [--transcript <path>] [--start-sha <sha>]
```

- `--transcript` 있으면: transcript JSONL에서 토큰 사용량 파싱
- `--start-sha` 있으면: `git log <sha>..HEAD` + `git diff --stat <sha>..HEAD`로 커밋/라인 수집
- 둘 다 없으면: 기존 동작 (프로세스 지표만 계산)
- 비율 지표는 분모가 0이면 null

---

## metrics JSONL 스키마 변경

기존 10개 지표에 추가:

```json
{
  "metrics": {
    "read_before_edit_ratio": 5,
    "doom_loop_count": 0,
    "test_invoked": true,
    "build_invoked": false,
    "lint_invoked": false,
    "typecheck_invoked": false,
    "tool_call_count": 42,
    "session_duration_ms": 180000,
    "edit_files": ["a.rs"],
    "bash_error_rate": 0.1,
    "total_input_tokens": 1376,
    "total_output_tokens": 3353,
    "total_cache_read_tokens": 752567,
    "total_cache_creation_tokens": 297682,
    "commit_count": 3,
    "lines_added": 120,
    "lines_deleted": 45,
    "tokens_per_commit": 351659,
    "tokens_per_line_changed": 6393
  }
}
```

신규 필드는 모두 Option으로 처리 (없으면 null).
기존 데이터와 호환 유지.

---

## 구현 순서

### Step 1: pre-tool.sh에 HEAD SHA 기록 추가

구현:
- 세션 내 첫 호출 시 `/tmp/seogi/{SESSION_ID}_head_sha`에 `git rev-parse HEAD` 저장

테스트:
- 첫 호출 시 파일 생성 확인
- 두 번째 호출 시 덮어쓰지 않음 확인
- git 레포가 아닌 디렉토리에서 실패하지 않음 확인

### Step 2: transcript 파서 구현

구현:
- `app/src/transcript_reader.rs` — transcript JSONL에서 토큰 사용량 합산

테스트:
- 정상 transcript 파싱
- usage 필드 없는 엔트리 건너뛰기
- 빈 파일 처리

### Step 3: git 데이터 수집 구현

구현:
- `app/src/git_stats.rs` — start_sha 기반 커밋 수, 라인 변경 수 계산

테스트:
- 커밋 있는 경우
- 커밋 없는 경우 (start_sha == HEAD)
- start_sha가 유효하지 않은 경우

### Step 4: seogi analyze 확장

구현:
- `--transcript`, `--start-sha` 옵션 추가
- 토큰 + git + 비율 지표를 metrics에 포함
- SessionMetrics 모델 확장

테스트:
- 옵션 있을 때 신규 지표 포함 확인
- 옵션 없을 때 기존 동작 유지 확인
- 비율 계산 (분모 0이면 null)

### Step 5: stop.sh 수정 + report 출력 업데이트

구현:
- stop.sh: transcript_path, start_sha를 seogi analyze에 전달
- report: 토큰/git/비율 지표 출력 추가

테스트:
- stop 훅 → analyze 호출 체인 동작 확인
- report에 신규 지표 표시 확인

---

## 논의 필요 사항

없음.
