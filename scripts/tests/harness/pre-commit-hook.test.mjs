// Verifies the deterministic, local-only command sequence used by the pre-commit hook.
import assert from "node:assert/strict";
import test from "node:test";

import { preCommitCommands, runPreCommit } from "../../harness/pre-commit.mjs";

test("pre-commit uses installed tools without a shell or network discovery", () => {
  assert.deepEqual(preCommitCommands("darwin"), [
    { command: "npm", args: ["run", "lint:secrets"], reason: "reject staged or tracked secrets" },
    { command: "npm", args: ["run", "lint:file-description", "--", "--staged"], reason: "require staged file responsibility descriptions" },
    { command: "npx", args: ["--no-install", "lint-staged"], reason: "check only staged files with installed tools" },
  ]);
  assert.equal(preCommitCommands("win32")[0].command, "npm.cmd");
});

test("pre-commit passing fixture runs all deterministic steps", () => {
  const calls = [];
  const status = runPreCommit({
    stdio: "ignore",
    spawnSync(command, args, options) {
      calls.push({ command, args, shell: options.shell });
      return { status: 0 };
    },
  });
  assert.equal(status, 0);
  assert.equal(calls.length, 3);
  assert.ok(calls.every((call) => call.shell === false));
});

test("pre-commit failing fixture preserves status and stops immediately", () => {
  let calls = 0;
  const status = runPreCommit({
    stdio: "ignore",
    spawnSync() {
      calls += 1;
      return { status: 19 };
    },
  });
  assert.equal(status, 19);
  assert.equal(calls, 1);
});
