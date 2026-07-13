# Work Harvest payloads

## Work item creation

Use only after `work-item list` confirms no existing match.

```yaml
id: WH-20260713-auth-tests
project_id: jajak-front
title: 인증 갱신 테스트
objective: 토큰 갱신과 재시도 동작을 자동화된 테스트로 검증한다.
desired_outcomes:
  - 동시 인증 실패 상황이 테스트로 검증된다.
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
    evidence_refs:
      - tests/auth/refresh-token.test.ts
outcomes:
  - description: 갱신 후 원 요청이 정상적으로 재시도되는 동작을 검증했다.
    impact: null
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
      - 인증 갱신 기본 성공 경로 테스트 통과
    pending:
      - 동시 요청 테스트
  next_steps:
    - 동시 요청 테스트 작성
  risks: []
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
    evidence_refs:
      - tests/auth/refresh-token.test.ts
outcomes:
  - description: 계획한 인증 갱신 시나리오가 자동화된 테스트로 검증됐다.
    impact: null
    evidence_refs:
      - tests/auth/refresh-token.test.ts
blockers: []
next_steps: []
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

For backfill, set `kind: backfill` and give `captured_at`, `work_period.precision`, and `work_period.basis` honestly.

For correction, set:

```yaml
kind: correction
correction_of: CP-20260713-001
```

Describe the corrected fact. Never edit the prior checkpoint.
