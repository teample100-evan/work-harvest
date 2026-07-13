---
name: record-work
description: Record work performed in the current Codex task into the local Work Harvest as a dated checkpoint and refresh its handoff context. Use when the user says to record today's work, record up to the current point, save progress since the last checkpoint, prepare or update a handoff for a new session, finalize a completed work item, backfill missed work, or correct an earlier work record. Trigger on Korean requests such as "오늘 작업은 여기까지 기록해줘", "현재 단계까지 기록해줘", "지난 체크포인트 이후 작업을 기록해줘", "새 세션용 인수인계를 갱신해줘", and "업무 완료 기록을 작성해줘".
---

# Record Work

Persist a concise, evidence-backed delta from the current task into Work Harvest. Keep the source JSON valid and let the CLI render the human-readable Markdown.

## CLI

Run every Work Harvest command through:

```bash
<skill-dir>/scripts/wh <command>
```

The wrapper resolves its CLI checkout from `WORK_HARVEST_CLI_HOME` or the Skill's source checkout. The CLI resolves the data store from `WORK_HARVEST_HOME` or `--root`. Stop and report the missing configuration if the wrapper cannot locate Work Harvest.

## Workflow

### 1. Interpret the request

Map the request to one checkpoint kind:

| Intent | Kind | Status |
| --- | --- | --- |
| Today/current stage/since last checkpoint | `progress` | Usually `in_progress` |
| Finish the work item | `final` | `completed` |
| Reconstruct missed prior work | `backfill` | Reflect actual state |
| Correct an immutable prior checkpoint | `correction` | Reflect actual state |

An explicit handoff request still creates a checkpoint, then refreshes context. Do not update context without preserving the delta that led to it.

### 2. Find the work item before creating one

Run:

```bash
<skill-dir>/scripts/wh work-item list --json
```

Match candidates using project, repository, issue or PR, objective, initiative, branch, and current state. Then inspect the best candidate:

```bash
<skill-dir>/scripts/wh work-item show <id> --json
```

- Reuse a work item only when the match is clear.
- If multiple candidates are materially plausible, ask the user to select one.
- Create a new work item only when no existing item represents the same reportable objective.
- Prefer an existing external issue ID. Otherwise use `WH-YYYYMMDD-<short-slug>`.

Read [references/payloads.md](references/payloads.md) before creating a work item or checkpoint payload.

### 3. Establish the checkpoint boundary

Run:

```bash
<skill-dir>/scripts/wh checkpoint last --work-item <id> --json
```

Record only the delta after that checkpoint. Use its `captured_at`, Git refs, evidence, and current context as boundary evidence. Do not summarize the entire task again.

If there is no prior checkpoint, record the current task scope. If the user requests a date range that cannot be reconstructed reliably, use `precision: range` or `unknown`; never invent daily boundaries.

### 4. Gather evidence

Use the current conversation and safe read-only repository checks to confirm:

- Changed files and relevant Git refs
- Commands, tests, builds, lint, review, or measurements actually performed
- Decisions and their rationale
- Confirmed outcomes and impact
- Blockers, remaining risk, and next steps

Treat code and current Git state as stronger evidence than remembered conversation context. Do not claim an unexecuted verification passed. Do not copy full transcripts, full diffs, secrets, environment variables, or sensitive command output into the record.

### 5. Prepare the payload

Create a JSON or YAML object following [references/payloads.md](references/payloads.md).

For `context_update`:

- Send the complete latest value for every array field you include.
- Omit fields that should remain unchanged.
- Send `[]` only when the current list should be cleared.
- Keep only current decisions, files, verification state, next steps, and risks—not historical narration.

For `final`, include at least one confirmed outcome. If completion or its evidence is uncertain, use `progress` and state what remains.

### 6. Capture and validate

Pass the payload through a safely created file or stdin. Do not interpolate JSON into a shell command.

```bash
<skill-dir>/scripts/wh checkpoint capture --input <file|-> --json
<skill-dir>/scripts/wh validate --json
```

If capture fails, do not manually edit around the validator. Correct the payload or report the actual blocker. Never overwrite a prior checkpoint; use `correction` with `correction_of`.

### 7. Report the result

Return a compact confirmation containing:

- Work item ID and title
- Checkpoint ID and kind
- Work period and precision
- JSON and Markdown record paths
- Whether handoff context was refreshed
- Any unverified claim, blocker, or follow-up the user should know

Do not turn the confirmation into a weekly report.

## No-op and ambiguity rules

- Do not create an empty checkpoint when nothing changed since the last checkpoint.
- Do not merge separate reportable outcomes just because they occurred in one Codex task.
- Do not split one outcome into a new work item just because the session or date changed.
- Do not guess a session reference. Store `null` when Codex does not expose one reliably.
- Ask one concise question only when the work item or completion status cannot be resolved safely from local evidence.
