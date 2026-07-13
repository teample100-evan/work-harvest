import { constants } from "node:fs";
import {
  access,
  mkdir,
  readFile,
  readdir,
  rename,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import YAML from "yaml";
import { CliError } from "./errors.js";

export async function pathExists(filePath) {
  try {
    await access(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

export async function readStructuredInput(inputPath) {
  if (!inputPath && process.stdin.isTTY) {
    throw new CliError("--input <file|-> is required when stdin is a TTY", {
      exitCode: 2,
    });
  }

  const text =
    !inputPath || inputPath === "-"
      ? await readStdin()
      : await readFile(path.resolve(inputPath), "utf8");

  if (!text.trim()) {
    throw new CliError("Input is empty", { exitCode: 2 });
  }

  try {
    const value = YAML.parse(text);
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      throw new Error("Expected an object");
    }
    return value;
  } catch (error) {
    throw new CliError(`Could not parse JSON/YAML input: ${error.message}`, {
      exitCode: 2,
    });
  }
}

export async function readJson(filePath) {
  try {
    return JSON.parse(await readFile(filePath, "utf8"));
  } catch (error) {
    throw new CliError(`Could not read JSON file ${filePath}: ${error.message}`);
  }
}

export async function writeJsonExclusive(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  try {
    await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`, {
      encoding: "utf8",
      flag: "wx",
    });
  } catch (error) {
    if (error.code === "EEXIST") {
      throw new CliError(`Refusing to overwrite existing file: ${filePath}`);
    }
    throw error;
  }
}

export async function writeTextExclusive(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  try {
    await writeFile(filePath, value, { encoding: "utf8", flag: "wx" });
  } catch (error) {
    if (error.code === "EEXIST") {
      throw new CliError(`Refusing to overwrite existing file: ${filePath}`);
    }
    throw error;
  }
}

export async function writeJsonAtomic(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporaryPath = `${filePath}.tmp-${process.pid}-${Date.now()}`;
  await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  await rename(temporaryPath, filePath);
}

export function splitFrontmatter(markdown) {
  const match = markdown.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if (!match) {
    throw new CliError("Context document is missing YAML frontmatter");
  }
  return {
    attributes: YAML.parse(match[1]) ?? {},
    body: markdown.slice(match[0].length),
  };
}

export function joinFrontmatter(attributes, body) {
  const yaml = YAML.stringify(attributes, { lineWidth: 0 }).trimEnd();
  return `---\n${yaml}\n---\n\n${body.replace(/^\s*/, "")}`;
}

export async function writeTextAtomic(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporaryPath = `${filePath}.tmp-${process.pid}-${Date.now()}`;
  await writeFile(temporaryPath, value, "utf8");
  await rename(temporaryPath, filePath);
}

export async function listFilesRecursively(directory, predicate = () => true) {
  if (!(await pathExists(directory))) {
    return [];
  }

  const files = [];
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listFilesRecursively(entryPath, predicate)));
    } else if (predicate(entryPath)) {
      files.push(entryPath);
    }
  }
  return files.sort();
}
