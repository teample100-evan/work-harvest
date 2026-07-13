import path from "node:path";
import { fileURLToPath } from "node:url";

export const packageRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

export function resolveDataRoot(value) {
  return path.resolve(value ?? process.env.WORK_HARNESS_HOME ?? process.cwd());
}

export function resolveWithinRoot(root, relativePath) {
  const resolvedRoot = path.resolve(root);
  const resolvedPath = path.resolve(resolvedRoot, relativePath);
  if (
    resolvedPath !== resolvedRoot &&
    !resolvedPath.startsWith(`${resolvedRoot}${path.sep}`)
  ) {
    throw new Error(`Path escapes the data root: ${relativePath}`);
  }
  return resolvedPath;
}

export function canonicalWorkItemPath(root, workItemId) {
  return resolveWithinRoot(root, `work-items/${workItemId}/work-item.json`);
}

export function canonicalContextPath(workItemId) {
  return `work-items/${workItemId}/context.md`;
}

export function canonicalContextDataPath(root, workItemId) {
  return resolveWithinRoot(root, `work-items/${workItemId}/context.json`);
}
