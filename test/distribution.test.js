import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  binaryName,
  prepareSidecar,
  resolveTargetTriple,
  sidecarName,
} from "../scripts/prepare-tauri-sidecar.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

test("sidecar names follow the Tauri target-triple convention", () => {
  assert.equal(binaryName("aarch64-apple-darwin"), "wh");
  assert.equal(sidecarName("aarch64-apple-darwin"), "wh-aarch64-apple-darwin");
  assert.equal(binaryName("x86_64-pc-windows-msvc"), "wh.exe");
  assert.equal(
    sidecarName("x86_64-pc-windows-msvc"),
    "wh-x86_64-pc-windows-msvc.exe",
  );
  assert.throws(
    () => resolveTargetTriple("universal-apple-darwin"),
    /separate lipo build/,
  );
});

test("a prepared sidecar preserves the native CLI and executable mode", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "work-harvest-sidecar-"));
  const source = path.join(root, "target", "release", "wh");
  const outputDirectory = path.join(root, "binaries");

  try {
    mkdirSync(path.dirname(source), { recursive: true });
    writeFileSync(source, "native-cli");

    const result = prepareSidecar({
      repositoryRoot: root,
      targetTriple: "aarch64-apple-darwin",
      source,
      outputDirectory,
      build: false,
    });

    assert.equal(
      result.destination,
      path.join(outputDirectory, "wh-aarch64-apple-darwin"),
    );
    assert.equal(readFileSync(result.destination, "utf8"), "native-cli");
    assert.notEqual(statSync(result.destination).mode & 0o111, 0);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("release tags must match every distributed package version", () => {
  const script = path.join(repositoryRoot, "scripts", "verify-release-version.mjs");
  const valid = spawnSync(process.execPath, [script, "v0.2.0"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  const invalid = spawnSync(process.execPath, [script, "v9.9.9"], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });

  assert.equal(valid.status, 0, valid.stderr);
  assert.match(valid.stdout, /Release version verified/);
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /does not match project version/);
});
