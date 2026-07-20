# Work Harvest payloads

The examples use YAML for readability. The Codex record-work workflow must serialize equivalent
JSON when sending a create or capture payload so `#` inside factual text cannot become a YAML
comment.

## Work item creation

Use only after `work-item list` confirms no existing match.

Use exact schema values for `classification.work_types`: `planning`, `design`, `implementation`, `bugfix`, `refactoring`, `testing`, `documentation`, `operation`, `communication`, `research`, or `other`. In particular, use singular `operation`, not `operations`.

```yaml
id: WH-20260713-auth-tests
project_id: jajak-front
title: 인증 갱신 테스트
objective: 토큰 갱신과 재시도 동작을 자동화된 테스트로 검증한다.
problem:
  statement: 토큰 만료 뒤 원 요청이 안전하게 재시도되는지 보장할 근거가 부족하다.
  expected_behavior: 갱신 성공 뒤 원 요청을 한 번만 재시도한다.
  actual_behavior: 자동화된 회귀 검증이 없다.
  observed_example: 동시 인증 실패 시 갱신 요청 횟수와 원 요청 재시도 횟수를 확인할 테스트가 없다.
  affected_surfaces:
    - 인증이 필요한 API 요청
  source_refs:
    - linear:AUTH-142
desired_outcomes:
  - 동시 인증 실패 상황이 테스트로 검증된다.
scope: company
reporting:
  mode: primary
  exclusion_reason: null
classification:
  initiative_id: authentication
  work_types:
    - testing
  tags:
    - auth
repositories:
  - name: jajak-front
    path: /absolute/path/to/jajak-front
    remote_url: null
    default_branch: main
links: []
external_refs:
  - provider: linear
    external_id: AUTH-142
    url: https://linear.app/example/issue/AUTH-142
    role: source
completion:
  target_gate: qa
  current_gate: null
  remaining_gates:
    - development
    - review
    - qa
context:
  current_state: 인증 갱신 테스트를 시작하기 전이다.
  decisions: []
  files:
    - path: tests/auth/refresh-token.test.ts
      description: 인증 갱신 테스트
  verification:
    completed: []
    pending:
      - 기본 성공 경로 테스트
  next_steps:
    - 기본 성공 경로 테스트 작성
  risks: []
  git:
    repository: jajak-front
    branch: feat/auth-refresh
    commit: abc1234
    checked_at: 2026-07-13T09:00:00+09:00
```

`scope` must be `company` or `personal` for new records. `reporting.mode` must be:

- `primary`: independent weekly-report row
- `supporting`: useful operational evidence, hidden from the default report
- `excluded`: retained record that must not appear in reports

Use `supporting` for branch synchronization, PR mechanics, formatting, dependency installation, and similar steps unless they independently produced a manager-visible outcome.

Run:

```bash
<skill-dir>/scripts/wh work-item create --input <file|-> --json
```

## Progress checkpoint

Omit `captured_at` for a checkpoint created now. Omit `work_period` for a same-day checkpoint. Supply them explicitly for a backfill or user-defined range.

```yaml
work_item_id: WH-20260713-auth-tests
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
decisions:
  - summary: 원 요청 재시도를 한 번으로 제한한다.
    rationale: refresh token 만료 시 무한 재시도를 방지하기 위해서다.
    status: accepted
verifications:
  - type: test
    description: 인증 갱신 기본 성공 경로 테스트
    status: passed
    command: pnpm test auth
    method: command
    observed_at: 2026-07-13T18:10:00+09:00
    evidence_refs:
      - tests/auth/refresh-token.test.ts
outcomes:
  - description: 갱신 후 원 요청이 정상적으로 재시도되는 동작을 검증했다.
    impact:
      description: 인증 만료 시 사용자의 요청이 불필요하게 실패하는 회귀를 자동으로 탐지할 수 있다.
      status: observed
      basis: 자동화 테스트 통과
    category: quality
    reporting: primary
    evidence_refs:
      - tests/auth/refresh-token.test.ts
blockers: []
next_steps:
  - 동시 요청 테스트 작성
evidence:
  commits:
    - abc1234
  pull_requests: []
  issues: []
  files:
    - tests/auth/refresh-token.test.ts
  commands:
    - pnpm test auth
  urls: []
git:
  repository: jajak-front
  branch: feat/auth-refresh
  head_before: def5678
  head_after: abc1234
  dirty: false
context_update:
  current_state: 기본 성공 경로를 검증했고 동시 요청 테스트가 남아 있다.
  decisions:
    - 원 요청 재시도를 한 번으로 제한한다. 무한 재시도를 방지하기 위해서다.
  files:
    - path: tests/auth/refresh-token.test.ts
      description: 인증 갱신 테스트
  verification:
    completed:
      - description: 인증 갱신 기본 성공 경로 테스트
        status: passed
        method: command
        source_ref: command:pnpm-test-auth
        observed_at: 2026-07-13T18:10:00+09:00
    pending:
      - 동시 요청 테스트
  next_steps:
    - 동시 요청 테스트 작성
  risks: []
  lifecycle:
    target_gate: qa
    current_gate: development
    external_state: In Progress
    remaining_gates:
      - review
      - qa
    observed_at: 2026-07-13T18:10:00+09:00
```

## Final checkpoint

Set `kind: final`; the CLI supplies `status_after: completed`. Include at least one confirmed outcome and clear completed next steps when appropriate.

```yaml
work_item_id: WH-20260713-auth-tests
kind: final
title: 인증 갱신 테스트 완료
summary: 계획한 인증 갱신 테스트를 완료했다.
activities:
  - 동시 인증 실패 테스트를 추가했다.
verifications:
  - type: test
    description: 인증 테스트 전체 실행
    status: passed
    command: pnpm test auth
    method: command
    observed_at: 2026-07-13T18:10:00+09:00
    evidence_refs:
      - tests/auth/refresh-token.test.ts
outcomes:
  - description: 계획한 인증 갱신 시나리오가 자동화된 테스트로 검증됐다.
    impact:
      description: 인증 갱신 회귀를 배포 전에 탐지할 수 있다.
      status: observed
      basis: 전체 인증 테스트 통과
    category: quality
    reporting: primary
    evidence_refs:
      - tests/auth/refresh-token.test.ts
blockers: []
next_steps: []
completion:
  reached_gate: qa
  remaining_gates: []
  evidence_refs:
    - tests/auth/refresh-token.test.ts
context_update:
  current_state: 인증 갱신 테스트를 완료했다.
  verification:
    completed:
      - 인증 테스트 전체 통과
    pending: []
  next_steps: []
  risks: []
```

## Backfill and correction

For backfill, set `kind: backfill` and give `work_period.precision` and `work_period.basis` honestly. Omit `captured_at` to use the current recording time unless an exact capture timestamp is explicitly required.

Example for work described in a later recording-only task:

```yaml
work_item_id: WH-20260713-release-guide
kind: backfill
source:
  agent: manual
  surface: desktop
  session_ref: null
  task_title: 지난주 배포 가이드 작성 기록
work_period:
  start: 2026-07-06
  end: 2026-07-10
  precision: range
  basis:
    - user
  timezone: Asia/Seoul
title: 배포 가이드 초안 작성
summary: 사용자가 전달한 내용에 따르면 지난주에 신규 배포 가이드 초안을 작성했다.
activities:
  - 배포 전 점검 절차와 장애 복구 순서를 문서화했다.
verifications:
  - type: review
    description: 배포 가이드 내용 검토
    status: not_run
    command: null
    method: external
    observed_at: null
    evidence_refs: []
outcomes:
  - description: 배포 가이드 초안이 작성됐다고 사용자가 보고했다.
    impact: 사용자 제공 설명 기준이며 Codex가 문서를 직접 확인하지 않았다.
    category: product_change
    reporting: primary
    evidence_refs: []
next_steps:
  - 배포 가이드 문서 링크를 연결하고 내용을 검토한다.
evidence:
  urls: []
context_update:
  current_state: 배포 가이드 초안 작성 사실을 사후 기록했으며 문서 확인이 남아 있다.
  verification:
    completed: []
    pending:
      - 배포 가이드 문서 확인
  next_steps:
    - 문서 링크 연결 및 내용 검토
  risks:
    - 현재 기록은 사용자 설명에 기반하며 문서가 독립적으로 확인되지 않았다.
```

Documentation or non-code work performed in the current task is not a backfill merely because it has no Git change. Use `progress` or `final`, select the appropriate work types, and cite documents, URLs, decisions, or reviews when available.

For correction, set:

```yaml
kind: correction
correction_of: CP-20260713-001
```

Describe the corrected fact. Never edit the prior checkpoint.
