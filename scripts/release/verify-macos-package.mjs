#!/usr/bin/env node

import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  parseArgs,
  parseLipoArchitectures,
  hasExpectedArchitectures,
  bundleIconResourceNames,
  formatGatekeeperStatus,
  macosSmokeDataPaths,
  assertLaunchWindowVisible,
  assertSmokeDataPermissions,
  buildMacosOpenArgs,
  formatMacosOpenArgsForLog,
  createSmokeDatabaseKeyHex,
  findMacosAppProcess,
  verifyLocalDmgChecksum,
  bundleMetadataViolations,
} from "./macos-package-contract.mjs";
import {
  assertMacosRuntimeProfileArtifact,
  normalizeMacosRuntimeProfile,
  verifyMacosRuntimeProfile,
} from "../platform/macos-runtime-profile.mjs";
export {
  getArgValue,
  hasArg,
  defaultArchitectures,
  parseArgs,
  parseLipoArchitectures,
  hasExpectedArchitectures,
  bundleIconResourceNames,
  formatGatekeeperStatus,
  macosSmokeDataPaths,
  smokeDataPermissionViolations,
  buildCgWindowSmokeScript,
  parseCgWindowSmokeOutput,
  buildMacosOpenArgs,
  formatMacosOpenArgsForLog,
  createSmokeDatabaseKeyHex,
  findMacosAppProcess,
  parseSha256Checksum,
  verifyLocalDmgChecksum,
  bundleMetadataViolations,
  modelPayloadFiles,
  runtimeProfileArtifactViolations,
  runtimeProfileCommandViolations,
} from "./macos-package-contract.mjs";

import {
  assertDeveloperIdSignature,
  parseCodesignDetails,
} from "../platform/macos-signature.mjs";
import {
  createLaunchRssMetrics,
  recordLaunchRssSample,
  sampleMacosAppRss,
  stopMacosAppCoalition,
} from "./macos-package-performance.mjs";

export {
  buildMacosCoalitionKillArgs,
  parseLsappinfoCoalition,
  parsePsRssSample,
} from "./macos-package-performance.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRoot = resolve(dirname(scriptPath), "../..");
function runCapture(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: options.cwd ?? defaultRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    timeout: options.timeout,
  });
}

function runChecked(command, args, options = {}) {
  console.log(`$ ${[command, ...args].join(" ")}`);
  return runCapture(command, args, options);
}

function runResult(command, args) {
  try {
    const output = runCapture(command, args);
    return {
      accepted: true,
      output,
      status: 0,
    };
  } catch (error) {
    return {
      accepted: false,
      output: `${error.stdout ?? ""}${error.stderr ?? ""}`,
      status: error.status ?? 1,
    };
  }
}

export function buildGatekeeperAssessArgs(subject, type) {
  return ["--assess", "--type", type, "--verbose=4", subject];
}

export function buildStaplerValidateArgs(subject) {
  return ["stapler", "validate", "-v", subject];
}

function requireMacos() {
  if (process.platform !== "darwin") {
    throw new Error("macOS package verification must run on macOS.");
  }
}

function assertPathExists(path, label) {
  if (!path || !existsSync(path)) {
    throw new Error(`${label} missing: ${path ?? "(not provided)"}`);
  }
}

function readBundleString(infoPlistPath, key) {
  try {
    return runCapture("plutil", ["-extract", key, "raw", "-o", "-", infoPlistPath]).trim();
  } catch {
    return "";
  }
}

function readBundleMetadata(appPath) {
  const infoPlistPath = join(appPath, "Contents", "Info.plist");
  assertPathExists(infoPlistPath, "Info.plist");
  const iconFile = readBundleString(infoPlistPath, "CFBundleIconFile");
  const resourcesPath = join(appPath, "Contents", "Resources");

  return {
    bundleIdentifier: readBundleString(infoPlistPath, "CFBundleIdentifier"),
    bundleName: readBundleString(infoPlistPath, "CFBundleName"),
    bundleVersion: readBundleString(infoPlistPath, "CFBundleVersion"),
    displayName: readBundleString(infoPlistPath, "CFBundleDisplayName"),
    executable: readBundleString(infoPlistPath, "CFBundleExecutable"),
    iconFile,
    iconResourceExists: bundleIconResourceNames(iconFile).some((name) => existsSync(join(resourcesPath, name))),
    minimumSystemVersion: readBundleString(infoPlistPath, "LSMinimumSystemVersion"),
    shortVersion: readBundleString(infoPlistPath, "CFBundleShortVersionString"),
  };
}

function verifyBundleMetadata(appPath, expectedBundleMetadata) {
  const metadata = readBundleMetadata(appPath);
  const violations = bundleMetadataViolations(metadata, expectedBundleMetadata);
  if (violations.length > 0) {
    throw new Error(`App bundle metadata check failed:\n- ${violations.join("\n- ")}`);
  }

  console.log(
    `App bundle metadata verified: ${metadata.bundleName} ${metadata.shortVersion} (${metadata.bundleIdentifier}), icon ${metadata.iconFile}, macOS ${metadata.minimumSystemVersion}+`,
  );
  return metadata;
}

function getBundleExecutable(appPath) {
  const infoPlistPath = join(appPath, "Contents", "Info.plist");
  assertPathExists(infoPlistPath, "Info.plist");
  return runCapture("plutil", ["-extract", "CFBundleExecutable", "raw", "-o", "-", infoPlistPath]).trim();
}

function gatekeeperAssess(subject, type, requireGatekeeper) {
  const result = runResult("spctl", buildGatekeeperAssessArgs(subject, type));
  const status = formatGatekeeperStatus({
    accepted: result.accepted,
    requireGatekeeper,
    subject,
  });

  if (result.accepted) {
    console.log(status);
    return result;
  }

  if (requireGatekeeper) {
    throw new Error(`${status}\n${result.output.trim()}`);
  }

  console.log(status);
  if (result.output.trim()) {
    console.log(result.output.trim());
  }

  return result;
}

function readCodesignDetails(subject) {
  console.log(`$ codesign -dv --verbose=4 ${subject}`);
  const result = spawnSync("codesign", ["-dv", "--verbose=4", subject], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    throw new Error(`codesign details failed for ${subject}:\n${result.stderr || result.stdout}`);
  }
  return parseCodesignDetails(`${result.stdout ?? ""}${result.stderr ?? ""}`);
}

function verifyDeveloperIdSignature(subject, { requireHardenedRuntime }) {
  const details = readCodesignDetails(subject);
  assertDeveloperIdSignature(details, { requireHardenedRuntime });
  console.log(`Developer ID signature verified: ${subject}`);
}

function validateStapledTicket(subject) {
  runChecked("xcrun", buildStaplerValidateArgs(subject));
  console.log(`Stapled notarization ticket verified: ${subject}`);
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function smokeLaunch({ appPath, seconds }) {
  const executable = join(appPath, "Contents", "MacOS", getBundleExecutable(appPath));
  assertPathExists(executable, "App executable");

  const smokeRoot = mkdtempSync(join(tmpdir(), "jobsentinel-macos-smoke-"));
  const stdoutPath = join(smokeRoot, "stdout.log");
  const stderrPath = join(smokeRoot, "stderr.log");
  let deadline;
  const rssMetrics = createLaunchRssMetrics();
  let startedAt;
  let visibleWindowUpperBoundMs;
  let child;
  let pid;
  const smokeDatabaseKeyHex = createSmokeDatabaseKeyHex();

  try {
    const homePath = join(smokeRoot, "home");
    mkdirSync(homePath);
    mkdirSync(join(smokeRoot, "xdg"));
    writeFileSync(stdoutPath, "");
    writeFileSync(stderrPath, "");

    const openArgs = buildMacosOpenArgs({
      appPath,
      stderrPath,
      stdoutPath,
      smokeDatabaseKeyHex,
      smokeRoot,
    });
    console.log(`$ open ${formatMacosOpenArgsForLog(openArgs).join(" ")}`);
    startedAt = performance.now();
    deadline = startedAt + (seconds * 1000);
    child = spawn("open", openArgs, {
      stdio: ["ignore", "pipe", "pipe"],
    });

    while (performance.now() < deadline) {
      pid ??= findMacosAppProcess(appPath, getBundleExecutable(appPath));
      if (pid) {
        const sample = sampleMacosAppRss(pid);
        recordLaunchRssSample(rssMetrics, sample, performance.now(), startedAt);
        if (visibleWindowUpperBoundMs === undefined) {
          try {
            assertLaunchWindowVisible(pid);
            visibleWindowUpperBoundMs = Math.ceil(performance.now() - startedAt);
          } catch {
            // Retry until the smoke deadline so slow first-run windows get the full allowance.
          }
        }
      }
      await sleep(Math.min(100, Math.max(0, deadline - performance.now())));
    }

    if (child.exitCode !== null) {
      if (child.exitCode !== 0) {
        throw new Error(`Launch smoke open command exited with status ${child.exitCode}.`);
      }
    }

    const runningPid = findMacosAppProcess(appPath, getBundleExecutable(appPath));
    if (!runningPid) {
      const stdout = readFileSync(stdoutPath, "utf8");
      const stderr = readFileSync(stderrPath, "utf8");
      throw new Error(
        `Launch smoke did not find a running app process.\nstdout:\n${stdout}\nstderr:\n${stderr}`,
      );
    }
    pid = runningPid;
    if (visibleWindowUpperBoundMs === undefined) {
      assertLaunchWindowVisible(pid);
      visibleWindowUpperBoundMs = Math.ceil(performance.now() - startedAt);
    }
    const finalSample = sampleMacosAppRss(pid);
    recordLaunchRssSample(rssMetrics, finalSample, performance.now(), startedAt);
    if (rssMetrics.sampleCount === 0) {
      throw new Error("Launch smoke could not measure main-process RSS.");
    }
    console.log(
      `Launch performance sample: visible_window_upper_bound_ms=${visibleWindowUpperBoundMs} aggregate_observed_max_rss_kib=${rssMetrics.observedMaxRssKib} max_process_count=${rssMetrics.maxProcessCount} rss_sample_count=${rssMetrics.sampleCount} max_rss_sample_gap_ms=${rssMetrics.maxSampleGapMs}.`,
    );

    await stopMacosAppCoalition(pid);
    pid = undefined;

    const stderr = readFileSync(stderrPath, "utf8");
    if (stderr.trim()) {
      throw new Error(`Launch smoke wrote stderr:\n${stderr}`);
    }

    const dataPaths = macosSmokeDataPaths(smokeRoot);
    assertPathExists(dataPaths.dataDir, "Smoke data directory");
    assertPathExists(dataPaths.dbPath, "Smoke database");
    assertSmokeDataPermissions(dataPaths);
    console.log("Local data smoke passed: app created isolated macOS data directory and jobs.db.");
    console.log(`Launch smoke passed: app stayed running for ${seconds} seconds with empty stderr.`);
  } finally {
    if (pid) {
      await stopMacosAppCoalition(pid);
    }
    rmSync(smokeRoot, { recursive: true, force: true });
  }
}

function assertApplicationsSymlink(mountPath) {
  const applicationsPath = join(mountPath, "Applications");
  assertPathExists(applicationsPath, "Applications symlink");
  if (!lstatSync(applicationsPath).isSymbolicLink()) {
    throw new Error(`Applications entry is not a symlink: ${applicationsPath}`);
  }
}

async function verifyAppBundle({
  appPath,
  expectedBundleMetadata,
  expectedArchitectures,
  launchSmoke,
  requireGatekeeper,
  runtimeProfile,
  smokeSeconds,
  bundleLabel = "App bundle",
}) {
  assertPathExists(appPath, bundleLabel);
  verifyBundleMetadata(appPath, expectedBundleMetadata);

  const executable = join(appPath, "Contents", "MacOS", getBundleExecutable(appPath));
  assertPathExists(executable, "App executable");
  verifyMacosRuntimeProfile(appPath, executable, runtimeProfile);

  const lipoOutput = runChecked("lipo", ["-info", executable]);
  const architectures = parseLipoArchitectures(lipoOutput);
  if (!hasExpectedArchitectures(architectures, expectedArchitectures)) {
    throw new Error(
      `App binary architectures mismatch. Expected ${expectedArchitectures.join(", ")}, found ${architectures.join(", ") || "(none)"}.`,
    );
  }
  console.log(`App binary architectures verified: ${architectures.join(" ")}`);

  runChecked("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]);
  console.log(`App bundle signature verified: ${appPath}`);

  gatekeeperAssess(appPath, "execute", requireGatekeeper);
  if (requireGatekeeper) {
    verifyDeveloperIdSignature(appPath, { requireHardenedRuntime: true });
  }

  if (launchSmoke) {
    await smokeLaunch({ appPath, seconds: smokeSeconds });
  }
}

async function verifyInstalledApp({
  sourceAppPath,
  appName,
  expectedBundleMetadata,
  expectedArchitectures,
  requireGatekeeper,
  runtimeProfile,
  smokeSeconds,
}) {
  const installRoot = mkdtempSync(join(tmpdir(), "jobsentinel-macos-install-"));
  const installedAppPath = join(installRoot, appName);

  try {
    runChecked("ditto", [sourceAppPath, installedAppPath]);
    await verifyAppBundle({
      appPath: installedAppPath,
      bundleLabel: "Installed app bundle",
      expectedBundleMetadata,
      expectedArchitectures,
      launchSmoke: true,
      requireGatekeeper,
      runtimeProfile,
      smokeSeconds,
    });
    console.log(`Install smoke passed: copied app launched from ${installedAppPath}.`);
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
  }
}

export async function verifyMacosPackage(options) {
  requireMacos();
  const runtimeProfile = normalizeMacosRuntimeProfile(options.runtimeProfile);
  assertPathExists(options.dmgPath, "DMG");
  assertMacosRuntimeProfileArtifact(options.dmgPath, runtimeProfile);
  verifyLocalDmgChecksum(options.dmgPath, {
    requireChecksum: options.requireChecksum,
    verifyChecksum: options.verifyChecksum,
  });

  runChecked("hdiutil", ["verify", options.dmgPath]);
  gatekeeperAssess(options.dmgPath, "open", options.requireGatekeeper);
  if (options.requireGatekeeper) {
    verifyDeveloperIdSignature(options.dmgPath, { requireHardenedRuntime: false });
    validateStapledTicket(options.dmgPath);
  }

  const tempRoot = mkdtempSync(join(tmpdir(), "jobsentinel-macos-verify-"));
  const mountPath = join(tempRoot, "mount");
  let mounted = false;

  try {
    mkdirSync(mountPath);
    runChecked("hdiutil", [
      "attach",
      "-nobrowse",
      "-readonly",
      "-mountpoint",
      mountPath,
      options.dmgPath,
    ]);
    mounted = true;

    assertApplicationsSymlink(mountPath);
    const mountedAppPath = join(mountPath, options.appName);
    await verifyAppBundle({
      appPath: mountedAppPath,
      bundleLabel: "Mounted app bundle",
      expectedBundleMetadata: options.expectedBundleMetadata,
      expectedArchitectures: options.expectedArchitectures,
      launchSmoke: options.launchSmoke,
      requireGatekeeper: options.requireGatekeeper,
      runtimeProfile,
      smokeSeconds: options.smokeSeconds,
    });

    if (options.installSmoke) {
      await verifyInstalledApp({
        sourceAppPath: mountedAppPath,
        appName: options.appName,
        expectedBundleMetadata: options.expectedBundleMetadata,
        expectedArchitectures: options.expectedArchitectures,
        requireGatekeeper: options.requireGatekeeper,
        runtimeProfile,
        smokeSeconds: options.smokeSeconds,
      });
    }
  } finally {
    if (mounted) {
      try {
        runChecked("hdiutil", ["detach", mountPath]);
      } catch (error) {
        console.warn(error instanceof Error ? error.message : String(error));
      }
    }
    rmSync(tempRoot, { recursive: true, force: true });
  }

  console.log(`macOS package verification passed: ${options.dmgPath}`);
}

export async function main({ args = process.argv.slice(2) } = {}) {
  const options = parseArgs(args);
  if (!options.dmgPath) {
    throw new Error("Usage: verify-macos-package.mjs --dmg <path-to-dmg> [--runtime-profile essentials|stronger-local] [--expected-architectures x86_64,arm64] [--expected-bundle-id com.example.app] [--expected-product-name AppName] [--expected-version X.Y.Z] [--expected-icon-file icon.icns] [--expected-minimum-system-version 13.0] [--launch-smoke] [--install-smoke] [--require-checksum] [--require-gatekeeper]");
  }
  if (!Number.isFinite(options.smokeSeconds) || options.smokeSeconds < 1) {
    throw new Error(`Invalid --smoke-seconds value: ${options.smokeSeconds}`);
  }
  await verifyMacosPackage(options);
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    await main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
