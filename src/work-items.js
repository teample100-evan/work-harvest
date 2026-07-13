import { readFile } from "node:fs/promises";
import path from "node:path";
import { CliError } from "./errors.js";
import {
  pathExists,
  readJson,
  writeJsonExclusive,
  writeTextExclusive,
} from "./io.js";
import {
  canonicalContextDataPath,
  canonicalContextPath,
  canonicalWorkItemPath,
  resolveWithinRoot,
} from "./paths.js";

function stringList(values) {
  return (values ?? []).map((value) => String(value));
}

function normalizeFiles(values) {
  return (values ?? []).map((value) =>
    typeof value === "string"
      ? { path: value, description: null }
      : { path: String(value.path), description: value.description ?? null },
  );
}

export function normalizeWorkItem(input, now = new Date().toISOString()) {
  if (!input.id || !input.project_id || !input.title || !input.objective) {
    throw new CliError(
      "Work item input requires id, project_id, title, and objective",
      { exitCode: 2 },
    );
  }

  const contextPath = canonicalContextPath(input.id);
  if (input.context_path && input.context_path !== contextPath) {
    throw new CliError(`context_path must be ${contextPath}`);
  }

  return {
    schema_version: "1.0",
    id: String(input.id),
    project_id: String(input.project_id),
    title: String(input.title),
    status: input.status ?? "planned",
    objective: String(input.objective),
    desired_outcomes: stringList(input.desired_outcomes),
    classification: {
      initiative_id: input.classification?.initiative_id ?? null,
      work_types: stringList(input.classification?.work_types),
      tags: stringList(input.classification?.tags),
    },
    repositories: input.repositories ?? [],
    links: input.links ?? [],
    context_path: contextPath,
    created_at: input.created_at ?? now,
    updated_at: input.updated_at ?? now,
    completed_at:
      input.status === "completed" ? (input.completed_at ?? now) : null,
  };
}

export function normalizeContextState(input, workItem) {
  const value = input ?? {};
  return {
    schema_version: "1.0",
    work_item_id: workItem.id,
    project_id: workItem.project_id,
    updated_at: workItem.updated_at,
    last_checkpoint_id: value.last_checkpoint_id ?? null,
    last_verified_git_ref: value.last_verified_git_ref ?? null,
    current_state:
      value.current_state ??
      "업무 항목을 생성했으며 구체적인 작업을 시작하기 전이다.",
    decisions: stringList(value.decisions),
    files: normalizeFiles(value.files),
    verification: {
      completed: stringList(
        value.verification?.completed ?? value.verification_completed,
      ),
      pending: stringList(
        value.verification?.pending ?? value.verification_pending,
      ),
    },
    next_steps: stringList(value.next_steps),
    risks: stringList(value.risks),
    git: {
      repository: value.git?.repository ?? null,
      branch: value.git?.branch ?? null,
      commit: value.git?.commit ?? null,
      checked_at: value.git?.checked_at ?? null,
    },
  };
}

function bullets(values, fallback) {
  return values.length
    ? values.map((value) => `- ${value}`).join("\n")
    : `- ${fallback}`;
}

function renderFileBullets(values) {
  return values.length
    ? values
        .map(
          (value) =>
            `- \`${value.path}\`${value.description ? `: ${value.description}` : ""}`,
        )
        .join("\n")
    : "- 아직 지정하지 않음";
}

export function renderContext(workItem, context) {
  const frontmatter = [
    "---",
    'schema_version: "1.0"',
    `work_item_id: ${JSON.stringify(workItem.id)}`,
    `project_id: ${JSON.stringify(workItem.project_id)}`,
    `title: ${JSON.stringify(workItem.title)}`,
    `status: ${JSON.stringify(workItem.status)}`,
    `updated_at: ${JSON.stringify(context.updated_at)}`,
    `last_checkpoint_id: ${context.last_checkpoint_id ? JSON.stringify(context.last_checkpoint_id) : "null"}`,
    `last_verified_git_ref: ${context.last_verified_git_ref ? JSON.stringify(context.last_verified_git_ref) : "null"}`,
    "---",
  ].join("\n");

  return `${frontmatter}

# ${workItem.id} ${workItem.title}

## 목표

${workItem.objective}

## 현재 상태

${context.current_state}

## 주요 결정과 이유

${bullets(context.decisions, "아직 확정된 결정 없음")}

## 주요 파일과 문서

${renderFileBullets(context.files)}

## 검증 상태

### 완료

${bullets(context.verification.completed, "완료된 검증 없음")}

### 미완료

${bullets(context.verification.pending, "예정된 검증 없음")}

## 남은 작업

${bullets(context.next_steps, "다음 작업을 구체화해야 함")}

## 리스크와 확인할 사항

${bullets(context.risks, "현재 확인된 리스크 없음")}

## 마지막으로 확인한 Git 기준점

- 저장소: ${context.git.repository ? `\`${context.git.repository}\`` : "지정하지 않음"}
- 브랜치: ${context.git.branch ? `\`${context.git.branch}\`` : "지정하지 않음"}
- 커밋: ${context.git.commit ? `\`${context.git.commit}\`` : "지정하지 않음"}
- 확인 시각: ${context.git.checked_at ?? "지정하지 않음"}
`;
}

export async function createWorkItem({ root, input, validators }) {
  const workItem = normalizeWorkItem(input);
  const context = normalizeContextState(input.context, workItem);

  const workItemValidation = validators.workItem(workItem);
  if (!workItemValidation.valid) {
    throw new CliError("Work item validation failed", {
      details: workItemValidation.errors,
    });
  }
  const contextValidation = validators.workContext(context);
  if (!contextValidation.valid) {
    throw new CliError("Work context validation failed", {
      details: contextValidation.errors,
    });
  }

  const workItemPath = canonicalWorkItemPath(root, workItem.id);
  const contextPath = resolveWithinRoot(root, workItem.context_path);
  const contextDataPath = canonicalContextDataPath(root, workItem.id);
  if (
    (await pathExists(workItemPath)) ||
    (await pathExists(contextPath)) ||
    (await pathExists(contextDataPath))
  ) {
    throw new CliError(`Work item already exists: ${workItem.id}`);
  }

  await writeJsonExclusive(contextDataPath, context);
  await writeTextExclusive(contextPath, renderContext(workItem, context));
  await writeJsonExclusive(workItemPath, workItem);

  return {
    work_item: workItem,
    context,
    paths: {
      work_item: path.relative(root, workItemPath),
      context_data: path.relative(root, contextDataPath),
      context: path.relative(root, contextPath),
    },
  };
}

export async function loadWorkItem(root, workItemId) {
  const workItemPath = canonicalWorkItemPath(root, workItemId);
  if (!(await pathExists(workItemPath))) {
    throw new CliError(`Unknown work item: ${workItemId}`);
  }
  return { workItem: await readJson(workItemPath), workItemPath };
}

export async function readContext(root, workItem, validators) {
  const contextPath = resolveWithinRoot(root, workItem.context_path);
  const contextDataPath = canonicalContextDataPath(root, workItem.id);
  if (!(await pathExists(contextPath))) {
    throw new CliError(`Missing context document: ${workItem.context_path}`);
  }
  if (!(await pathExists(contextDataPath))) {
    throw new CliError(
      `Missing structured context: ${path.relative(root, contextDataPath)}`,
    );
  }

  const context = await readJson(contextDataPath);
  if (validators) {
    const validation = validators.workContext(context);
    if (!validation.valid) {
      throw new CliError(`Stored work context is invalid: ${workItem.id}`, {
        details: validation.errors,
      });
    }
  }

  return {
    contextPath,
    contextDataPath,
    context,
    markdown: await readFile(contextPath, "utf8"),
  };
}
