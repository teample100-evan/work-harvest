# Collection and reporting quality

Use these rules when the work item has an external issue or when the checkpoint may be used in a
performance or weekly report.

## Hydrate the problem, not the whole external system

Read the canonical issue before the first capture and whenever its completion state may have
changed. Store only the stable, report-relevant facts in `work_item.problem`:

- the exact user-visible or operational symptom;
- expected and actual behavior;
- affected users or surfaces;
- exact limits, counts, or a compact reproduction example when those numbers explain the failure;
- compact references to the evidence source.

Keep the stable identity in `external_refs`. Never copy the entire issue description, comments,
or provider workflow into Work Harvest. Capture a changing provider status in checkpoint
`external_states` with `observed_at` so later reports do not need to query the provider again.

## Separate completion gates

Choose the work item's `completion.target_gate` from `development`, `review`, `qa`, `release`, or
`operational`. A merged PR reaches a delivery milestone; it does not prove QA or release completion.

- Use `milestone` when a meaningful gate was reached but later gates remain.
- Use `progress` when work advanced without reaching a named gate.
- Use `final` only when the target gate is reached and `remaining_gates` is empty.
- Preserve an external state such as `QA Pending` exactly as observed. Do not translate it to
  `completed` merely because implementation or review finished.

## Classify outcomes for reports

Every new outcome should set both `category` and `reporting`.

| Category | Typical reporting |
| --- | --- |
| `user_impact`, `product_change`, `quality` | `primary` when independently reportable |
| `delivery`, `operation` | usually `supporting` |
| `record_maintenance` | `excluded` |

A primary outcome states the result and its impact. Store impact as an object and distinguish
`expected`, `observed`, `measured`, and `user_reported` impact. Do not describe an expected effect
as already realized while QA or release remains. PR creation, merge, branch synchronization,
build installation, and record correction are delivery or maintenance evidence unless they solve
an independent organizational problem.

For `sensitive` records, keep the primary outcome and impact report-safe and sanitized because
derived reports may omit detailed activities, decisions, files, commands, Git data, and URLs.

## Preserve verification provenance

For new verifications set `method` to `command`, `artifact`, `manual`, or `external`, and set
`observed_at` when known.

- A passed command verification includes the exact command and an evidence reference when one is
  available.
- Artifact and external verification cite the artifact, CI run, review, or URL reference.
- Manual verification describes the observed behavior; it does not masquerade as an automated test.
- Never mark a verification passed from a remembered claim alone.

Before capture, reject or correct these quality failures:

- `final` while completion gates remain;
- a passed verification with neither a command nor an evidence reference;
- a primary delivery or record-maintenance outcome;
- a primary outcome with no meaningful impact;
- an external state without its observation time.

Refresh `context_update.lifecycle` whenever completion or external state changes. Preserve
verification provenance in Context objects instead of flattening external evidence into an
unqualified “passed” sentence.

## Example: implementation merged, QA pending

```json
{
  "kind": "milestone",
  "status_after": "in_progress",
  "external_states": [
    {
      "provider": "linear",
      "reference": "JAK-295",
      "state": "QA Pending",
      "observed_at": "2026-07-20T11:47:00+09:00",
      "url": "https://linear.app/example/issue/JAK-295"
    }
  ],
  "completion": {
    "reached_gate": "review",
    "remaining_gates": ["qa"],
    "evidence_refs": ["pull-request:3"]
  }
}
```
