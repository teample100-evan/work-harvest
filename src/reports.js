import path from "node:path";
import { CliError } from "./errors.js";
import { resolveWithinRoot } from "./paths.js";
import { createPerformanceNoteOperation } from "./rust-writer.js";

export async function createPerformanceNote({ root, workItemId, output }) {
  if (output != null) {
    const reportPath = resolveWithinRoot(root, output);
    if (path.extname(reportPath) !== ".md") {
      throw new CliError("Report output must be a .md file", { exitCode: 2 });
    }
  }
  try {
    const result = await createPerformanceNoteOperation({
      root,
      input: {
        work_item_id: workItemId,
        output: output ?? null,
      },
      generatedAt: new Date().toISOString(),
    });
    return {
      work_item: result.work_item,
      checkpoint_count: result.checkpoint_count,
      paths: result.paths,
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    const marker = "Refusing to overwrite existing file: ";
    const offset = message.indexOf(marker);
    if (offset >= 0) {
      throw new CliError(
        `Performance note already exists: ${message.slice(offset + marker.length)}`,
      );
    }
    throw error;
  }
}
