#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  buildMacosTauriEnv,
  getSigningIdentity,
  buildDmgCodesignArgs,
  ensureSignedApp,
  readCodesignDetails,
  buildNotarytoolSubmitArgs,
  hasPartialNotarizationCredentials,
  shouldNotarizeDmg,
  redactNotarytoolArgs,
  parseNotarytoolSubmitResult,
  buildNotarytoolLogArgs,
} from "./macos-signing.mjs";
import {
  assertDeveloperIdSignature,
  extractTeamIdFromSigningIdentity,
} from "./macos-signature.mjs";
import {
  getMacosRuntimeProfile,
  macosRuntimeProfileTauriArgs,
} from "./macos-runtime-profile.mjs";

export {
  prependPathDir,
  stripMacosTauriBuildSecrets,
  getRustupToolchainBinDir,
  buildMacosTauriEnv,
  getSigningIdentity,
  buildAppCodesignArgs,
  buildDmgCodesignArgs,
  buildNotarytoolSubmitArgs,
  hasPartialNotarizationCredentials,
  shouldNotarizeDmg,
  redactNotarytoolArgs,
  parseNotarytoolSubmitResult,
  buildNotarytoolLogArgs,
} from "./macos-signing.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRoot = resolve(dirname(scriptPath), "../..");

export function hasArg(args, name) {
  return args.some((arg) => arg === name || arg.startsWith(`${name}=`));
}

export function getArgValue(args, name) {
  const exactIndex = args.indexOf(name);
  if (exactIndex >= 0) {
    return args[exactIndex + 1];
  }

  const prefixed = args.find((arg) => arg.startsWith(`${name}=`));
  return prefixed ? prefixed.slice(name.length + 1) : undefined;
}

export { getMacosRuntimeProfile as getRuntimeProfile } from "./macos-runtime-profile.mjs";

export function buildTauriArgs(args) {
  const tauriArgs = ["build", ...macosRuntimeProfileTauriArgs(args)];
  const separatorIndex = tauriArgs.indexOf("--");
  const tauriArgsEnd = separatorIndex === -1 ? tauriArgs.length : separatorIndex;

  if (!hasArg(tauriArgs.slice(0, tauriArgsEnd), "--bundles")) {
    tauriArgs.splice(tauriArgsEnd, 0, "--bundles", "app");
  }

  return tauriArgs;
}

export function getReleaseDir(root, args) {
  const target = getArgValue(args, "--target");

  if (target) {
    return join(root, "target", target, "release");
  }

  return join(root, "target", "release");
}

export function getArchSuffix(target, arch = process.arch) {
  if (target === "universal-apple-darwin") {
    return "universal";
  }

  if (target?.includes("aarch64")) {
    return "aarch64";
  }

  if (target?.includes("x86_64")) {
    return "x64";
  }

  if (arch === "arm64") {
    return "aarch64";
  }

  if (arch === "x64") {
    return "x64";
  }

  return arch;
}

export function readBuildMetadata(root = defaultRoot) {
  const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const tauriConfig = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));

  return {
    productName: tauriConfig.productName ?? packageJson.name,
    version: tauriConfig.version ?? packageJson.version,
  };
}

function run(command, args, options = {}) {
  const logArgs = options.logArgs ?? args;
  console.log(`$ ${[command, ...logArgs].join(" ")}`);
  execFileSync(command, args, {
    cwd: options.cwd ?? defaultRoot,
    env: options.env ?? process.env,
    stdio: options.stdio ?? "inherit",
    encoding: "utf8",
    timeout: options.timeout,
  });
}

function sleepSync(ms) {
  if (ms <= 0) {
    return;
  }
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function runCapture(command, args, options = {}) {
  const logArgs = options.logArgs ?? args;
  console.log(`$ ${[command, ...logArgs].join(" ")}`);
  return execFileSync(command, args, {
    cwd: options.cwd ?? defaultRoot,
    env: options.env ?? process.env,
    stdio: ["ignore", "pipe", "pipe"],
    encoding: "utf8",
    timeout: options.timeout,
  });
}

export function isTransientHdiutilVerifyError(error) {
  const detail = [
    error instanceof Error ? error.message : String(error),
    error?.stdout,
    error?.stderr,
  ]
    .filter(Boolean)
    .join("\n");

  return /resource temporarily unavailable|temporarily unavailable|resource busy/i.test(detail);
}

export function verifyDmgWithRetry(
  dmgPath,
  {
    delaysMs = [1000, 2000, 5000, 10000],
    runCommand = run,
    sleep = sleepSync,
  } = {},
) {
  const maxAttempts = delaysMs.length + 1;

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      runCommand("hdiutil", ["verify", dmgPath]);
      return;
    } catch (error) {
      const canRetry = attempt < maxAttempts && isTransientHdiutilVerifyError(error);
      if (!canRetry) {
        throw error;
      }

      const delayMs = delaysMs[attempt - 1];
      console.warn(
        `hdiutil verify busy for ${dmgPath}; retrying in ${delayMs}ms ` +
          `(${attempt}/${maxAttempts})`,
      );
      sleep(delayMs);
    }
  }
}

export function staleDmgArtifactNames(dmgName) {
  const names = new Set([dmgName, `${dmgName}.sha256`]);

  const noAccountName = noAccountDmgArtifactName(dmgName);
  if (noAccountName) {
    names.add(noAccountName);
    names.add(`${noAccountName}.sha256`);
  }

  const unlabeledName = unlabeledDmgArtifactName(dmgName);
  if (unlabeledName) {
    names.add(unlabeledName);
    names.add(`${unlabeledName}.sha256`);
  }

  return names;
}

export function noAccountDmgArtifactName(dmgName) {
  if (dmgName.includes("_no-account_")) {
    return null;
  }

  return dmgName.endsWith("_universal.dmg")
    ? dmgName.replace("_universal.dmg", "_no-account_universal.dmg")
    : `${dmgName.replace(/\.dmg$/, "")}_no-account_macos.dmg`;
}

export function unlabeledDmgArtifactName(dmgName) {
  if (dmgName.endsWith("_no-account_universal.dmg")) {
    return dmgName.replace("_no-account_universal.dmg", "_universal.dmg");
  }

  if (dmgName.endsWith("_no-account_macos.dmg")) {
    return dmgName.replace("_no-account_macos.dmg", ".dmg");
  }

  return null;
}

export function removeStaleDmgArtifacts(paths, dmgName) {
  const staleNames = staleDmgArtifactNames(dmgName);

  for (const path of paths) {
    if (!existsSync(path)) {
      continue;
    }

    for (const entry of readdirSync(path)) {
      if (staleNames.has(entry) || (entry.startsWith("rw.") && entry.endsWith(".dmg"))) {
        rmSync(join(path, entry), { force: true });
      }
    }
  }
}

function signDmgForDistribution(dmgPath, identity) {
  if (identity === "-") {
    return false;
  }

  run("codesign", buildDmgCodesignArgs(identity, dmgPath));
  run("codesign", ["--verify", "--verbose=2", dmgPath]);
  assertDeveloperIdSignature(readCodesignDetails(dmgPath), {
    expectedTeamId: extractTeamIdFromSigningIdentity(identity),
  });
  return true;
}

function notarizeAndStapleDmg(dmgPath) {
  const submitArgs = buildNotarytoolSubmitArgs(dmgPath);
  const shouldNotarize = shouldNotarizeDmg(process.env, dmgPath);

  if (!shouldNotarize) {
    return;
  }

  if (hasPartialNotarizationCredentials()) {
    throw new Error(
      "Incomplete macOS notarization credentials. Use APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID; APPLE_API_KEY and APPLE_API_KEY_PATH; or JOBSENTINEL_MACOS_NOTARY_PROFILE.",
    );
  }

  if (!submitArgs) {
    throw new Error("macOS DMG notarization was requested but notarization credentials are missing.");
  }

  const identity = getSigningIdentity();
  if (identity === "-") {
    throw new Error("macOS DMG notarization requires APPLE_SIGNING_IDENTITY or JOBSENTINEL_MACOS_SIGNING_IDENTITY.");
  }

  signDmgForDistribution(dmgPath, identity);
  let submitOutput;
  try {
    submitOutput = runCapture("xcrun", submitArgs, {
      logArgs: redactNotarytoolArgs(submitArgs),
    });
  } catch (error) {
    const output = `${error.stdout ?? ""}${error.stderr ?? ""}`;
    const submissionId = output.match(/"id"\s*:\s*"([^"]+)"/)?.[1];
    const logArgs = submissionId ? buildNotarytoolLogArgs(submissionId) : null;
    if (logArgs) {
      run("xcrun", logArgs, { logArgs: redactNotarytoolArgs(logArgs) });
    }
    throw error;
  }
  const result = parseNotarytoolSubmitResult(submitOutput);
  if (result.id) {
    console.log(`macOS notarization accepted: ${result.id}`);
  }
  run("xcrun", ["stapler", "staple", dmgPath]);
  run("xcrun", ["stapler", "validate", dmgPath]);
}

function createDmg({ appPath, dmgPath, productName }) {
  const staging = mkdtempSync(join(tmpdir(), "jobsentinel-dmg-"));

  try {
    cpSync(appPath, join(staging, basename(appPath)), { recursive: true });
    symlinkSync("/Applications", join(staging, "Applications"));

    mkdirSync(dirname(dmgPath), { recursive: true });
    run("hdiutil", [
      "create",
      "-volname",
      productName,
      "-srcfolder",
      staging,
      "-ov",
      "-format",
      "UDZO",
      "-imagekey",
      "zlib-level=9",
      dmgPath,
    ]);
    verifyDmgWithRetry(dmgPath);
    notarizeAndStapleDmg(dmgPath);
    writeDmgChecksum(dmgPath);
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

export function formatDmgChecksum(dmgPath, digest) {
  return `${digest}  ${basename(dmgPath)}\n`;
}

function writeDmgChecksum(dmgPath) {
  const digest = createHash("sha256").update(readFileSync(dmgPath)).digest("hex");
  const checksumPath = `${dmgPath}.sha256`;
  writeFileSync(checksumPath, formatDmgChecksum(dmgPath, digest));
  console.log(`macOS DMG SHA-256: ${checksumPath}`);
}

export function getMacBuildPaths(root, args, metadata = readBuildMetadata(root), env = process.env) {
  const target = getArgValue(args, "--target");
  const runtimeProfile = getMacosRuntimeProfile(args);
  const releaseDir = getReleaseDir(root, args);
  const appPath = join(releaseDir, "bundle", "macos", `${metadata.productName}.app`);
  const profileLabel = runtimeProfile === "stronger-local" ? "_stronger-local" : "";
  const baseDmgName = `${metadata.productName}_${metadata.version}${profileLabel}_${getArchSuffix(target)}.dmg`;
  const dmgName = env.JOBSENTINEL_MACOS_NO_ACCOUNT === "true"
    ? noAccountDmgArtifactName(baseDmgName) ?? baseDmgName
    : baseDmgName;
  const dmgDir = join(releaseDir, "bundle", "dmg");
  const dmgPath = join(dmgDir, dmgName);

  return {
    appPath,
    dmgDir,
    dmgName,
    dmgPath,
    macosDir: join(releaseDir, "bundle", "macos"),
    releaseDir,
    runtimeProfile,
    target,
  };
}

export function main({ root = defaultRoot, args = process.argv.slice(2) } = {}) {
  if (process.platform !== "darwin") {
    throw new Error("macOS DMG build must run on macOS.");
  }

  const metadata = readBuildMetadata(root);
  const paths = getMacBuildPaths(root, args, metadata);
  const tauriBin = join(root, "node_modules", ".bin", "tauri");

  console.log(`macOS runtime profile: ${paths.runtimeProfile}`);
  removeStaleDmgArtifacts([paths.dmgDir, paths.macosDir], paths.dmgName);
  run(tauriBin, buildTauriArgs(args), { cwd: root, env: buildMacosTauriEnv() });

  if (!existsSync(paths.appPath)) {
    throw new Error(`Tauri app bundle missing: ${paths.appPath}`);
  }

  ensureSignedApp(paths.appPath);
  createDmg({
    appPath: paths.appPath,
    dmgPath: paths.dmgPath,
    productName: metadata.productName,
  });

  console.log(`macOS app: ${paths.appPath}`);
  console.log(`macOS DMG: ${paths.dmgPath}`);
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
