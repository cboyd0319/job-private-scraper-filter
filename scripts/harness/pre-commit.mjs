#!/usr/bin/env node
// Runs the deterministic local checks required before accepting staged repository changes.

import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

export function preCommitCommands(platform = process.platform) {
  const suffix = platform === "win32" ? ".cmd" : "";
  return [
    { command: `npm${suffix}`, args: ["run", "lint:secrets"], reason: "reject staged or tracked secrets" },
    { command: `npm${suffix}`, args: ["run", "lint:file-description", "--", "--staged"], reason: "require staged file responsibility descriptions" },
    { command: `npx${suffix}`, args: ["--no-install", "lint-staged"], reason: "check only staged files with installed tools" },
  ];
}

export function runPreCommit(options = {}) {
  const run = options.spawnSync ?? spawnSync;
  const cwd = options.cwd ?? process.cwd();
  for (const step of preCommitCommands(options.platform)) {
    const result = run(step.command, step.args, {
      cwd,
      env: options.env ?? process.env,
      shell: false,
      stdio: options.stdio ?? "inherit",
    });
    if (result.error) {
      console.error(`Pre-commit could not run ${step.command}: ${result.error.message}`);
      console.error("Install locked project dependencies, or use --no-verify only for recovery and run the applicable local gate before relying on the commit.");
      return 127;
    }
    if (result.status !== 0) {
      const status = Number.isInteger(result.status) ? result.status : 1;
      console.error(`Pre-commit failed (${status}): ${step.reason}.`);
      console.error("Correct the reported issue, or use --no-verify only for recovery and run the applicable local gate before relying on the commit.");
      return status;
    }
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = runPreCommit();
}
