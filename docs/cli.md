# Work Harvest CLI

## 실행 방식

개발 저장소에서는 다음과 같이 실행한다.

```bash
pnpm wh <command>
```

패키지의 `bin`을 실행 경로에 설치한 환경에서는 `wh <command>`를 사용한다.

데이터 저장 위치는 다음 우선순위로 결정한다.

1. `--root <path>`
2. `WORK_HARVEST_HOME`
3. 현재 작업 디렉터리

`WORK_HARVEST_CLI_HOME`은 Codex Skill 래퍼가 CLI 코드의 checkout을 찾는 용도다. 기록 데이터 위치를 정하는 `WORK_HARVEST_HOME`과 구분한다.

## 공통 입력

생성 명령은 JSON 또는 YAML 객체를 파일이나 stdin으로 받는다.

```bash
pnpm wh work-item create --input payload.yaml
pnpm wh work-item create --input - < payload.json
```

에이전트 연동에서는 임시 파일을 만들지 않아도 되는 stdin 방식을 권장한다. `--json`을 사용하면 성공과 오류 결과가 기계 판독 가능한 JSON으로 출력된다.

## `work-item create`

```bash
pnpm wh work-item create --input <file|-> [--root <path>] [--json]
```

필수 입력:

- `id`
- `project_id`
- `title`
- `objective`

나머지 값에는 안전한 기본값을 적용한다. 생성 결과:

```text
work-items/<work_item_id>/work-item.json
work-items/<work_item_id>/context.json
work-items/<work_item_id>/context.md
```

기존 업무 항목이나 context 파일은 덮어쓰지 않는다.

최소 입력 예시:

```yaml
id: AUTH-142
project_id: jajak-front
title: 인증 시스템 개선
objective: 토큰 만료 시 요청을 안전하게 재시도한다.
desired_outcomes:
  - 인증 갱신 동작을 테스트로 검증한다.
classification:
  initiative_id: authentication
  work_types:
    - testing
  tags:
    - auth
context:
  current_state: 테스트 작업을 시작하기 전이다.
  next_steps:
    - 기본 성공 경로 테스트 작성
```

`context`는 `context.json`의 초기 상태를 만들고 `context.md`로 렌더링된다.

## `work-item list`

```bash
pnpm wh work-item list [--project <id>] [--status <status>] [--root <path>] [--json]
```

업무 항목을 최근 갱신 순서로 출력한다. 에이전트는 새 업무를 만들기 전에 이 명령으로 기존 업무 항목 후보를 확인한다.

## `work-item show`

```bash
pnpm wh work-item show <id> [--root <path>] [--json]
```

업무 메타데이터, 현재 context와 마지막 체크포인트를 함께 출력한다. 새 세션을 시작할 때 사용할 기본 조회 명령이다.

## `checkpoint capture`

```bash
pnpm wh checkpoint capture --input <file|-> [--root <path>] [--json]
```

필수 입력:

- `work_item_id`
- `title`
- `summary`
- 활동·결정·검증·결과·차단 요소·다음 작업 중 하나 이상

생성 결과:

```text
records/YYYY/MM/DD/<checkpoint_id>.json
records/YYYY/MM/DD/<checkpoint_id>.md
```

저장 폴더 날짜는 `captured_at`과 체크포인트 timezone을 기준으로 계산한다. `captured_at`을 생략하면 현재 시각, `work_period`를 생략하면 해당 날짜의 당일 체크포인트가 된다.

체크포인트 저장과 함께 다음 값이 갱신된다.

- 업무 항목의 `status`, `updated_at`, `completed_at`
- `context.json`의 현재 상태와 인수인계 정보
- `context.md` 파생 문서

최소 입력 예시:

```yaml
work_item_id: AUTH-142
source:
  agent: codex
  surface: desktop
  session_ref: null
  task_title: 인증 테스트 코드 작성
title: 인증 갱신 기본 테스트 작성
summary: refresh token 갱신과 요청 재시도의 기본 성공 경로를 검증했다.
activities:
  - refresh token 갱신 성공 테스트를 추가했다.
verifications:
  - type: test
    description: 인증 갱신 기본 성공 경로 테스트
    status: passed
    command: pnpm test auth
    evidence_refs:
      - tests/auth/refresh-token.test.ts
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
  files:
    - tests/auth/refresh-token.test.ts
```

`kind: final`인 체크포인트는 `status_after: completed`와 하나 이상의 확인된 `outcomes`를 가져야 한다.

`context_update`는 전달된 필드만 교체한다. 전달하지 않은 결정, 파일, 검증, 다음 작업과 리스크는 기존 값을 유지한다. 배열을 비우려면 명시적으로 빈 배열을 전달한다.

## `checkpoint last`

```bash
pnpm wh checkpoint last --work-item <id> [--root <path>] [--json]
```

지정 업무 항목의 가장 최근 체크포인트를 반환한다. 에이전트는 이를 이용해 “지난 체크포인트 이후”의 기록 범위를 정한다.

## `validate`

```bash
pnpm wh validate [--root <path>] [--include-examples] [--json]
```

다음을 검사한다.

- 업무 항목, 구조화된 context와 체크포인트 JSON Schema
- 중복 ID
- 업무 항목과 체크포인트의 프로젝트 관계
- context JSON·Markdown 존재 여부와 식별자 일치
- 정정·연관 체크포인트 참조
- 체크포인트의 날짜별 저장 경로
- 체크포인트 Markdown 존재 여부와 식별자 일치

`--include-examples`는 저장소의 `examples/` 데이터도 별도 데이터셋으로 검증한다.

## 종료 코드

- `0`: 성공
- `1`: 데이터·스키마·파일 처리 오류
- `2`: 잘못된 명령 또는 입력 방식
