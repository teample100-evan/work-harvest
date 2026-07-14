# Work Harvest

Work Harvest는 Codex·Claude Code 같은 작업 에이전트와 진행한 업무를 로컬에 구조화해 기록하고, 그 기록을 기능·업무 단위로 묶어 주간 업무 보고와 성과 자료로 활용하기 위한 도구다.

긴 에이전트 세션을 억지로 짧게 나누지 않으면서도 다음을 가능하게 하는 것이 목표다.

- 업무일 또는 의미 있는 단계마다 체크포인트 기록
- 여러 날짜·세션에 걸친 작업을 하나의 업무 항목으로 연결
- 새 세션에서 필요한 현재 상태와 결정만 빠르게 인수인계
- Git·테스트·PR·이슈 등 검증 가능한 근거 보존
- 기록에 근거한 주간 보고서와 업무 성과 작성

## 현재 상태

현재 구현된 범위:

- 업무 항목 생성·목록·상세 조회
- 날짜별 체크포인트 생성과 마지막 체크포인트 조회
- 구조화된 인수인계 context 갱신
- JSON 원본에서 사람이 읽는 Markdown 자동 생성
- JSON Schema 기반 데이터 검증
- Codex `record-work` Skill
- 체크포인트 기반 성과 노트 Markdown 초안 생성
- 임시 데이터 저장소를 이용한 CLI 통합 테스트
- macOS용 Tauri 읽기 전용 데스크톱 대시보드
- 데이터 폴더 선택·복구와 외부 파일 변경 자동 반영
- 데스크톱 업무 검색·상태 필터·Context 상세와 체크포인트 타임라인
- 체크포인트 활동·결정·검증·근거·Git 기준점 전체 상세
- Finder 업무 표시와 기본 앱으로 Context·체크포인트 Markdown 열기
- Rust Core에 내장된 Draft 2020-12 전체 JSON Schema 검증
- 창을 닫아도 유지되는 macOS 메뉴바 상주 실행
- 메뉴바 최근 업무·마지막 기록과 창 위치·크기 복구
- 명시적으로 켜는 새 체크포인트·검증 오류 알림
- Node CLI와 데스크톱 Core가 공유하는 advisory lock·revision·rollback 쓰기 계층

아직 구현되지 않은 범위:

- 주간·월간 보고서 생성
- 자동 활동 이벤트 수집과 누락 감지
- Claude Code Skill·Hook
- 여러 프로젝트의 자동 taxonomy 분류
- 데스크톱 앱의 쓰기 편집 화면

## 핵심 개념

Work Harvest는 세션과 업무 기록을 같은 단위로 취급하지 않는다.

| 단위 | 의미 | 나누는 기준 |
| --- | --- | --- |
| 프로젝트 | 제품 또는 코드베이스 | 저장소·제품 경계 |
| 업무 항목 | 보고 가능한 기능·업무 목표 | 업무 목적과 성과 경계 |
| 세션 | 하나의 결과물을 만드는 에이전트 작업 맥락 | 결과물과 완료 조건 |
| 체크포인트 | 마지막 기록 이후의 진행 내용 | 업무일 종료·단계 완료·세션 전환 |
| 보고 기간 | 기록을 집계하는 기간 | 주·월·분기·반기 |

같은 결과물을 작업한다면 세션을 며칠 동안 유지할 수 있다. 대신 업무일 종료 시 체크포인트를 남겨 날짜 경계를 보존한다. 같은 기능이라도 새로운 결과물을 만들기 시작하면 인수인계 context를 갱신하고 새 세션을 사용한다.

자세한 운영 기준은 [작업 방법론](./docs/work-method.md)을 참고한다.

## 저장 구조

```text
work-harvest/
├── work-items/
│   └── <work_item_id>/
│       ├── work-item.json       # 업무 메타데이터 원본
│       ├── context.json         # 현재 인수인계 상태 원본
│       └── context.md           # 사람이 읽는 인수인계 문서
├── records/
│   └── YYYY/MM/DD/
│       ├── <checkpoint_id>.json # 체크포인트 원본
│       └── <checkpoint_id>.md   # 사람이 읽는 체크포인트
├── reports/
│   └── performance-notes/       # 체크포인트에서 생성한 성과 노트 초안
├── schemas/                     # JSON Schema
├── templates/                   # Markdown 템플릿
├── adapters/codex/record-work/  # Codex Skill
├── src/                         # CLI 구현
└── test/                        # 통합 테스트
```

JSON을 검증 가능한 원본으로 사용하고 Markdown을 파생 문서로 생성한다. 체크포인트 JSON은 append-only이며 기존 기록을 수정하지 않는다. 오류를 고쳐야 할 때는 정정 체크포인트를 추가한다.

## 요구 사항

- Node.js 24 이상
- pnpm 11 이상
- macOS, Linux 또는 Node.js를 실행할 수 있는 로컬 환경
- CLI 쓰기와 데스크톱 앱 개발 시 Rust stable
- 데스크톱 앱 개발 시 macOS Xcode Command Line Tools

## 설치

```bash
git clone git@github.com:teample100-evan/work-harvest.git
cd work-harvest
pnpm install
pnpm wh --help
```

개발 저장소에서는 `pnpm wh`로 CLI를 실행한다. 패키지의 `bin`을 실행 경로에 설치한 환경에서는 `wh`로 직접 실행할 수 있다.

Node CLI는 입력 파싱과 출력 호환성을 유지하되 실제 파일 commit은 Rust write helper에 위임한다. 개발 저장소에서는 최신 debug helper가 있으면 재사용하고, 없거나 Core 소스보다 오래됐으면 Cargo가 자동으로 빌드한다. 별도 배포한 helper를 사용하려면 `WORK_HARVEST_WRITE_HELPER`에 실행 파일 경로를 지정한다.

## 데스크톱 앱

현재 데스크톱 앱은 macOS Apple Silicon을 우선 지원하는 Tauri v2 기반 읽기 전용 내부 알파다. 사용자가 선택한 데이터 폴더를 기억하고 업무 검색·상태 필터, 현재 context와 체크포인트 전체 상세를 제공한다. Finder에서 업무 위치를 확인하거나 Context·체크포인트 Markdown을 기본 앱으로 열 수 있다. Rust Core는 저장소의 공통 JSON Schema를 앱에 내장해 전체 문서와 파일 관계를 검사하며, Codex나 CLI가 파일을 바꾸면 화면과 메뉴바를 자동으로 갱신한다. 창을 닫아도 메뉴바에서 감시를 유지하고, 사용자가 알림을 켜면 새 체크포인트와 새 검증 오류만 한 번씩 알린다.

```bash
pnpm desktop:dev
pnpm desktop:build
pnpm check:all
```

- `desktop:dev`: 개발용 Tauri 앱 실행
- `desktop:build`: `.app`과 DMG 생성
- `check:all`: 기존 Node CLI, React 프론트엔드와 Rust workspace 전체 검증

읽기 전용 내부 알파인 M1과 M2 상시 실행 경험 1차 구현, 변경 업무 단위 증분 인덱스는 완료됐다. M2의 실제 24시간 watcher soak를 병행하면서 M3의 복구 가능한 쓰기 기반, Node 호환 업무 항목 Core API와 기존 Node CLI의 잠금 호환까지 구현했다. 따라서 다음 구현 단위는 Tauri 쓰기 명령과 저장 전 diff를 포함한 편집 화면이다. 구체적인 범위와 데이터 무결성 전략은 [데스크톱 앱 구현 계획](./docs/desktop-app-plan.md), [Tauri 도입 결정](./docs/adr/0001-tauri-desktop-app.md)과 [복구 가능한 쓰기 결정](./docs/adr/0002-recoverable-local-write-transactions.md)에 정리되어 있다.

## 데이터 위치

CLI는 다음 우선순위로 데이터 저장 위치를 정한다.

1. `--root <path>`
2. `WORK_HARVEST_HOME`
3. 현재 작업 디렉터리

Codex `record-work` Skill은 기본적으로 `~/work-records`를 사용해 업무 기록과 도구 저장소를 분리한다. 터미널이나 다른 에이전트에서도 같은 위치를 사용하려면 다음과 같이 설정한다.

```bash
export WORK_HARVEST_HOME="$HOME/work-records"
```

Codex Skill 래퍼가 CLI checkout을 자동으로 찾지 못하는 경우에만 별도로 지정한다.

```bash
export WORK_HARVEST_CLI_HOME="$HOME/Desktop/projects/work-harvest"
```

`WORK_HARVEST_HOME`은 기록 데이터 위치, `WORK_HARVEST_CLI_HOME`은 CLI 코드 위치다.

## CLI 사용법

### 업무 항목 만들기

에이전트 연동에 적합하도록 JSON 또는 YAML 파일과 stdin을 지원한다.

```bash
pnpm wh work-item create --input work-item.yaml
pnpm wh work-item create --input - --json
```

최소 입력:

```yaml
id: AUTH-142
project_id: jajak-front
title: 인증 시스템 개선
objective: 토큰 만료 시 요청을 안전하게 재시도한다.
desired_outcomes:
  - 인증 갱신 동작을 자동화된 테스트로 검증한다.
classification:
  initiative_id: authentication
  work_types:
    - implementation
    - testing
  tags:
    - auth
context:
  current_state: 인증 갱신 테스트를 시작하기 전이다.
  next_steps:
    - 기본 성공 경로 테스트 작성
```

생성 결과:

```text
work-items/AUTH-142/work-item.json
work-items/AUTH-142/context.json
work-items/AUTH-142/context.md
```

### 기존 업무 찾기

```bash
pnpm wh work-item list
pnpm wh work-item list --project jajak-front --status in_progress
pnpm wh work-item show AUTH-142 --json
```

`work-item show`는 업무 메타데이터, 현재 context와 마지막 체크포인트를 함께 반환한다. 새 세션에서 업무 맥락을 읽을 때 사용하는 기본 명령이다.

### 체크포인트 기록하기

```bash
pnpm wh checkpoint capture --input checkpoint.yaml --json
```

예시:

```yaml
work_item_id: AUTH-142
kind: progress
source:
  agent: codex
  surface: desktop
  session_ref: null
  task_title: 인증 테스트 코드 작성
title: 인증 갱신 기본 테스트 작성
summary: 토큰 갱신과 요청 재시도의 기본 성공 경로를 검증했다.
activities:
  - refresh token 갱신 성공 테스트를 추가했다.
verifications:
  - type: test
    description: 인증 갱신 기본 성공 경로 테스트
    status: passed
    command: pnpm test auth
    evidence_refs:
      - tests/auth/refresh-token.test.ts
outcomes:
  - description: 갱신 후 원 요청이 정상적으로 재시도되는 동작을 검증했다.
    impact: null
    evidence_refs:
      - tests/auth/refresh-token.test.ts
next_steps:
  - 동시 요청 테스트 작성
evidence:
  files:
    - tests/auth/refresh-token.test.ts
  commands:
    - pnpm test auth
context_update:
  current_state: 기본 성공 경로를 검증했고 동시 요청 테스트가 남아 있다.
  verification:
    completed:
      - 인증 갱신 기본 성공 경로 테스트 통과
    pending:
      - 동시 요청 테스트
  next_steps:
    - 동시 요청 테스트 작성
```

`context_update`는 전달한 필드만 교체한다. 배열 필드를 전달할 때는 현재 유효한 전체 목록을 전달하며, 빈 배열은 기존 목록을 비운다는 의미다.

### 마지막 기록 확인하기

```bash
pnpm wh checkpoint last --work-item AUTH-142 --json
```

에이전트는 이 결과를 이용해 “지난 체크포인트 이후”의 기록 범위를 결정한다.

### 성과 노트 초안 만들기

업무 항목에 연결된 체크포인트와 현재 context를 바탕으로, 성과 노트 공통 템플릿의 Markdown 초안을 생성한다.

```bash
pnpm wh report performance-note --work-item AUTH-142
```

기본 생성 위치는 다음과 같다.

```text
reports/performance-notes/AUTH-142-20260713.md
```

원하는 파일명이나 분류 경로가 있으면 데이터 루트 내부의 Markdown 경로를 지정한다.

```bash
pnpm wh report performance-note \
  --work-item AUTH-142 \
  --output reports/performance-notes/2026-q3-auth-improvement.md
```

생성된 성과 노트는 사람이 완성하는 문서다. 체크포인트에서 확인 가능한 작업·검증·결정·근거는 채우고, 근거가 부족한 정량 성과·배포 결과·사용자 영향은 `미확인`으로 남긴다. 작업이 작으면 필요 없는 섹션을 삭제해도 된다. 동일 경로의 기존 노트는 덮어쓰지 않는다.

### 전체 데이터 검증하기

```bash
pnpm wh validate
pnpm wh validate --include-examples --json
```

검증 항목:

- 업무 항목·context·체크포인트 JSON Schema
- 중복 ID
- 업무와 체크포인트의 프로젝트 관계
- context JSON·Markdown 존재 여부와 식별자
- 마지막·정정·연관 체크포인트 참조
- 체크포인트 날짜별 저장 경로
- 체크포인트 JSON·Markdown 식별자

상세 명령은 [CLI 문서](./docs/cli.md), 데이터 구조는 [데이터 모델](./docs/data-model.md)을 참고한다.

## Codex Skill

Skill 원본은 [`adapters/codex/record-work`](./adapters/codex/record-work)에 있다.

개인 Skill 경로에 연결하려면:

```bash
ln -s "$(pwd)/adapters/codex/record-work" "$HOME/.codex/skills/record-work"
```

Codex가 Skill 목록을 다시 읽도록 새 작업을 시작한 후 다음처럼 사용할 수 있다.

```text
$record-work를 사용해 오늘 작업은 여기까지 기록해줘.
```

명시적인 `$record-work` 없이 다음 표현도 트리거 대상이다.

```text
오늘 작업은 여기까지 기록해줘
현재 단계까지 체크포인트를 기록해줘
지난 체크포인트 이후 작업을 기록해줘
새 세션용 인수인계를 갱신해줘
이 업무를 완료 처리하고 최종 기록을 작성해줘
```

Skill 실행 순서:

```text
work-item list
→ work-item show
→ checkpoint last
→ 현재 세션과 저장소 근거 확인
→ checkpoint capture
→ context 갱신
→ validate
```

Skill은 기존 업무와 일치하는지 먼저 확인하고, 여러 후보가 비슷할 때만 사용자에게 선택을 요청한다. 테스트를 실행하지 않았다면 통과했다고 기록하지 않으며, 측정되지 않은 영향은 추정하지 않는다.

## 사용 시나리오

Work Harvest는 코드 작업뿐 아니라 문서, 기획, 회의, 연구와 운영 업무를 기록할 수 있다. 브랜치나 Codex 세션이 달라도 같은 보고 목표라면 하나의 업무 항목을 유지하고, 한 세션에서 독립된 결과물을 여러 개 만들었다면 업무 항목을 분리한다.

### 브랜치마다 별도 세션을 사용하는 경우

```text
이 브랜치에서 진행한 인증 API 문서 작업을 기존 AUTH-142 업무에 기록해줘.
```

### 코드 변경 없는 문서·운영 작업

```text
오늘 작성한 장애 대응 정책 문서를 완료 업무로 기록해줘.
```

저장소나 브랜치 없이도 문서, URL, 결정 또는 리뷰를 근거로 기록할 수 있다.

### 작업 후 별도 세션에서 사후 기록

```text
지난주 월요일부터 수요일까지 배포 가이드를 정리했어. 지금 말한 내용으로 기록해줘.
```

이 경우 `backfill` 체크포인트로 저장한다. 기록 시각과 실제 작업 기간을 분리하고, 사용자 설명에 기반한 내용은 독립 검증된 사실과 구분한다.

### 한 세션에서 여러 독립 업무를 수행한 경우

```text
인증 버그 수정과 배포 가이드 작성은 별도 업무로 나눠서 기록해줘.
```

더 자세한 판단 기준과 10가지 예시는 [사용 시나리오](./docs/usage-scenarios.md)를 참고한다.

## 일반적인 작업 흐름

### 긴 세션을 계속 사용하는 경우

세션은 그대로 유지하고 업무일 종료 시 다음과 같이 요청한다.

```text
오늘 작업은 여기까지 기록해줘. 세션은 계속 유지할 거야.
```

같은 `work_item_id`와 세션 context를 유지하면서 날짜별 체크포인트가 추가된다.

### 새 세션으로 넘기는 경우

```text
새 세션에서 이어갈 수 있도록 현재 단계까지 기록하고 인수인계를 갱신해줘.
```

Skill은 체크포인트를 먼저 생성한 다음 `context.json`과 `context.md`를 현재 상태로 갱신한다. 새 세션은 `work-item show <id>` 결과와 실제 코드를 다시 확인하고 시작한다.

### 업무를 완료하는 경우

```text
이 업무를 완료 처리하고 최종 기록을 작성해줘.
```

최종 체크포인트는 하나 이상의 확인된 결과를 가져야 한다. 완료 근거가 부족하면 Skill은 진행 중 상태를 유지한다.

## 개발과 검증

```bash
pnpm install
pnpm test
pnpm run check
```

`pnpm run check`는 다음을 실행한다.

- CLI JavaScript 문법 검사
- 임시 데이터 저장소 기반 통합 테스트
- 예시 업무·context·체크포인트 전체 검증

현재 통합 테스트는 다음 흐름을 확인한다.

- 업무 생성 → 체크포인트 → 조회 → 검증
- 중복 업무 생성 거부
- 내용 없는 체크포인트 거부
- 확인된 결과 없는 완료 기록 거부
- 최종 기록 시 업무 상태와 context 갱신
- 체크포인트 Markdown 생성
- 코드 없는 사후 작업의 기록 시각·실제 작업 기간·사용자 근거 분리

## 정확성과 보안

- 전체 세션 transcript를 기본 기록에 저장하지 않는다.
- 전체 소스 diff, 환경 변수, 비밀값과 민감한 명령 출력을 저장하지 않는다.
- 실행하지 않은 테스트와 측정하지 않은 성과를 만들어내지 않는다.
- 날짜 근거가 불충분하면 `range` 또는 `unknown` 정밀도를 사용한다.
- 커밋 시각만으로 모든 작업 날짜를 단정하지 않는다.
- 회사 정보가 포함될 수 있으므로 기록 저장소의 공개 범위를 확인한다.

현재 GitHub 저장소는 공개 저장소다. 실제 회사 업무 기록을 푸시하기 전에는 기록 내용과 원격 공개 범위를 반드시 검토해야 한다.

## 문서

- [작업 방법론](./docs/work-method.md)
- [데이터 모델](./docs/data-model.md)
- [CLI 사용법](./docs/cli.md)
- [사용 시나리오](./docs/usage-scenarios.md)
- [Codex Skill](./adapters/codex/record-work/SKILL.md)
- [데스크톱 앱 구현 계획](./docs/desktop-app-plan.md)
- [Tauri 도입 ADR](./docs/adr/0001-tauri-desktop-app.md)
