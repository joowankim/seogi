# Feature 19: Claude Code MCP 등록

## 목적

install.sh/uninstall.sh에 MCP 서버 설정을 추가/제거하고, README와 CLAUDE.md를 업데이트하여 seogi의 설치·사용·제거 경험을 완성한다.

ground-truth 기여: 목적 1 (측정 수단 확보). MCP 서버가 Claude Code에 등록되어야 에이전트가 세션 중 태스크를 자동으로 관리할 수 있고, 이를 통해 `TaskEvent` 기반 프록시 지표 수집이 가능해진다.

## 입력

- 사용자 입력: `./install.sh` 또는 `./uninstall.sh` 실행
- 시스템 입력: `~/.claude.json` (MCP 서버 설정), `~/.claude/settings.json` (훅 설정), 파일 시스템

## 출력

- 반환값:
  - install.sh: 성공 시 exit 0, jq 실패 또는 유효하지 않은 JSON 시 exit 1
  - uninstall.sh: 성공 시 exit 0 (파일 미존재 포함)
- stdout: 기존 install.sh/uninstall.sh의 진행 메시지 유지, MCP 등록 단계 메시지 추가
- 부수효과:
  - install.sh: `~/.claude.json`에 seogi MCP 서버 설정 추가
  - uninstall.sh: `~/.claude.json`에서 seogi MCP 서버 설정 제거
  - README.md 생성
  - CLAUDE.md 워크플로우 섹션 업데이트

### install.sh에 추가되는 MCP 설정

`~/.claude.json`에 다음 설정이 추가/병합된다:

```json
{
  "mcpServers": {
    "seogi": {
      "command": "seogi",
      "args": ["mcp-server"]
    }
  }
}
```

### uninstall.sh에서 제거되는 MCP 설정

`~/.claude.json`에서 `mcpServers.seogi` 키를 제거한다. `mcpServers`에 다른 서버가 없으면 `mcpServers` 키도 제거한다.

## 성공 시나리오

1. 사용자가 `./install.sh`를 실행한다
2. cargo install로 seogi 바이너리가 설치된다 (기존)
3. `~/.claude/settings.json`에 훅이 등록된다 (기존)
4. `~/.claude.json`에 seogi MCP 서버 설정이 추가된다 (신규)
5. Claude Code를 시작하면 seogi MCP 서버가 자동으로 연결된다
6. 사용자가 `./uninstall.sh`를 실행한다
7. 훅 설정이 제거된다 (기존)
8. MCP 서버 설정이 제거된다 (신규)
9. seogi 디렉토리가 삭제된다 (기존)

## 실패 시나리오

- **`~/.claude.json` 미존재 시**: install.sh가 새 파일을 생성하여 MCP 설정 작성. exit 0
- **`~/.claude.json`에 다른 MCP 서버가 이미 존재 시**: 기존 서버를 유지하고 seogi만 추가/제거. exit 0
- **`~/.claude.json`이 유효하지 않은 JSON일 때**: jq가 에러를 발생시키고 `set -e`에 의해 install.sh가 exit 1로 종료. MCP 설정 변경 없음
- **jq 미설치 시**: 기존 install.sh와 동일하게 jq 의존. `set -e`에 의해 exit 1로 종료
- **`~/.claude.json` 쓰기 권한 없을 때**: jq 출력을 파일에 쓸 수 없으므로 `set -e`에 의해 exit 1로 종료. MCP 설정 변경 없음
- **uninstall.sh 시 `~/.claude.json`이 유효하지 않은 JSON일 때**: jq가 에러를 발생시키고 `set -e`에 의해 exit 1로 종료. 파일 변경 없음
- **uninstall.sh 시 `mcpServers`에 seogi 키가 없을 때**: 에러 없이 exit 0. 파일 변경 없음
- **install.sh 재실행 시**: 멱등성 보장 — 기존 seogi 설정을 덮어쓰기. exit 0

## 제약 조건

- 기존 훅 등록 로직과 MCP 등록이 공존 (install.sh 한 번 실행으로 모두 설정)
- uninstall → install 사이클이 정상 동작
- `~/.claude.json`의 기존 데이터(다른 MCP 서버 등)를 보존
- README.md 내용: 프로젝트 소개, 설치, CLI 레퍼런스, MCP 사용법, 제거 방법
- CLAUDE.md: 기능 구현 워크플로우에서 MCP 도구 사용 안내 추가

## 의존하는 기능

- Feature 17 (SEO-1): MCP 서버 부트스트랩 — 완료
- Feature 18 (SEO-2): MCP 도구 구현 — 완료

---

## QA 목록

### install.sh

1. `install.sh` 실행 후 `~/.claude.json`에 `mcpServers.seogi` 설정이 존재한다
2. `mcpServers.seogi.command`가 `"seogi"`이고 `args`가 `["mcp-server"]`이다
3. `~/.claude.json` 미존재 시 install.sh가 새 파일을 생성하고 MCP 설정을 작성한다
4. `~/.claude.json`에 다른 MCP 서버가 있을 때 install.sh 실행 후 기존 서버가 보존된다
5. install.sh 재실행 시 seogi MCP 설정이 중복되지 않는다 (멱등성)
6. 기존 훅 등록도 정상 동작한다 (MCP 등록과 공존)

### uninstall.sh

7. `uninstall.sh` 실행 후 `~/.claude.json`에서 `mcpServers.seogi`가 제거된다
8. `~/.claude.json`에 다른 MCP 서버가 있을 때 uninstall.sh 실행 후 기존 서버가 보존된다
9. `mcpServers`에 seogi만 있었을 때 uninstall 후 `mcpServers` 키가 제거된다
10. `~/.claude.json` 미존재 시 uninstall.sh가 에러 없이 종료된다

### install → uninstall 사이클

11. install → uninstall → install 사이클 후 MCP 설정이 정상 존재한다

### README.md

12. README.md가 프로젝트 소개 섹션을 포함한다
13. README.md가 설치 방법(install.sh) 섹션을 포함한다
14. README.md가 CLI 명령어 레퍼런스 섹션을 포함한다
15. README.md가 MCP 서버 사용법 섹션을 포함한다
16. README.md가 제거 방법(uninstall.sh) 섹션을 포함한다

### CLAUDE.md

17. CLAUDE.md 워크플로우 섹션에 MCP 도구 사용 안내가 포함된다

### 코드 리뷰 체크리스트

- [ ] `cargo test` 전체 통과
- [ ] `prek` pre-commit 훅 통과

---

## Test Pyramid

| QA 항목 | 레벨 | 이유 |
|---|---|---|
| 1. install.sh MCP 설정 존재 | E2E | 셸 스크립트 실행 + 파일 상태 검증 |
| 2. command/args 값 확인 | E2E | 파일 내용 검증 |
| 3. 파일 미존재 시 생성 | E2E | 셸 스크립트 분기 검증 |
| 4. 기존 MCP 서버 보존 | E2E | 파일 병합 로직 검증 |
| 5. 멱등성 | E2E | 셸 스크립트 재실행 검증 |
| 6. 훅 공존 | E2E | install.sh 전체 흐름 |
| 7. uninstall MCP 제거 | E2E | 셸 스크립트 실행 + 파일 상태 검증 |
| 8. uninstall 기존 서버 보존 | E2E | 파일 필터링 로직 검증 |
| 9. mcpServers 키 제거 | E2E | jq 로직 검증 |
| 10. 파일 미존재 시 에러 없음 | E2E | 셸 스크립트 분기 검증 |
| 11. install-uninstall 사이클 | E2E | 전체 흐름 |
| 12-16. README 섹션 | 수동 검증 | 문서 내용은 자동 테스트 부적합 |
| 17. CLAUDE.md 업데이트 | 수동 검증 | 문서 내용은 자동 테스트 부적합 |

### 테스트가 E2E에 집중되는 이유 (분배 원칙 예외)

이 Feature의 변경 대상은 셸 스크립트(install.sh, uninstall.sh)와 문서(README.md, CLAUDE.md)이다. Rust 코드 변경이 없으므로 단위/통합 테스트 대상이 없다. 셸 스크립트의 동작은 실제 파일 시스템에서 실행해야만 검증할 수 있으므로 E2E가 유일한 자동 테스트 수단이다.

### E2E 테스트 전략

- bash 테스트 스크립트(`tests/install_test.sh`)로 작성
- 임시 HOME 디렉토리를 생성하여 `~/.claude.json`, `~/.claude/settings.json` 격리
- `HOME` 환경변수를 임시 디렉토리로 설정하여 install.sh/uninstall.sh 실행
- jq로 결과 파일의 JSON 구조 검증
- cargo install은 건너뛰고 MCP/훅 등록 로직만 테스트 (바이너리는 이미 빌드됨)
