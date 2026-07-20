---
name: record-work
description: Record work into the local Work Harvest, classify company versus personal work, control weekly-report inclusion, and prepare scope-filtered weekly report drafts. Use when the user asks to record progress, prepare a handoff, finalize or correct a work item, backfill missed work, exclude supporting activity, or draft a company-only weekly report.
---

# Record Work

Persist a concise, evidence-backed delta from the current task into Work Harvest. Keep the source JSON valid and let the CLI render the human-readable Markdown.

## CLI

Run every Work Harvest command through:

```bash
<skill-dir>/scripts/wh <command>
```

The wrapper uses `WORK_HARVEST_CLI_BIN` when set. Otherwise it prefers the CLI bundled in an explicit `WORK_HARVEST_APP_PATH`, a release Rust CLI from `WORK_HARVEST_CLI_HOME` or the Skill's source checkout, then `/Applications/Work Harvest.app`, `~/Applications/Work Harvest.app`, `~/.local/bin/wh`, and a debug Rust CLI. It falls back to the Node compatibility CLI only when no native binary exists. It stores records in `~/work-records` by default; set `WORK_HARVEST_HOME` or pass `--root` only to use a different data store. Stop and report the missing configuration if the wrapper cannot locate Work Harvest.

## Workflow

### 1. Interpret the request

If the user asks for a weekly report, company form, or a scope-filtered summary rather than a new record, skip checkpoint creation and follow **Weekly reporting** below.

Map the request to one checkpoint kind:

| Intent | Kind | Status |
| --- | --- | --- |
| Today/current stage/since last checkpoint | `progress` | Usually `in_progress` |
| A named gate reached while later gates remain | `milestone` | `in_progress` or `blocked` |
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

If this Codex task already established a work item ID during an earlier successful capture, reuse
that ID. Do not list or show work items again unless the objective changed, the user selected a
different work item, or current evidence makes the prior match doubtful.

For the first capture in a task, run the compact index:

```bash
<skill-dir>/scripts/wh work-item list --compact --json
```

Add `--project <id>` when the current repository or task establishes the project ID reliably. Use
the unfiltered index only when the project boundary is genuinely unknown.

Match candidates using project, objective, intended outcome, initiative, repository, issue or PR, and current state. Treat a branch or session as supporting context, not as the work item identity. Then inspect the best candidate:

```bash
<skill-dir>/scripts/wh work-item show <id> --compact --json
```

- Reuse a work item only when the match is clear.
- If multiple candidates are materially plausible, ask the user to select one.
- Create a new work item only when no existing item represents the same reportable objective.
- Apply the manager test before creating a work item: **Would a manager recognize this as an independent result?** If not, attach it to the related objective or classify it as supporting activity.
- Branch synchronization, PR creation, formatting, dependency installation, routine release steps, and local environment maintenance are `supporting` by default. They become `primary` only when they independently solve a material organizational problem.
- Prefer an existing external issue ID. Otherwise use `WH-YYYYMMDD-<short-slug>`.
- Reuse one work item across branches or sessions when they pursue the same reportable outcome.
- Split work performed in one branch or session when it produces materially separate reportable outcomes.
- Permit empty `repositories` for documentation, planning, communication, research, and operations work.
- Use only the exact `work_types` values: `planning`, `design`, `implementation`, `bugfix`, `refactoring`, `testing`, `documentation`, `operation`, `communication`, `research`, or `other`. Use singular `operation`, never `operations`.
- Set `scope` explicitly to `company` or `personal`. Ask one concise question when repository ownership and user context do not resolve it safely.
- Set `reporting.mode` to `primary`, `supporting`, or `excluded`. Use `primary` only for a standalone reportable outcome.

Read [references/payloads.md](references/payloads.md) before creating a work item or checkpoint payload.
When an external issue, completion boundary, or reportable result is involved, also read
[references/collection-quality.md](references/collection-quality.md).

### 3. Establish the checkpoint boundary

The compact `work-item show` result already contains the last checkpoint boundary. After reusing a
work item ID in a later capture, run only:

```bash
<skill-dir>/scripts/wh checkpoint boundary --work-item <id> --json
```

Record only the delta after that checkpoint. Use its ID, `captured_at`, work period, Git refs, and
the compact work item context as boundary evidence. Do not load the checkpoint body unless a
specific factual ambiguity cannot be resolved from the current task, repository, boundary, and
context. Do not summarize the entire task again.

If there is no prior checkpoint, record the current task scope. If the user requests a date range that cannot be reconstructed reliably, use `precision: range` or `unknown`; never invent daily boundaries.

### 4. Gather evidence

Use the current conversation and safe read-only repository checks to confirm:

- Changed files and relevant Git refs
- Commands, tests, builds, lint, review, or measurements actually performed
- Decisions and their rationale
- Confirmed outcomes and impact
- Blockers, remaining risk, and next steps

When a stable external issue ID exists, read that issue's canonical current state. Populate the
compact `problem` and `external_refs` fields on the work item, then capture changing workflow state
in checkpoint `external_states`. Do not rely on an issue key or title alone and do not copy full
issue histories into the record.

Treat code and current Git state as stronger evidence than remembered conversation context. Do not claim an unexecuted verification passed. Do not copy full transcripts, full diffs, secrets, environment variables, or sensitive command output into the record.

Classify confidentiality before preparing the payload:

- `normal`: safe to include with full evidence for the intended report audience.
- `sensitive`: keep locally, but omit commands, files, URLs, Git details, decisions, and evidence
  references from derived reports. Prefer this when private repository, account, local path,
  internal issue, or environment details are useful to preserve but unnecessary to share.
- `restricted`: exclude from derived reports and notifications. Use for secrets, credentials,
  personal or customer data, access-control details, or other content that should only appear when
  the user explicitly opens the local record.

When uncertain between `normal` and `sensitive`, use `sensitive`. Never lower a recent
confidentiality level without user direction or clear evidence that the new delta is safe for the
broader audience.

For retrospectively described work:

- Treat the user's statement as a valid record source, but not as independent verification.
- Use `work_period.basis: [user]` unless stronger date evidence is available.
- Keep `captured_at` as the current recording time; never disguise it as the work time.
- Write user-reported outcomes as user-reported in `impact` or the description.
- Leave verification as `not_run`, omit it, or state a follow-up when Codex did not verify it.
- Use `precision: range` or `unknown` when exact dates cannot be reconstructed.

For non-code work, look for deliverables such as documents, decisions, meeting notes, review comments, dashboards, messages, and URLs. Do not require a branch, commit, or changed source file.

### 5. Prepare the payload

Create a JSON object following [references/payloads.md](references/payloads.md).

For `context_update`:

- Send the complete latest value for every array field you include.
- Omit fields that should remain unchanged.
- Send `[]` only when the current list should be cleared.
- Keep only current decisions, files, verification state, next steps, and risks—not historical narration.
- Keep the handoff compact: current state 3–5 sentences, up to 5 active decisions, 8 key files, 5
  representative completed verifications, 3 pending verifications, 5 next steps, and 3 current
  risks. These are working budgets rather than reasons to omit a materially important fact.

For the checkpoint delta:

- Prefer a one-sentence summary, 1–3 outcomes, 0–2 material decisions, and distinct verification
  gates rather than every repeated command invocation.
- Do not copy unchanged context next steps, context files, or the previous Git commit into the new
  checkpoint evidence.
- Keep resolved experiments and transient failures out unless they changed a decision, explain a
  remaining risk, or are necessary to understand the result.
- Set `category` and `reporting` on every new outcome. Keep delivery mechanics supporting and
  record maintenance excluded.
- Set verification `method` and `observed_at`; a passed verification needs an actual command,
  artifact, manual observation, or external evidence.
- Preserve exact limits or observed counts that materially explain a reproduction in
  `problem.observed_example`; omit full payloads and HTML.
- Store primary impact with `description`, evidence maturity `status`, and `basis`. Use `expected`
  until QA, measurement, or user observation confirms the effect.
- Record the reached and remaining completion gates. Use `milestone` for merged or reviewed work
  that still awaits QA, release, or operational confirmation.
- Refresh `context_update.lifecycle` after an external status or gate change, and use structured
  Context verification entries so the next task retains their provenance.

For `final`, include at least one confirmed primary outcome and require the target completion gate
to be reached with no remaining gates. If a meaningful gate was reached but later gates remain,
use `milestone`; if completion or its evidence is uncertain, use `progress`.

### 6. Capture and validate

Pass the JSON payload through a safely created file or stdin. Do not interpolate JSON into a shell
command. Do not generate YAML for capture payloads; unquoted `#` characters can be interpreted as
comments and truncate factual text.

```bash
<skill-dir>/scripts/wh checkpoint capture --compact --input <file|-> --json
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

Do not print the checkpoint body, context body, full validation dataset, or full work item after a
successful capture. Remember the confirmed work item ID and new checkpoint boundary for the next
capture in the same Codex task.

Do not turn the confirmation into a weekly report.

### 8. Weekly reporting

For a company-only report preview, run:

```bash
<skill-dir>/scripts/wh report weekly --start <YYYY-MM-DD> --end <YYYY-MM-DD> --scope company --json
```

This includes only `scope: company` and `reporting.mode: primary` by default. Add `--include-supporting` only when the user explicitly asks to see supporting activity. For personal reporting, pass `--scope personal`; omit `--scope` only when the user asks for everything.

Use the Markdown preview from `files[0].after` as the evidence source, then adapt it to the user's requested company form. Do not silently add `personal`, `unclassified`, `supporting`, or `excluded` work to a company report.

When an existing item is classified incorrectly, update it instead of deleting its evidence:

```bash
<skill-dir>/scripts/wh work-item update <id> --input <file|-> --json
```

Use `reporting.mode: excluded` to remove it from reports, or `supporting` to preserve it as operational context without creating a standalone weekly-report row. Use the app's reversible trash only for records the user explicitly wants removed as records.

## No-op and ambiguity rules

- Do not create an empty checkpoint when nothing changed since the last checkpoint.
- Do not merge separate reportable outcomes just because they occurred in one Codex task.
- Do not split one outcome into a new work item just because the session or date changed.
- Do not create a standalone primary work item for routine operational steps that merely support another outcome.
- Do not infer that a company-owned repository makes every activity company-reportable; `scope` and `reporting.mode` are separate decisions.
- Do not use a branch name as the sole reason to merge or split work items.
- Do not reject non-code work because no repository or Git evidence exists.
- Do not present user-reported work as independently verified.
- Do not guess a session reference. Store `null` when Codex does not expose one reliably.
- Ask one concise question only when the work item or completion status cannot be resolved safely from local evidence.
