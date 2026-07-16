#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const defaultAppPath = path.join(
  repositoryRoot,
  "target",
  "release",
  "bundle",
  "macos",
  "Work Harvest.app",
);
const arguments_ = process.argv.slice(2);
const requireDeveloperId = arguments_.includes("--require-developer-id");
const appArgument = arguments_.find(
  (argument) => argument !== "--require-developer-id",
);
const appPath = path.resolve(appArgument || defaultAppPath);
const cliPath = path.join(appPath, "Contents", "MacOS", "wh");

if (!existsSync(appPath) || !statSync(appPath).isDirectory()) {
  throw new Error(`macOS app bundle not found: ${appPath}`);
}
if (!existsSync(cliPath) || !statSync(cliPath).isFile()) {
  throw new Error(`Bundled native CLI not found: ${cliPath}`);
}

const help = execFileSync(cliPath, ["--help"], { encoding: "utf8" });
if (!help.includes("Work Harvest CLI")) {
  throw new Error("Bundled native CLI did not return the expected help output.");
}

execFileSync(
  cliPath,
  ["validate", "--root", path.join(repositoryRoot, "examples"), "--json"],
  { stdio: "ignore" },
);

const signatureResult = spawnSync(
  "codesign",
  ["--display", "--verbose=4", appPath],
  { encoding: "utf8" },
);
if (signatureResult.status !== 0) {
  throw new Error(
    `macOS app bundle is not signed: ${signatureResult.stderr.trim()}`,
  );
}

if (requireDeveloperId) {
  const verification = spawnSync(
    "codesign",
    ["--verify", "--deep", "--strict", "--verbose=2", appPath],
    { encoding: "utf8" },
  );
  if (verification.status !== 0) {
    throw new Error(
      `Developer ID signature verification failed: ${verification.stderr.trim()}`,
    );
  }
  if (!signatureResult.stderr.includes("Authority=Developer ID Application")) {
    throw new Error("macOS app bundle is not signed with Developer ID Application.");
  }
}

process.stdout.write(
  JSON.stringify(
    {
      app: appPath,
      cli: cliPath,
      cli_bytes: statSync(cliPath).size,
      developer_id_required: requireDeveloperId,
      signature: signatureResult.stderr.trim(),
    },
    null,
    2,
  ) + "\n",
);
