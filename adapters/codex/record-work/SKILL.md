---
name: record-work
description: Record live or retrospectively described work into the local Work Harvest as a dated checkpoint and refresh its handoff context. Use for code, documentation, planning, research, communication, operations, or other reportable work when the user asks to record today's work, save progress, record work performed outside the current Codex task, prepare a handoff, finalize a work item, backfill missed work, or correct an earlier record. Trigger on Korean requests such as "오늘 작업은 여기까지 기록해줘", "현재 단계까지 기록해줘", "지난주에 한 작업을 기록해줘", "이 문서 작업을 기록해줘", "새 세션용 인수인계를 갱신해줘", and "업무 완료 기록을 작성해줘".
---

# Record Work

Persist a concise, evidence-backed delta from the current task into Work Harvest. Keep the source JSON valid and let the CLI render the human-readable Markdown.

## CLI

Run every Work Harvest command through:

```bash
<skill-dir>/scripts/wh <command>
```

The wrapper uses `WORK_HARVEST_CLI_BIN` when set. Otherwise it resolves a checkout from `WORK_HARVEST_CLI_HOME` or the Skill's source checkout, prefers the release or debug Rust CLI, and falls back to the Node compatibility CLI only when no native binary exists. It stores records in `~/work-records` by default; set `WORK_HARVEST_HOME` or pass `--root` only to use a different data store. Stop and report the missing configuration if the wrapper cannot locate Work Harvest.

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

Classify the recording situation before gathering evidence:

| Situation | Kind | Source and evidence handling |
| --- | --- | --- |
| Work performed in this Codex task | `progress` or `final` | Use current task, repository, commands, and artifacts as direct evidence |
| Documentation or other non-code work in this task | `progress` or `final` | Use documents, URLs, decisions, reviews, or delivered artifacts; Git is optional |
| Work described after it happened | `backfill` | Use `source.agent: manual`, preserve the real work period, and identify user-reported claims |
| Existing document or external record imported as work history | `backfill` | Reference the document or URL and do not claim independent verification |
| Prior checkpoint contains a factual error | `correction` | Point `correction_of` to the immutable prior checkpoint |

Do not add a new schema field to represent these situations. Express them with `kind`, `source`, `work_period`, evidence, and precise wording.

### 2. Find the work item before creating one

Run:

```bash
<skill-dir>/scripts/wh work-item list --json
```

Match candidates using project, objective, intended outcome, initiative, repository, issue or PR, and current state. Treat a branch or session as supporting context, not as the work item identity. Then inspect the best candidate:

```bash
<skill-dir>/scripts/wh work-item show <id> --json
```

- Reuse a work item only when the match is clear.
- If multiple candidates are materially plausible, ask the user to select one.
- Create a new work item only when no existing item represents the same reportable objective.
- Prefer an existing external issue ID. Otherwise use `WH-YYYYMMDD-<short-slug>`.
- Reuse one work item across branches or sessions when they pursue the same reportable outcome.
- Split work performed in one branch or session when it produces materially separate reportable outcomes.
- Permit empty `repositories` for documentation, planning, communication, research, and operations work.
- Use only the exact `work_types` values: `planning`, `design`, `implementation`, `bugfix`, `refactoring`, `testing`, `documentation`, `operation`, `communication`, `research`, or `other`. Use singular `operation`, never `operations`.

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

For retrospectively described work:

- Treat the user's statement as a valid record source, but not as independent verification.
- Use `work_period.basis: [user]` unless stronger date evidence is available.
- Keep `captured_at` as the current recording time; never disguise it as the work time.
- Write user-reported outcomes as user-reported in `impact` or the description.
- Leave verification as `not_run`, omit it, or state a follow-up when Codex did not verify it.
- Use `precision: range` or `unknown` when exact dates cannot be reconstructed.

For non-code work, look for deliverables such as documents, decisions, meeting notes, review comments, dashboards, messages, and URLs. Do not require a branch, commit, or changed source file.

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
- Do not use a branch name as the sole reason to merge or split work items.
- Do not reject non-code work because no repository or Git evidence exists.
- Do not present user-reported work as independently verified.
- Do not guess a session reference. Store `null` when Codex does not expose one reliably.
- Ask one concise question only when the work item or completion status cannot be resolved safely from local evidence.
