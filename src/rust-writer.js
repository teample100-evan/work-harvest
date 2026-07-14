import { spawn } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import path from "node:path";
import { CliError } from "./errors.js";
import { packageRoot } from "./paths.js";

const protocolVersion = 1;

function newestModifiedAt(target) {
  try {
    const stat = statSync(target);
    if (!stat.isDirectory()) return stat.mtimeMs;
    return readdirSync(target, { withFileTypes: true }).reduce(
      (latest, entry) =>
        Math.max(latest, newestModifiedAt(path.join(target, entry.name))),
      stat.mtimeMs,
    );
  } catch {
    return Number.POSITIVE_INFINITY;
  }
}

function freshDebugHelper() {
  const executable = path.join(
    packageRoot,
    "target",
    "debug",
    `work-harvest-write-helper${process.platform === "win32" ? ".exe" : ""}`,
  );
  let executableModifiedAt;
  try {
    executableModifiedAt = statSync(executable).mtimeMs;
  } catch {
    return null;
  }
  const sourceModifiedAt = Math.max(
    newestModifiedAt(path.join(packageRoot, "Cargo.toml")),
    newestModifiedAt(path.join(packageRoot, "Cargo.lock")),
    newestModifiedAt(
      path.join(packageRoot, "crates", "work-harvest-core", "Cargo.toml"),
    ),
    newestModifiedAt(
      path.join(packageRoot, "crates", "work-harvest-core", "src"),
    ),
    newestModifiedAt(
      path.join(
        packageRoot,
        "crates",
        "work-harvest-write-helper",
        "Cargo.toml",
      ),
    ),
    newestModifiedAt(
      path.join(packageRoot, "crates", "work-harvest-write-helper", "src"),
    ),
  );
  return executableModifiedAt >= sourceModifiedAt ? executable : null;
}

function helperCommand() {
  if (process.env.WORK_HARVEST_WRITE_HELPER) {
    return {
      command: path.resolve(process.env.WORK_HARVEST_WRITE_HELPER),
      args: [],
    };
  }
  const debugHelper = freshDebugHelper();
  if (debugHelper) {
    return { command: debugHelper, args: [] };
  }
  return {
    command: process.env.CARGO ?? "cargo",
    args: [
      "run",
      "--quiet",
      "--manifest-path",
      path.join(packageRoot, "Cargo.toml"),
      "--package",
      "work-harvest-write-helper",
      "--",
    ],
  };
}

function protocolPath(root, filePath) {
  const relative = path.relative(path.resolve(root), path.resolve(filePath));
  if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`)) {
    throw new CliError(`Write path escapes the data root: ${filePath}`);
  }
  return relative.split(path.sep).join("/");
}

export function createFileOperation(root, filePath, contents) {
  return {
    path: protocolPath(root, filePath),
    contents,
    expectation: "create",
  };
}

export function replaceFileOperation(root, filePath, revision, contents) {
  if (!revision?.sha256) {
    throw new CliError(`Missing file revision: ${filePath}`);
  }
  return {
    path: protocolPath(root, filePath),
    contents,
    expectation: "match_sha256",
    expected_sha256: revision.sha256,
  };
}

async function runHelper(root, payload) {
  const { command, args } = helperCommand();
  const request = JSON.stringify({
    protocol_version: protocolVersion,
    root: path.resolve(root),
    ...payload,
  });

  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: packageRoot,
      stdio: ["pipe", "pipe", "pipe"],
    });
    const stdout = [];
    const stderr = [];
    let spawnError;

    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => {
      spawnError = error;
    });
    child.on("close", (code) => {
      if (spawnError) {
        reject(
          new CliError(`Could not start Rust write helper: ${spawnError.message}`),
        );
        return;
      }
      const errorText = Buffer.concat(stderr).toString("utf8").trim();
      if (code !== 0) {
        reject(
          new CliError(
            `Rust write helper failed${errorText ? `: ${errorText}` : ""}`,
          ),
        );
        return;
      }
      try {
        const response = JSON.parse(Buffer.concat(stdout).toString("utf8"));
        if (response.protocol_version !== protocolVersion) {
          throw new Error(
            `Unexpected protocol version: ${response.protocol_version}`,
          );
        }
        resolve(response);
      } catch (error) {
        reject(
          new CliError(`Invalid Rust write helper response: ${error.message}`),
        );
      }
    });
    child.stdin.on("error", () => {
      // The close handler reports the helper failure and captured stderr.
    });
    child.stdin.end(request);
  });
}

export async function commitFileOperations({ root, operations }) {
  const response = await runHelper(root, { operations });
  if (!response.commit) {
    throw new CliError("Rust write helper omitted the file commit result");
  }
  return response.commit;
}

export async function captureCheckpointOperation({
  root,
  input,
  expected,
  now,
}) {
  const response = await runHelper(root, {
    checkpoint_capture: { input, expected, now },
  });
  if (!response.checkpoint_capture) {
    throw new CliError("Rust write helper omitted the checkpoint result");
  }
  return response.checkpoint_capture;
}
