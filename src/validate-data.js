import path from "node:path";
import { readFile } from "node:fs/promises";
import {
  listFilesRecursively,
  pathExists,
  readJson,
  splitFrontmatter,
} from "./io.js";
import { canonicalContextDataPath, resolveWithinRoot } from "./paths.js";
import { calendarParts } from "./time.js";

function addSchemaErrors(errors, filePath, result) {
  for (const message of result.errors) {
    errors.push({ file: filePath, message });
  }
}

function expectedRecordDirectory(checkpoint) {
  const timezone = checkpoint.work_period?.timezone ?? "Asia/Seoul";
  const { year, month, day } = calendarParts(checkpoint.captured_at, timezone);
  return path.join("records", year, month, day);
}

export async function validateDataRoot({ root, validators, label = "root" }) {
  const workItemFiles = await listFilesRecursively(
    path.join(root, "work-items"),
    (filePath) => path.basename(filePath) === "work-item.json",
  );
  const checkpointFiles = await listFilesRecursively(
    path.join(root, "records"),
    (filePath) => filePath.endsWith(".json"),
  );

  const errors = [];
  const workItems = new Map();
  const checkpoints = new Map();
  let contextCount = 0;

  for (const filePath of workItemFiles) {
    try {
      const value = await readJson(filePath);
      const result = validators.workItem(value);
      if (!result.valid) {
        addSchemaErrors(errors, filePath, result);
        continue;
      }
      if (workItems.has(value.id)) {
        errors.push({ file: filePath, message: `Duplicate work item id: ${value.id}` });
      }

      const contextPath = resolveWithinRoot(root, value.context_path);
      const contextDataPath = canonicalContextDataPath(root, value.id);
      if (!(await pathExists(contextPath))) {
        errors.push({ file: filePath, message: `Missing context: ${value.context_path}` });
      }
      if (!(await pathExists(contextDataPath))) {
        errors.push({
          file: filePath,
          message: `Missing structured context: ${path.relative(root, contextDataPath)}`,
        });
      } else {
        const context = await readJson(contextDataPath);
        contextCount += 1;
        const contextResult = validators.workContext(context);
        if (!contextResult.valid) {
          addSchemaErrors(errors, contextDataPath, contextResult);
        }
        if (context.work_item_id !== value.id || context.project_id !== value.project_id) {
          errors.push({
            file: contextDataPath,
            message: "Context identity does not match its work item",
          });
        }
        if (await pathExists(contextPath)) {
          const { attributes } = splitFrontmatter(await readFile(contextPath, "utf8"));
          if (attributes.work_item_id !== value.id) {
            errors.push({
              file: contextPath,
              message: "Context Markdown frontmatter has the wrong work_item_id",
            });
          }
        }
        if (!workItems.has(value.id)) {
          workItems.set(value.id, { value, filePath, context, contextDataPath });
        }
      }
    } catch (error) {
      errors.push({ file: filePath, message: error.message });
    }
  }

  for (const filePath of checkpointFiles) {
    try {
      const value = await readJson(filePath);
      const result = validators.checkpoint(value);
      if (!result.valid) {
        addSchemaErrors(errors, filePath, result);
        continue;
      }
      if (checkpoints.has(value.id)) {
        errors.push({ file: filePath, message: `Duplicate checkpoint id: ${value.id}` });
      } else {
        checkpoints.set(value.id, { value, filePath });
      }

      const relativeDirectory = path.dirname(path.relative(root, filePath));
      if (relativeDirectory !== expectedRecordDirectory(value)) {
        errors.push({
          file: filePath,
          message: `Record is in ${relativeDirectory}; expected ${expectedRecordDirectory(value)}`,
        });
      }
      const markdownPath = filePath.replace(/\.json$/, ".md");
      if (!(await pathExists(markdownPath))) {
        errors.push({
          file: filePath,
          message: `Missing checkpoint Markdown: ${path.relative(root, markdownPath)}`,
        });
      } else {
        const { attributes } = splitFrontmatter(await readFile(markdownPath, "utf8"));
        if (attributes.id !== value.id || attributes.work_item_id !== value.work_item_id) {
          errors.push({
            file: markdownPath,
            message: "Checkpoint Markdown frontmatter does not match JSON",
          });
        }
      }
    } catch (error) {
      errors.push({ file: filePath, message: error.message });
    }
  }

  for (const { value, filePath } of checkpoints.values()) {
    if (!workItems.has(value.work_item_id)) {
      errors.push({
        file: filePath,
        message: `Unknown work_item_id: ${value.work_item_id}`,
      });
    } else if (workItems.get(value.work_item_id).value.project_id !== value.project_id) {
      errors.push({
        file: filePath,
        message: `project_id does not match work item ${value.work_item_id}`,
      });
    }

    if (value.correction_of && !checkpoints.has(value.correction_of)) {
      errors.push({
        file: filePath,
        message: `Unknown correction_of checkpoint: ${value.correction_of}`,
      });
    }
    for (const relatedId of value.related_checkpoint_ids) {
      if (!checkpoints.has(relatedId)) {
        errors.push({
          file: filePath,
          message: `Unknown related checkpoint: ${relatedId}`,
        });
      }
    }
  }

  for (const { value, context, contextDataPath } of workItems.values()) {
    if (context.last_checkpoint_id) {
      const checkpoint = checkpoints.get(context.last_checkpoint_id)?.value;
      if (!checkpoint) {
        errors.push({
          file: contextDataPath,
          message: `Unknown last_checkpoint_id: ${context.last_checkpoint_id}`,
        });
      } else if (checkpoint.work_item_id !== value.id) {
        errors.push({
          file: contextDataPath,
          message: `last_checkpoint_id belongs to another work item: ${context.last_checkpoint_id}`,
        });
      }
    }
  }

  return {
    label,
    valid: errors.length === 0,
    counts: {
      work_items: workItemFiles.length,
      contexts: contextCount,
      checkpoints: checkpointFiles.length,
    },
    errors,
  };
}
