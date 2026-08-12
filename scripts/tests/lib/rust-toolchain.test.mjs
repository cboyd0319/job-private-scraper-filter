import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import test from "node:test";

import {
  cargoCommand,
  repositoryToolchainEnvironment,
  runPinnedCargo,
} from "../../lib/rust-toolchain.mjs";

test("repository toolchain path supports spaces without mutating the caller", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel rust root "));
  try {
    const bin = join(root, "toolchain with spaces", "bin");
    const rustc = join(bin, "rustc");
    mkdirSync(bin, { recursive: true });
    writeFileSync(rustc, "", "utf8");
    const original = { PATH: "/system/bin" };
    const env = repositoryToolchainEnvironment(root, {
      env: original,
      spawnSync: () => ({ status: 0, stdout: `${rustc}\n` }),
    });
    assert.equal(env.PATH, `${bin}${delimiter}/system/bin`);
    assert.deepEqual(original, { PATH: "/system/bin" });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("pinned cargo command preserves argument boundaries on Windows", () => {
  assert.equal(cargoCommand("win32"), "cargo.exe");
  const calls = [];
  const status = runPinnedCargo("C:\\repo with spaces", ["test", "--workspace"], {
    platform: "win32",
    env: { PATH: "C:\\system" },
    spawnSync: (command, args, options) => {
      calls.push({ command, args, cwd: options.cwd });
      return args[0] === "which" ? { status: 1, stdout: "" } : { status: 0 };
    },
  });
  assert.equal(status, 0);
  assert.deepEqual(calls.at(-1), {
    command: "cargo.exe",
    args: ["test", "--workspace"],
    cwd: "C:\\repo with spaces",
  });
});

test("pinned cargo never inherits the live Keychain test opt-in", () => {
  const original = {
    PATH: "/system/bin",
    JOBSENTINEL_LIVE_KEYRING_TESTS: "1",
    Jobsentinel_Live_Keyring_Tests: "also-on",
  };
  let cargoEnv;
  const status = runPinnedCargo("/repo", ["test"], {
    env: original,
    spawnSync: (command, args, options) => {
      if (command === "cargo") cargoEnv = options.env;
      return args[0] === "which" ? { status: 1, stdout: "" } : { status: 0 };
    },
  });

  assert.equal(status, 0);
  assert.equal(cargoEnv.JOBSENTINEL_LIVE_KEYRING_TESTS, undefined);
  assert.equal(cargoEnv.Jobsentinel_Live_Keyring_Tests, undefined);
  assert.equal(original.JOBSENTINEL_LIVE_KEYRING_TESTS, "1");
});
