#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  statSync,
} from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRepositoryRoot = path.resolve(path.dirname(scriptPath), "..");

export function binaryName(targetTriple) {
  return targetTriple.includes("windows") ? "wh.exe" : "wh";
}

export function sidecarName(targetTriple) {
  const extension = targetTriple.includes("windows") ? ".exe" : "";
  return `wh-${targetTriple}${extension}`;
}

export function rustHostTriple() {
  const output = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const hostLine = output
    .split("\n")
    .find((line) => line.startsWith("host: "));

  if (!hostLine) {
    throw new Error("Could not determine the Rust host target triple.");
  }

  return hostLine.slice("host: ".length).trim();
}

export function resolveTargetTriple(explicitTarget) {
  const target =
    explicitTarget ||
    process.env.WORK_HARVEST_TARGET_TRIPLE ||
    process.env.CARGO_BUILD_TARGET ||
    rustHostTriple();

  if (target === "universal-apple-darwin") {
    throw new Error(
      "Universal macOS sidecars require a separate lipo build; use aarch64-apple-darwin or x86_64-apple-darwin.",
    );
  }

  return target;
}

export function prepareSidecar({
  repositoryRoot = defaultRepositoryRoot,
  targetTriple,
  profile = "release",
  source,
  outputDirectory,
  build = true,
}) {
  const target = resolveTargetTriple(targetTriple);
  const artifact =
    source ||
    path.join(
      repositoryRoot,
      "target",
      target,
      profile,
      binaryName(target),
    );

  if (build) {
    const cargoArgs = [
      "build",
      "--package",
      "work-harvest-cli",
      "--profile",
      profile,
      "--target",
      target,
    ];
    execFileSync("cargo", cargoArgs, {
      cwd: repositoryRoot,
      stdio: "inherit",
    });
  }

  if (!existsSync(artifact) || !statSync(artifact).isFile()) {
    throw new Error(`Native CLI artifact not found: ${artifact}`);
  }

  const binariesDirectory =
    outputDirectory ||
    path.join(repositoryRoot, "apps", "desktop", "src-tauri", "binaries");
  const destination = path.join(binariesDirectory, sidecarName(target));
  mkdirSync(binariesDirectory, { recursive: true });
  copyFileSync(artifact, destination);

  if (!target.includes("windows")) {
    chmodSync(destination, 0o755);
  }

  return { artifact, destination, target };
}

function parseArguments(args) {
  const options = {};

  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--target") {
      options.targetTriple = args[++index];
    } else if (argument === "--source") {
      options.source = path.resolve(args[++index]);
    } else if (argument === "--output-directory") {
      options.outputDirectory = path.resolve(args[++index]);
    } else if (argument === "--skip-build") {
      options.build = false;
    } else {
      throw new Error(`Unknown argument: ${argument}`);
    }
  }

  if (process.env.WORK_HARVEST_SKIP_CLI_BUILD === "1") {
    options.build = false;
  }
  if (process.env.WORK_HARVEST_CLI_ARTIFACT) {
    options.source = path.resolve(process.env.WORK_HARVEST_CLI_ARTIFACT);
  }

  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  try {
    const result = prepareSidecar(parseArguments(process.argv.slice(2)));
    process.stdout.write(
      `Prepared ${result.target} sidecar: ${result.destination}\n`,
    );
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
