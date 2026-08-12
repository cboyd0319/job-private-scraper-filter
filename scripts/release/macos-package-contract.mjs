import { spawnSync } from "node:child_process";
import { createHash, randomBytes } from "node:crypto";
import { existsSync, lstatSync, readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { getMacosRuntimeProfile } from "../platform/macos-runtime-profile.mjs";
import { parseSha256Checksum } from "./checksum.mjs";

export { parseSha256Checksum } from "./checksum.mjs";
export {
  modelPayloadFiles,
  runtimeProfileArtifactViolations,
  runtimeProfileCommandViolations,
} from "../platform/macos-runtime-profile.mjs";

const defaultSmokeSeconds = 12;
const smokeDatabaseKeyHexEnv = "JOBSENTINEL_MACOS_PACKAGE_SMOKE_DATABASE_KEY_HEX";
const smokeRootEnv = "JOBSENTINEL_MACOS_PACKAGE_SMOKE_ROOT";

export function getArgValue(args, name) {
  const exactIndex = args.indexOf(name);
  if (exactIndex >= 0) {
    return args[exactIndex + 1];
  }

  const prefixed = args.find((arg) => arg.startsWith(`${name}=`));
  return prefixed ? prefixed.slice(name.length + 1) : undefined;
}

export function hasArg(args, name) {
  return args.some((arg) => arg === name || arg.startsWith(`${name}=`));
}

function splitList(value) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function defaultArchitectures(arch = process.arch) {
  if (arch === "arm64") return ["arm64"];
  if (arch === "x64") return ["x86_64"];
  return [arch];
}

export function parseArgs(args, arch = process.arch) {
  const positionalDmg = args.find((arg) => !arg.startsWith("-"));
  const dmgPath = getArgValue(args, "--dmg") ?? positionalDmg;
  const expectedArchitectures =
    getArgValue(args, "--expected-architectures") ??
    getArgValue(args, "--expected-archs") ??
    "";
  const smokeValue = getArgValue(args, "--smoke-seconds");

  return {
    appName: getArgValue(args, "--app-name") ?? "JobSentinel.app",
    dmgPath,
    expectedBundleMetadata: {
      bundleIdentifier: getArgValue(args, "--expected-bundle-id"),
      iconFile: getArgValue(args, "--expected-icon-file"),
      minimumSystemVersion: getArgValue(args, "--expected-minimum-system-version"),
      productName: getArgValue(args, "--expected-product-name"),
      version: getArgValue(args, "--expected-version"),
    },
    expectedArchitectures: expectedArchitectures
      ? splitList(expectedArchitectures)
      : defaultArchitectures(arch),
    installSmoke: hasArg(args, "--install-smoke"),
    launchSmoke: hasArg(args, "--launch-smoke") || Boolean(smokeValue),
    requireChecksum: hasArg(args, "--require-checksum"),
    requireGatekeeper: hasArg(args, "--require-gatekeeper"),
    runtimeProfile: getMacosRuntimeProfile(args),
    verifyChecksum: !hasArg(args, "--no-checksum"),
    smokeSeconds: smokeValue ? Number(smokeValue) : defaultSmokeSeconds,
  };
}

export function parseLipoArchitectures(output) {
  const fatMatch = output.match(/are:\s*([^\n]+)/);
  if (fatMatch) {
    return fatMatch[1].split(/\s+/).filter(Boolean);
  }

  const thinMatch = output.match(/architecture:\s*([^\s]+)/);
  if (thinMatch) {
    return [thinMatch[1]];
  }

  return [];
}

export function hasExpectedArchitectures(found, expected) {
  const foundSet = new Set(found);
  return expected.every((architecture) => foundSet.has(architecture));
}

export function formatGatekeeperStatus({ subject, accepted, requireGatekeeper }) {
  if (accepted) {
    return `Gatekeeper accepted: ${subject}`;
  }

  if (requireGatekeeper) {
    return `Gatekeeper rejected required public release artifact: ${subject}`;
  }

  return `Gatekeeper rejected optional check: ${subject}. Developer ID signing and notarization are still required for a zero-friction public macOS release.`;
}

export function macosSmokeDataPaths(smokeRoot) {
  const dataDir = join(smokeRoot, "home", "Library", "Application Support", "JobSentinel");
  return {
    dataDir,
    dbPath: join(dataDir, "jobs.db"),
  };
}

function modeBits(path) {
  return lstatSync(path).mode & 0o777;
}

function smokeDataPermissionTargets(dataDir) {
  const targets = [];

  function visit(path, label) {
    const info = lstatSync(path);
    if (info.isSymbolicLink()) {
      return;
    }

    if (info.isDirectory()) {
      targets.push([`${label} directory`, path, 0o700]);
      for (const child of readdirSync(path).sort()) {
        visit(join(path, child), `${label}/${child}`);
      }
      return;
    }

    if (info.isFile()) {
      targets.push([`${label} file`, path, 0o600]);
    }
  }

  visit(dataDir, "app data");
  return targets;
}

export function smokeDataPermissionViolations(dataPaths, { platform = process.platform } = {}) {
  if (platform !== "darwin" && platform !== "linux") {
    return [];
  }

  const violations = [];

  for (const [label, path, expectedMode] of smokeDataPermissionTargets(dataPaths.dataDir)) {
    const actualMode = modeBits(path);
    if (actualMode !== expectedMode) {
      violations.push(
        `${label} permissions expected ${expectedMode.toString(8)}, found ${actualMode.toString(8)}: ${path}`,
      );
    }
  }

  return violations;
}

export function buildCgWindowSmokeScript() {
  return `
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2, let expectedPid = Int(CommandLine.arguments[1]) else {
  fputs("missing pid\\n", stderr)
  exit(64)
}

let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
let windows = (CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]]) ?? []

for window in windows {
  let pid = window[kCGWindowOwnerPID as String] as? Int ?? -1
  guard pid == expectedPid else {
    continue
  }

  let layer = window[kCGWindowLayer as String] as? Int ?? -1
  let isOnscreen = window[kCGWindowIsOnscreen as String] as? Int ?? 0
  let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
  let width = bounds["Width"] as? Double ?? 0
  let height = bounds["Height"] as? Double ?? 0

  if layer == 0 && isOnscreen == 1 && width >= 320 && height >= 240 {
    print("visible-window pid=\\(pid) width=\\(Int(width)) height=\\(Int(height))")
    exit(0)
  }
}

fputs("no visible app window for pid \\(expectedPid)\\n", stderr)
exit(2)
`;
}

export function parseCgWindowSmokeOutput(output) {
  const match = output.trim().match(/^visible-window pid=(\d+) width=(\d+) height=(\d+)$/);
  if (!match) {
    return undefined;
  }

  return {
    height: Number.parseInt(match[3], 10),
    pid: Number.parseInt(match[1], 10),
    width: Number.parseInt(match[2], 10),
  };
}

export function assertLaunchWindowVisible(pid) {
  if (process.platform !== "darwin") {
    return;
  }

  const result = spawnSync("swift", ["-", String(pid)], {
    encoding: "utf8",
    input: buildCgWindowSmokeScript(),
    stdio: ["pipe", "pipe", "pipe"],
  });

  if (result.status !== 0) {
    throw new Error(
      `Launch smoke did not expose a visible macOS app window for pid ${pid}:\n${result.stderr || result.stdout}`,
    );
  }

  const windowInfo = parseCgWindowSmokeOutput(result.stdout);
  if (!windowInfo) {
    throw new Error(`Launch smoke returned unexpected window evidence:\n${result.stdout}`);
  }

  console.log(`Visible window smoke passed: ${windowInfo.width}x${windowInfo.height} on-screen app window.`);
}

export function assertSmokeDataPermissions(dataPaths) {
  const violations = smokeDataPermissionViolations(dataPaths);
  if (violations.length > 0) {
    throw new Error(`Local data smoke permissions failed:\n- ${violations.join("\n- ")}`);
  }

  console.log("Local data permissions smoke passed: app data tree is private and jobs.db is owner-only.");
}

export function buildMacosOpenArgs({
  appPath,
  smokeDatabaseKeyHex,
  smokeRoot,
  stdoutPath,
  stderrPath,
}) {
  return [
    "-F",
    "-n",
    "--env",
    "ApplePersistenceIgnoreState=YES",
    "--env",
    `${smokeRootEnv}=${smokeRoot}`,
    "--env",
    `${smokeDatabaseKeyHexEnv}=${smokeDatabaseKeyHex}`,
    "-o",
    stdoutPath,
    "--stderr",
    stderrPath,
    appPath,
  ];
}

export function formatMacosOpenArgsForLog(args) {
  const smokeDatabaseKeyPrefix = `${smokeDatabaseKeyHexEnv}=`;
  return args.map((arg) =>
    arg.startsWith(smokeDatabaseKeyPrefix) ? `${smokeDatabaseKeyPrefix}<redacted>` : arg,
  );
}

export function createSmokeDatabaseKeyHex() {
  return randomBytes(32).toString("hex");
}

export function findMacosAppProcess(appPath, executableName) {
  const executablePath = join(appPath, "Contents", "MacOS", executableName);
  const result = spawnSync("ps", ["-axo", "pid,args"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.status !== 0) {
    return undefined;
  }

  const line = result.stdout
    .split("\n")
    .find((candidate) => candidate.includes(executablePath));
  if (!line) {
    return undefined;
  }

  const pid = Number.parseInt(line.trim().split(/\s+/)[0], 10);
  return Number.isFinite(pid) ? pid : undefined;
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function verifyLocalDmgChecksum(dmgPath, { requireChecksum = false, verifyChecksum = true } = {}) {
  if (!verifyChecksum) {
    return false;
  }

  const checksumPath = `${dmgPath}.sha256`;
  if (!existsSync(checksumPath)) {
    if (requireChecksum) {
      throw new Error(`SHA-256 checksum sidecar missing: ${checksumPath}`);
    }
    console.warn(`No SHA-256 checksum sidecar found; skipping checksum check: ${checksumPath}`);
    return false;
  }

  const expected = parseSha256Checksum(readFileSync(checksumPath, "utf8"), basename(dmgPath));
  const actual = sha256File(dmgPath);
  if (actual !== expected) {
    throw new Error(`SHA-256 checksum mismatch for ${dmgPath}. Expected ${expected}, found ${actual}.`);
  }

  console.log(`SHA-256 checksum verified: ${actual}`);
  return true;
}

const requiredBundleMetadata = [
  ["bundleIdentifier", "CFBundleIdentifier"],
  ["bundleName", "CFBundleName"],
  ["displayName", "CFBundleDisplayName"],
  ["executable", "CFBundleExecutable"],
  ["iconFile", "CFBundleIconFile"],
  ["minimumSystemVersion", "LSMinimumSystemVersion"],
  ["shortVersion", "CFBundleShortVersionString"],
  ["bundleVersion", "CFBundleVersion"],
];

export function bundleIconResourceNames(iconFile) {
  if (!iconFile) {
    return [];
  }

  return iconFile.endsWith(".icns") ? [iconFile] : [iconFile, `${iconFile}.icns`];
}

export function bundleMetadataViolations(metadata, expected = {}) {
  const violations = [];

  for (const [field, plistKey] of requiredBundleMetadata) {
    if (!metadata[field]) {
      violations.push(`${plistKey} is missing or empty`);
    }
  }

  if (expected.bundleIdentifier && metadata.bundleIdentifier !== expected.bundleIdentifier) {
    violations.push(
      `CFBundleIdentifier expected ${expected.bundleIdentifier}, found ${metadata.bundleIdentifier || "(empty)"}`,
    );
  }

  if (expected.productName) {
    if (metadata.bundleName !== expected.productName) {
      violations.push(
        `CFBundleName expected ${expected.productName}, found ${metadata.bundleName || "(empty)"}`,
      );
    }
    if (metadata.displayName !== expected.productName) {
      violations.push(
        `CFBundleDisplayName expected ${expected.productName}, found ${metadata.displayName || "(empty)"}`,
      );
    }
  }

  if (expected.version) {
    if (metadata.shortVersion !== expected.version) {
      violations.push(
        `CFBundleShortVersionString expected ${expected.version}, found ${metadata.shortVersion || "(empty)"}`,
      );
    }
    if (metadata.bundleVersion !== expected.version) {
      violations.push(
        `CFBundleVersion expected ${expected.version}, found ${metadata.bundleVersion || "(empty)"}`,
      );
    }
  }

  if (expected.iconFile && metadata.iconFile !== expected.iconFile) {
    violations.push(`CFBundleIconFile expected ${expected.iconFile}, found ${metadata.iconFile || "(empty)"}`);
  }

  if (expected.minimumSystemVersion && metadata.minimumSystemVersion !== expected.minimumSystemVersion) {
    violations.push(
      `LSMinimumSystemVersion expected ${expected.minimumSystemVersion}, found ${metadata.minimumSystemVersion || "(empty)"}`,
    );
  }

  if (metadata.iconFile && metadata.iconResourceExists === false) {
    violations.push(`CFBundleIconFile resource missing from Contents/Resources: ${metadata.iconFile}`);
  }

  return violations;
}
