#!/usr/bin/env node

import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

function readJson(relativePath) {
  return JSON.parse(
    readFileSync(path.join(repositoryRoot, relativePath), "utf8"),
  );
}

function readCargoVersion(relativePath) {
  const source = readFileSync(path.join(repositoryRoot, relativePath), "utf8");
  const packageSection = source.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)?.[1];
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) {
    throw new Error(`Package version not found in ${relativePath}.`);
  }
  return version;
}

const tag = process.argv[2];
if (!tag) {
  throw new Error("Usage: node scripts/verify-release-version.mjs <vMAJOR.MINOR.PATCH>");
}

const versions = new Map([
  ["package.json", readJson("package.json").version],
  ["apps/desktop/package.json", readJson("apps/desktop/package.json").version],
  [
    "apps/desktop/src-tauri/tauri.conf.json",
    readJson("apps/desktop/src-tauri/tauri.conf.json").version,
  ],
  [
    "apps/desktop/src-tauri/Cargo.toml",
    readCargoVersion("apps/desktop/src-tauri/Cargo.toml"),
  ],
  [
    "crates/work-harvest-cli/Cargo.toml",
    readCargoVersion("crates/work-harvest-cli/Cargo.toml"),
  ],
]);
const uniqueVersions = new Set(versions.values());

if (uniqueVersions.size !== 1) {
  const details = [...versions]
    .map(([file, version]) => `${file}=${version}`)
    .join(", ");
  throw new Error(`Release versions do not match: ${details}`);
}

const [version] = uniqueVersions;
if (tag !== `v${version}`) {
  throw new Error(`Release tag ${tag} does not match project version v${version}.`);
}

process.stdout.write(`Release version verified: ${tag}\n`);
