import { CliError } from "./errors.js";
import { captureCheckpointOperation } from "./rust-writer.js";
import { loadWorkItem, readContext } from "./work-items.js";

export async function captureCheckpoint({ root, input, validators }) {
  if (!input.work_item_id) {
    throw new CliError("Checkpoint input requires work_item_id", { exitCode: 2 });
  }

  const { workItem, workItemRevision } = await loadWorkItem(
    root,
    String(input.work_item_id),
  );
  const workItemValidation = validators.workItem(workItem);
  if (!workItemValidation.valid) {
    throw new CliError(`Stored work item is invalid: ${workItem.id}`, {
      details: workItemValidation.errors,
    });
  }

  const { contextDataRevision, contextRevision } = await readContext(
    root,
    workItem,
    validators,
  );
  const result = await captureCheckpointOperation({
    root,
    input,
    expected: {
      work_item: workItemRevision,
      context_data: contextDataRevision,
      context: contextRevision,
    },
    now: new Date().toISOString(),
  });

  return {
    checkpoint: result.checkpoint,
    work_item: result.work_item,
    context: result.context,
    paths: result.paths,
  };
}
