import path from "node:path";
import { CliError } from "./errors.js";
import { listFilesRecursively, readJson } from "./io.js";
import { loadWorkItem, readContext } from "./work-items.js";

async function loadCheckpoints(root, validators) {
  const files = await listFilesRecursively(
    path.join(root, "records"),
    (filePath) => filePath.endsWith(".json"),
  );
  const checkpoints = [];
  for (const filePath of files) {
    const checkpoint = await readJson(filePath);
    const validation = validators.checkpoint(checkpoint);
    if (!validation.valid) {
      throw new CliError(`Stored checkpoint is invalid: ${filePath}`, {
        details: validation.errors,
      });
    }
    checkpoints.push({
      checkpoint,
      paths: {
        json: path.relative(root, filePath),
        markdown: path.relative(root, filePath.replace(/\.json$/, ".md")),
      },
    });
  }
  return checkpoints;
}

export async function listCheckpointsForWorkItem({
  root,
  validators,
  workItemId,
}) {
  await loadWorkItem(root, workItemId);
  return (await loadCheckpoints(root, validators))
    .filter((entry) => entry.checkpoint.work_item_id === workItemId)
    .sort((left, right) => {
      const byTime = left.checkpoint.captured_at.localeCompare(
        right.checkpoint.captured_at,
      );
      return byTime || left.checkpoint.id.localeCompare(right.checkpoint.id);
    });
}

export async function findLastCheckpoint({ root, validators, workItemId }) {
  await loadWorkItem(root, workItemId);
  const checkpoints = await listCheckpointsForWorkItem({
    root,
    validators,
    workItemId,
  });
  checkpoints.reverse();
  return checkpoints[0] ?? null;
}

export async function listWorkItems({
  root,
  validators,
  projectId,
  status,
}) {
  const files = await listFilesRecursively(
    path.join(root, "work-items"),
    (filePath) => path.basename(filePath) === "work-item.json",
  );
  const items = [];
  for (const filePath of files) {
    const workItem = await readJson(filePath);
    const validation = validators.workItem(workItem);
    if (!validation.valid) {
      throw new CliError(`Stored work item is invalid: ${filePath}`, {
        details: validation.errors,
      });
    }
    if (projectId && workItem.project_id !== projectId) continue;
    if (status && workItem.status !== status) continue;

    const { context } = await readContext(root, workItem, validators);
    items.push({
      id: workItem.id,
      project_id: workItem.project_id,
      title: workItem.title,
      status: workItem.status,
      scope: workItem.scope ?? "unclassified",
      reporting_mode: workItem.reporting?.mode ?? "primary",
      initiative_id: workItem.classification.initiative_id,
      updated_at: workItem.updated_at,
      current_state: context.current_state,
      next_steps: context.next_steps,
      last_checkpoint_id: context.last_checkpoint_id,
    });
  }

  return items.sort((left, right) => {
    const byTime = right.updated_at.localeCompare(left.updated_at);
    return byTime || left.id.localeCompare(right.id);
  });
}

export async function showWorkItem({ root, validators, workItemId }) {
  const { workItem, workItemPath } = await loadWorkItem(root, workItemId);
  const validation = validators.workItem(workItem);
  if (!validation.valid) {
    throw new CliError(`Stored work item is invalid: ${workItemId}`, {
      details: validation.errors,
    });
  }
  const contextResult = await readContext(root, workItem, validators);
  const lastCheckpoint = await findLastCheckpoint({
    root,
    validators,
    workItemId,
  });
  const normalizedWorkItem = {
    schema_version: workItem.schema_version,
    id: workItem.id,
    project_id: workItem.project_id,
    title: workItem.title,
    status: workItem.status,
    objective: workItem.objective,
    ...(workItem.problem ? { problem: workItem.problem } : {}),
    desired_outcomes: workItem.desired_outcomes,
    classification: workItem.classification,
    scope: workItem.scope ?? "unclassified",
    reporting: workItem.reporting ?? {
      mode: "primary",
      exclusion_reason: null,
    },
    repositories: workItem.repositories,
    links: workItem.links,
    external_refs: workItem.external_refs ?? [],
    ...(workItem.completion ? { completion: workItem.completion } : {}),
    context_path: workItem.context_path,
    created_at: workItem.created_at,
    updated_at: workItem.updated_at,
    completed_at: workItem.completed_at,
  };
  return {
    work_item: normalizedWorkItem,
    context: contextResult.context,
    last_checkpoint: lastCheckpoint,
    paths: {
      work_item: path.relative(root, workItemPath),
      context_data: path.relative(root, contextResult.contextDataPath),
      context: path.relative(root, contextResult.contextPath),
    },
  };
}
