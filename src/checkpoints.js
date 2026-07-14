import path from "node:path";
import { CliError } from "./errors.js";
import { joinFrontmatter, pathExists, serializeJson } from "./io.js";
import { resolveWithinRoot } from "./paths.js";
import {
  commitFileOperations,
  createFileOperation,
  replaceFileOperation,
} from "./rust-writer.js";
import { calendarDate, calendarParts, generateCheckpointId } from "./time.js";
import {
  loadWorkItem,
  readContext,
  renderContext,
} from "./work-items.js";

function strings(values) {
  return (values ?? []).map((value) => String(value));
}

function normalizeFiles(values) {
  return (values ?? []).map((value) =>
    typeof value === "string"
      ? { path: value, description: null }
      : { path: String(value.path), description: value.description ?? null },
  );
}

function normalizeSource(value = {}) {
  return {
    agent: value.agent ?? "manual",
    surface: value.surface ?? "unknown",
    session_ref: value.session_ref ?? null,
    task_title: value.task_title ?? null,
  };
}

function normalizeEvidence(value = {}) {
  return {
    commits: strings(value.commits),
    pull_requests: strings(value.pull_requests),
    issues: strings(value.issues),
    files: strings(value.files),
    commands: strings(value.commands),
    urls: strings(value.urls),
  };
}

export function normalizeCheckpoint(input, workItem, now = new Date().toISOString()) {
  if (!input.title || !input.summary) {
    throw new CliError("Checkpoint input requires title and summary", {
      exitCode: 2,
    });
  }

  const timezone = input.work_period?.timezone ?? "Asia/Seoul";
  const capturedAt = input.captured_at ?? now;
  const workDate = calendarDate(capturedAt, timezone);
  const kind = input.kind ?? "progress";

  return {
    schema_version: "1.0",
    id: input.id ?? generateCheckpointId(capturedAt, timezone),
    work_item_id: workItem.id,
    project_id: workItem.project_id,
    kind,
    source: normalizeSource(input.source),
    captured_at: capturedAt,
    work_period: {
      start: input.work_period?.start ?? workDate,
      end: input.work_period?.end ?? workDate,
      precision: input.work_period?.precision ?? "day",
      basis: input.work_period?.basis ?? ["checkpoint"],
      timezone,
    },
    title: String(input.title),
    summary: String(input.summary),
    status_after:
      input.status_after ?? (kind === "final" ? "completed" : "in_progress"),
    activities: strings(input.activities),
    decisions: input.decisions ?? [],
    verifications: input.verifications ?? [],
    outcomes: input.outcomes ?? [],
    blockers: strings(input.blockers),
    next_steps: strings(input.next_steps),
    evidence: normalizeEvidence(input.evidence),
    ...(input.git ? { git: input.git } : {}),
    related_checkpoint_ids: strings(input.related_checkpoint_ids),
    correction_of: input.correction_of ?? null,
    confidentiality: input.confidentiality ?? "normal",
  };
}

function checkpointPath(root, checkpoint) {
  const timezone = checkpoint.work_period.timezone ?? "Asia/Seoul";
  const { year, month, day } = calendarParts(checkpoint.captured_at, timezone);
  return resolveWithinRoot(
    root,
    `records/${year}/${month}/${day}/${checkpoint.id}.json`,
  );
}

function updatedWorkItem(workItem, checkpoint) {
  return {
    ...workItem,
    status: checkpoint.status_after,
    updated_at: checkpoint.captured_at,
    completed_at:
      checkpoint.status_after === "completed" ? checkpoint.captured_at : null,
  };
}

function has(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

export function mergeContextState(current, updateValue, checkpoint) {
  const update = updateValue ?? {};
  const verification = update.verification ?? {};
  const gitUpdate = update.git ?? {};
  const checkpointGit = checkpoint.git;
  const inferredGit = checkpointGit
    ? {
        repository: checkpointGit.repository,
        branch: checkpointGit.branch,
        commit: checkpointGit.head_after,
        checked_at: checkpoint.captured_at,
      }
    : {};
  const mergedGitInput = { ...inferredGit, ...gitUpdate };

  return {
    ...current,
    updated_at: checkpoint.captured_at,
    last_checkpoint_id: checkpoint.id,
    last_verified_git_ref:
      checkpointGit?.head_after ?? current.last_verified_git_ref,
    current_state: has(update, "current_state")
      ? String(update.current_state)
      : current.current_state,
    decisions: has(update, "decisions")
      ? strings(update.decisions)
      : current.decisions,
    files: has(update, "files") ? normalizeFiles(update.files) : current.files,
    verification: {
      completed:
        has(verification, "completed") || has(update, "verification_completed")
          ? strings(verification.completed ?? update.verification_completed)
          : current.verification.completed,
      pending:
        has(verification, "pending") || has(update, "verification_pending")
          ? strings(verification.pending ?? update.verification_pending)
          : current.verification.pending,
    },
    next_steps: has(update, "next_steps")
      ? strings(update.next_steps)
      : current.next_steps,
    risks: has(update, "risks") ? strings(update.risks) : current.risks,
    git: {
      repository: has(mergedGitInput, "repository")
        ? (mergedGitInput.repository ?? null)
        : current.git.repository,
      branch: has(mergedGitInput, "branch")
        ? (mergedGitInput.branch ?? null)
        : current.git.branch,
      commit: has(mergedGitInput, "commit")
        ? (mergedGitInput.commit ?? null)
        : current.git.commit,
      checked_at: has(mergedGitInput, "checked_at")
        ? (mergedGitInput.checked_at ?? null)
        : current.git.checked_at,
    },
  };
}

function list(values, fallback = "없음") {
  return values.length
    ? values.map((value) => `- ${value}`).join("\n")
    : `- ${fallback}`;
}

function renderDecisions(values) {
  return values.length
    ? values
        .map(
          (value) =>
            `- ${value.summary}\n  - 이유: ${value.rationale}\n  - 상태: ${value.status}`,
        )
        .join("\n")
    : "- 없음";
}

function renderVerifications(values) {
  return values.length
    ? values
        .map(
          (value) =>
            `- ${value.description}\n  - 유형: ${value.type}\n  - 상태: ${value.status}\n  - 명령: ${value.command ? `\`${value.command}\`` : "없음"}\n  - 근거: ${value.evidence_refs.length ? value.evidence_refs.join(", ") : "없음"}`,
        )
        .join("\n")
    : "- 없음";
}

function renderOutcomes(values) {
  return values.length
    ? values
        .map(
          (value) =>
            `- ${value.description}\n  - 영향: ${value.impact ?? "확인되지 않음"}\n  - 근거: ${value.evidence_refs.length ? value.evidence_refs.join(", ") : "없음"}`,
        )
        .join("\n")
    : "- 없음";
}

function evidenceLines(evidence) {
  return [
    ["커밋", evidence.commits],
    ["PR", evidence.pull_requests],
    ["이슈", evidence.issues],
    ["파일", evidence.files],
    ["명령", evidence.commands],
    ["URL", evidence.urls],
  ]
    .map(([label, values]) => `- ${label}: ${values.length ? values.join(", ") : "없음"}`)
    .join("\n");
}

export function renderCheckpointMarkdown(checkpoint) {
  const attributes = {
    schema_version: checkpoint.schema_version,
    id: checkpoint.id,
    work_item_id: checkpoint.work_item_id,
    project_id: checkpoint.project_id,
    kind: checkpoint.kind,
    source: checkpoint.source,
    captured_at: checkpoint.captured_at,
    work_period: checkpoint.work_period,
    status_after: checkpoint.status_after,
    related_checkpoint_ids: checkpoint.related_checkpoint_ids,
    correction_of: checkpoint.correction_of,
    confidentiality: checkpoint.confidentiality,
  };

  const body = `# ${checkpoint.title}

## 요약

${checkpoint.summary}

## 진행한 작업

${list(checkpoint.activities)}

## 결정 및 이유

${renderDecisions(checkpoint.decisions)}

## 검증

${renderVerifications(checkpoint.verifications)}

## 결과와 영향

${renderOutcomes(checkpoint.outcomes)}

## 차단 요소

${list(checkpoint.blockers)}

## 다음 작업

${list(checkpoint.next_steps)}

## 근거

${evidenceLines(checkpoint.evidence)}
`;

  return joinFrontmatter(attributes, body);
}

export async function captureCheckpoint({ root, input, validators }) {
  if (!input.work_item_id) {
    throw new CliError("Checkpoint input requires work_item_id", { exitCode: 2 });
  }

  const { workItem, workItemPath, workItemRevision } = await loadWorkItem(
    root,
    String(input.work_item_id),
  );
  const workItemValidation = validators.workItem(workItem);
  if (!workItemValidation.valid) {
    throw new CliError(`Stored work item is invalid: ${workItem.id}`, {
      details: workItemValidation.errors,
    });
  }

  const checkpoint = normalizeCheckpoint(input, workItem);
  const checkpointValidation = validators.checkpoint(checkpoint);
  if (!checkpointValidation.valid) {
    throw new CliError("Checkpoint validation failed", {
      details: checkpointValidation.errors,
    });
  }

  const updated = updatedWorkItem(workItem, checkpoint);
  const updatedValidation = validators.workItem(updated);
  if (!updatedValidation.valid) {
    throw new CliError("Updated work item validation failed", {
      details: updatedValidation.errors,
    });
  }

  const recordPath = checkpointPath(root, checkpoint);
  const markdownPath = recordPath.replace(/\.json$/, ".md");
  if ((await pathExists(recordPath)) || (await pathExists(markdownPath))) {
    throw new CliError(`Checkpoint already exists: ${checkpoint.id}`);
  }

  const {
    contextPath,
    contextDataPath,
    context,
    contextDataRevision,
    contextRevision,
  } = await readContext(root, workItem, validators);
  const updatedContext = mergeContextState(
    context,
    input.context_update,
    checkpoint,
  );
  const contextValidation = validators.workContext(updatedContext);
  if (!contextValidation.valid) {
    throw new CliError("Updated work context validation failed", {
      details: contextValidation.errors,
    });
  }

  await commitFileOperations({
    root,
    operations: [
      createFileOperation(root, recordPath, serializeJson(checkpoint)),
      createFileOperation(
        root,
        markdownPath,
        renderCheckpointMarkdown(checkpoint),
      ),
      replaceFileOperation(
        root,
        workItemPath,
        workItemRevision,
        serializeJson(updated),
      ),
      replaceFileOperation(
        root,
        contextDataPath,
        contextDataRevision,
        serializeJson(updatedContext),
      ),
      replaceFileOperation(
        root,
        contextPath,
        contextRevision,
        renderContext(updated, updatedContext),
      ),
    ],
  });

  return {
    checkpoint,
    work_item: updated,
    context: updatedContext,
    paths: {
      checkpoint: path.relative(root, recordPath),
      checkpoint_markdown: path.relative(root, markdownPath),
      work_item: path.relative(root, workItemPath),
      context_data: path.relative(root, contextDataPath),
      context: path.relative(root, contextPath),
    },
  };
}
