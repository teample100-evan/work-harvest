import path from "node:path";
import { parseArgs } from "node:util";
import { captureCheckpoint } from "./checkpoints.js";
import { CliError } from "./errors.js";
import { readStructuredInput } from "./io.js";
import { packageRoot, resolveDataRoot } from "./paths.js";
import {
  findLastCheckpoint,
  listWorkItems,
  showWorkItem,
} from "./queries.js";
import { createSchemaValidators } from "./schema-validator.js";
import { validateDataRoot } from "./validate-data.js";
import { createWorkItem } from "./work-items.js";

const usage = `Work Harness CLI

Usage:
  wh work-item create --input <file|-> [--root <path>] [--json]
  wh work-item list [--project <id>] [--status <status>] [--root <path>] [--json]
  wh work-item show <id> [--root <path>] [--json]
  wh checkpoint capture --input <file|-> [--root <path>] [--json]
  wh checkpoint last --work-item <id> [--root <path>] [--json]
  wh validate [--root <path>] [--include-examples] [--json]

Environment:
  WORK_HARNESS_HOME  Default data root when --root is omitted
`;

function parseCommandOptions(
  args,
  { input = false, examples = false, positionals = false, extra = {} } = {},
) {
  const options = {
    root: { type: "string" },
    json: { type: "boolean", default: false },
    help: { type: "boolean", short: "h", default: false },
    ...extra,
  };
  if (input) options.input = { type: "string", short: "i" };
  if (examples) {
    options["include-examples"] = { type: "boolean", default: false };
  }
  try {
    return parseArgs({
      args,
      options,
      strict: true,
      allowPositionals: positionals,
    });
  } catch (error) {
    throw new CliError(error.message, { exitCode: 2 });
  }
}

function printResult(result, asJson, message) {
  if (asJson) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  } else {
    process.stdout.write(`${message}\n`);
  }
}

function printError(error, asJson) {
  const details = error.details ?? [];
  if (asJson) {
    process.stderr.write(
      `${JSON.stringify({ error: error.message, details }, null, 2)}\n`,
    );
    return;
  }
  process.stderr.write(`Error: ${error.message}\n`);
  for (const detail of details) {
    process.stderr.write(
      `  - ${typeof detail === "string" ? detail : `${detail.file}: ${detail.message}`}\n`,
    );
  }
}

async function handleValidate(args, validators) {
  const { values: options } = parseCommandOptions(args, { examples: true });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  const root = resolveDataRoot(options.root);
  const results = [await validateDataRoot({ root, validators })];
  if (options["include-examples"]) {
    results.push(
      await validateDataRoot({
        root: path.join(packageRoot, "examples"),
        validators,
        label: "examples",
      }),
    );
  }
  const errors = results.flatMap((result) => result.errors);
  if (errors.length > 0) {
    throw new CliError("Validation failed", { details: errors });
  }
  const result = {
    valid: true,
    datasets: results.map(({ label, counts }) => ({ label, counts })),
  };
  const counts = results.reduce(
    (total, item) => ({
      work_items: total.work_items + item.counts.work_items,
      contexts: total.contexts + item.counts.contexts,
      checkpoints: total.checkpoints + item.counts.checkpoints,
    }),
    { work_items: 0, contexts: 0, checkpoints: 0 },
  );
  printResult(
    result,
    options.json,
    `Valid: ${counts.work_items} work item(s), ${counts.contexts} context(s), ${counts.checkpoints} checkpoint(s)`,
  );
}

async function handleCreateWorkItem(args, validators) {
  const { values: options } = parseCommandOptions(args, { input: true });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  const root = resolveDataRoot(options.root);
  const input = await readStructuredInput(options.input);
  const result = await createWorkItem({ root, input, validators });
  printResult(
    result,
    options.json,
    `Created work item ${result.work_item.id}\n  metadata: ${result.paths.work_item}\n  context data: ${result.paths.context_data}\n  context: ${result.paths.context}`,
  );
}

async function handleListWorkItems(args, validators) {
  const { values: options } = parseCommandOptions(args, {
    extra: {
      project: { type: "string" },
      status: { type: "string" },
    },
  });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  const items = await listWorkItems({
    root: resolveDataRoot(options.root),
    validators,
    projectId: options.project,
    status: options.status,
  });
  const message = items.length
    ? items
        .map(
          (item) =>
            `${item.id}\t${item.status}\t${item.project_id}\t${item.title}\n  ${item.current_state}`,
        )
        .join("\n")
    : "No work items found";
  printResult({ work_items: items }, options.json, message);
}

async function handleShowWorkItem(args, validators) {
  const { values: options, positionals } = parseCommandOptions(args, {
    positionals: true,
  });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  if (positionals.length !== 1) {
    throw new CliError("work-item show requires exactly one work item id", {
      exitCode: 2,
    });
  }
  const result = await showWorkItem({
    root: resolveDataRoot(options.root),
    validators,
    workItemId: positionals[0],
  });
  const last = result.last_checkpoint?.checkpoint;
  printResult(
    result,
    options.json,
    `${result.work_item.id} ${result.work_item.title}\n  status: ${result.work_item.status}\n  current: ${result.context.current_state}\n  last checkpoint: ${last ? `${last.id} (${last.captured_at})` : "none"}`,
  );
}

async function handleCaptureCheckpoint(args, validators) {
  const { values: options } = parseCommandOptions(args, { input: true });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  const root = resolveDataRoot(options.root);
  const input = await readStructuredInput(options.input);
  const result = await captureCheckpoint({ root, input, validators });
  printResult(
    result,
    options.json,
    `Captured checkpoint ${result.checkpoint.id}\n  record: ${result.paths.checkpoint}\n  Markdown: ${result.paths.checkpoint_markdown}\n  work item: ${result.work_item.status}`,
  );
}

async function handleLastCheckpoint(args, validators) {
  const { values: options } = parseCommandOptions(args, {
    extra: {
      "work-item": { type: "string" },
    },
  });
  if (options.help) {
    process.stdout.write(usage);
    return;
  }
  if (!options["work-item"]) {
    throw new CliError("checkpoint last requires --work-item <id>", {
      exitCode: 2,
    });
  }
  const root = resolveDataRoot(options.root);
  const result = await findLastCheckpoint({
    root,
    validators,
    workItemId: options["work-item"],
  });
  const output = result ?? { checkpoint: null, paths: null };
  printResult(
    output,
    options.json,
    result
      ? `${result.checkpoint.id}\t${result.checkpoint.captured_at}\t${result.checkpoint.title}`
      : `No checkpoint found for ${options["work-item"]}`,
  );
}

export async function runCli(args) {
  const asJson = args.includes("--json");
  try {
    if (args.length === 0 || args[0] === "--help" || args[0] === "-h") {
      process.stdout.write(usage);
      return;
    }

    const validators = await createSchemaValidators();
    if (args[0] === "validate") {
      await handleValidate(args.slice(1), validators);
      return;
    }
    if (args[0] === "work-item" && args[1] === "create") {
      await handleCreateWorkItem(args.slice(2), validators);
      return;
    }
    if (args[0] === "work-item" && args[1] === "list") {
      await handleListWorkItems(args.slice(2), validators);
      return;
    }
    if (args[0] === "work-item" && args[1] === "show") {
      await handleShowWorkItem(args.slice(2), validators);
      return;
    }
    if (args[0] === "checkpoint" && args[1] === "capture") {
      await handleCaptureCheckpoint(args.slice(2), validators);
      return;
    }
    if (args[0] === "checkpoint" && args[1] === "last") {
      await handleLastCheckpoint(args.slice(2), validators);
      return;
    }

    throw new CliError(`Unknown command: ${args.join(" ")}`, { exitCode: 2 });
  } catch (error) {
    const cliError =
      error instanceof CliError
        ? error
        : new CliError(error.message ?? String(error));
    printError(cliError, asJson);
    if (cliError.exitCode === 2) {
      process.stderr.write(usage);
    }
    process.exitCode = cliError.exitCode;
  }
}
