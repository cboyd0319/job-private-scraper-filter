import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { delimiter, dirname } from "node:path";

export function repositoryToolchainEnvironment(root, options = {}) {
  const spawn = options.spawnSync ?? spawnSync;
  const env = options.env ?? process.env;
  const result = spawn("rustup", ["which", "rustc"], {
    cwd: root,
    env,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  const rustc = result.status === 0 ? String(result.stdout).trim() : "";
  if (!rustc || !existsSync(rustc)) return env;
  return { ...env, PATH: `${dirname(rustc)}${delimiter}${env.PATH ?? ""}` };
}

export function cargoCommand(platform = process.platform) {
  return platform === "win32" ? "cargo.exe" : "cargo";
}

export function runPinnedCargo(root, args, options = {}) {
  const spawn = options.spawnSync ?? spawnSync;
  const env = { ...(options.env ?? process.env) };
  for (const name of Object.keys(env)) {
    if (name.toUpperCase() === "JOBSENTINEL_LIVE_KEYRING_TESTS") delete env[name];
  }
  const result = spawn(cargoCommand(options.platform), args, {
    cwd: root,
    env: repositoryToolchainEnvironment(root, { ...options, env }),
    stdio: options.stdio ?? "inherit",
    encoding: options.encoding,
  });
  if (result.error) throw result.error;
  return result.status ?? 1;
}
