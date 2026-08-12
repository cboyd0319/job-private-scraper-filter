// Enforces concise responsibility descriptions at the top of maintained comment-capable files.
import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readFileSync, realpathSync } from "node:fs";
import { extname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { listChangedFiles, listGovernedFiles, normalizeRepoPath } from "./repo-file-size.mjs";

const policyPath = "scripts/harness/contracts/repository-structure.json";
const linePrefixes = new Map([
  [".rs", ["//"]], [".ts", ["//"]], [".tsx", ["//"]], [".js", ["//"]],
  [".mjs", ["//"]], [".cjs", ["//"]], [".jsx", ["//"]], [".cs", ["//"]],
  [".php", ["//"]], [".py", ["#"]], [".sh", ["#"]], [".ps1", ["#"]],
  [".toml", ["#"]], [".yml", ["#"]], [".yaml", ["#"]], [".tf", ["#"]],
  [".tfvars", ["#"]], [".hcl", ["#"]], [".sql", ["--"]], [".ini", [";", "#"]],
  [".txt", ["#"]],
]);
const blockDelimiters = new Map([
  [".rs", [["/*", "*/"]]], [".ts", [["/*", "*/"]]], [".tsx", [["/*", "*/"]]],
  [".js", [["/*", "*/"]]], [".mjs", [["/*", "*/"]]], [".cjs", [["/*", "*/"]]],
  [".jsx", [["/*", "*/"]]], [".cs", [["/*", "*/"]]], [".php", [["/*", "*/"]]],
  [".css", [["/*", "*/"]]], [".sql", [["/*", "*/"]]], [".ps1", [["<#", "#>"]]],
  [".md", [["<!--", "-->"]]], [".html", [["<!--", "-->"]]],
  [".xml", [["<!--", "-->"]]],
  [".props", [["<!--", "-->"]]], [".targets", [["<!--", "-->"]]],
  [".csproj", [["<!--", "-->"]]], [".slnx", [["<!--", "-->"]]],
]);
const hashCommentFiles = new Set([".claudeignore", ".gitattributes", ".gitignore", "CODEOWNERS"]);

function excluded(path, rows) {
  return rows.some((row) => {
    const owner = normalizeRepoPath(String(row?.path ?? ""));
    return owner && (path === owner || path.startsWith(`${owner}/`));
  });
}

function firstDescriptionLine(lines, extension) {
  let index = 0;
  if (lines[index]?.startsWith("#!")) index += 1;
  if (extension === ".rs") while (/^\s*#!\[/.test(lines[index] ?? "")) index += 1;
  if ([".ts", ".tsx"].includes(extension)) {
    while (/^\s*\/\/\/\s*<(?:reference|amd-module|amd-dependency)\b/i.test(lines[index] ?? "")) index += 1;
  }
  if (extension === ".py" && /^#.*coding[:=]/i.test(lines[index] ?? "")) index += 1;
  if (extension === ".sh" && /^#\s*shellcheck\b/i.test(lines[index] ?? "")) index += 1;
  if (extension === ".ps1") while (/^#requires\b/i.test(lines[index] ?? "")) index += 1;
  if ([".yml", ".yaml"].includes(extension)) {
    while (lines[index]?.trimStart().startsWith("%")) index += 1;
    if (lines[index]?.trim() === "---") index += 1;
  }
  if (extension === ".md" && lines[index]?.trim() === "---") {
    const close = lines.findIndex((line, candidate) => candidate > index && line.trim() === "---");
    index = close < 0 ? index : close + 1;
  }
  if (extension === ".php" && lines[index]?.trim().startsWith("<?php")) index += 1;
  if ([".xml", ".props", ".targets", ".csproj", ".slnx"].includes(extension) && /^\s*<\?xml\b/.test(lines[index] ?? "")) index += 1;
  if (extension === ".html" && /^\s*<!doctype\b/i.test(lines[index] ?? "")) index += 1;
  if (extension === ".css" && /^\s*@charset\b/i.test(lines[index] ?? "")) index += 1;
  return index;
}

function lineDescription(lines, index, prefixes) {
  const prefix = prefixes.find((candidate) => lines[index]?.trimStart().startsWith(candidate));
  if (!prefix || (prefix === "#" && lines[index]?.trimStart().startsWith("#!"))) return null;
  const parts = [];
  let cursor = index;
  while (lines[cursor]?.trimStart().startsWith(prefix)) {
    parts.push(lines[cursor].trimStart().slice(prefix.length).replace(/^[!/]?\s?/, "").trim());
    cursor += 1;
  }
  return { lines: parts.length, text: parts.join(" ").trim() };
}

function blockDescription(lines, index, delimiters) {
  for (const [open, close] of delimiters) {
    if (!lines[index]?.trimStart().startsWith(open)) continue;
    const parts = [];
    for (let cursor = index; cursor < lines.length; cursor += 1) {
      const line = lines[cursor];
      parts.push(line);
      if (line.includes(close)) {
        const text = parts.join("\n")
          .replace(open, "").replace(close, "")
          .split("\n").map((part) => part.replace(/^\s*\*?\s?/, "").trim()).join(" ").trim();
        return { lines: parts.length, text };
      }
      if (parts.length > 2) return { lines: parts.length, text: "" };
    }
    return { lines: parts.length, text: "" };
  }
  return null;
}

function descriptionViolation(root, path, classifications) {
  const name = path.split("/").at(-1);
  const extension = path === ".husky/pre-commit" ? ".sh" : extname(path).toLowerCase();
  if (classifications.commentlessPaths.has(path) || classifications.commentlessExtensions.has(extension) || classifications.binaryExtensions.has(extension)) return null;
  const prefixes = hashCommentFiles.has(name) ? ["#"] : (linePrefixes.get(extension) ?? []);
  const delimiters = blockDelimiters.get(extension) ?? [];
  if (prefixes.length === 0 && delimiters.length === 0) return `${path} has no registered native comment syntax or canonical commentless classification`;
  let lines;
  try {
    const fullPath = join(root, path);
    const stat = lstatSync(fullPath);
    const resolvedPath = realpathSync(fullPath);
    const resolvedRelative = relative(realpathSync(root), resolvedPath);
    if (!stat.isFile() || stat.isSymbolicLink() || resolvedRelative.startsWith("..") || isAbsolute(resolvedRelative)) throw new Error("path is not a regular file inside the repository");
    lines = readFileSync(fullPath, "utf8").replace(/^\uFEFF/, "").split(/\r?\n/);
  } catch (error) {
    return `${path} must be a readable regular file inside the repository: ${error instanceof Error ? error.message : String(error)}`;
  }
  const index = firstDescriptionLine(lines, extension);
  const description = lineDescription(lines, index, prefixes) ?? blockDescription(lines, index, delimiters);
  if (!description) return `${path} must start with a one- or two-line native comment describing its exact responsibility`;
  if (description.lines > 2) return `${path} description must be one or two lines`;
  if (description.text.length < 12 || !/[A-Za-z]/.test(description.text) || /\b(?:TODO|FIXME|placeholder)\b/i.test(description.text)) {
    return `${path} description must state its exact responsibility in substantive text`;
  }
  return null;
}

export function collectFileDescriptionViolations(root, options = {}) {
  let files;
  try {
    const exec = options.execFileSync ?? execFileSync;
    files = options.paths ?? (options.all ? listGovernedFiles(root, exec) : options.staged ? listStagedFiles(root, exec) : listChangedFiles(root, exec));
  } catch (error) {
    return [`file-description scan could not enumerate repository files: ${error instanceof Error ? error.message : String(error)}`];
  }
  let policy;
  try {
    policy = JSON.parse(readFileSync(join(root, policyPath), "utf8"));
  } catch (error) {
    return [`file-description scan could not read ${policyPath}: ${error instanceof Error ? error.message : String(error)}`];
  }
  const exclusions = Array.isArray(policy.non_hand_authored_exclusions) ? policy.non_hand_authored_exclusions : [];
  const config = policy.file_descriptions;
  if (![config?.binary_extensions, config?.commentless_extensions, config?.commentless_paths].every(Array.isArray)) {
    return [`${policyPath} file_descriptions must classify binary extensions, commentless extensions, and commentless paths`];
  }
  const classifications = {
    binaryExtensions: new Set(config.binary_extensions.map((value) => String(value).toLowerCase())),
    commentlessExtensions: new Set(config.commentless_extensions.map((value) => String(value).toLowerCase())),
    commentlessPaths: new Set(config.commentless_paths.map((value) => normalizeRepoPath(String(value)))),
  };
  return [...new Set(files.map(normalizeRepoPath).filter((path) => existsSync(join(root, path)) && !excluded(path, exclusions))
    .map((path) => descriptionViolation(root, path, classifications)).filter(Boolean))].sort();
}

function listStagedFiles(root, exec) {
  const output = exec("git", ["diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z", "--"], { cwd: root, encoding: "utf8" });
  return output.split("\0").map(normalizeRepoPath).filter(Boolean).sort();
}

function run() {
  const root = resolve(fileURLToPath(new URL("../../..", import.meta.url)));
  const all = process.argv.includes("--all");
  const staged = process.argv.includes("--staged");
  const violations = collectFileDescriptionViolations(root, { all, staged });
  if (violations.length === 0) {
    console.log(`File descriptions passed for ${all ? "all maintained files" : staged ? "staged maintained files" : "changed maintained files"}.`);
    return;
  }
  const shown = violations.slice(0, 100);
  console.error(shown.join("\n"));
  if (shown.length < violations.length) console.error(`... ${violations.length - shown.length} additional violations omitted`);
  process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) run();
