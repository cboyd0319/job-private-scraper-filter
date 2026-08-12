/** Verifies macOS package parsing, signature, checksum, and launch-smoke helpers. */

import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { developerIdSignatureViolations, parseCodesignDetails } from "../../platform/macos-signature.mjs";
import {
  buildGatekeeperAssessArgs,
  buildMacosCoalitionKillArgs,
  bundleIconResourceNames,
  buildMacosOpenArgs,
  buildStaplerValidateArgs,
  bundleMetadataViolations,
  buildCgWindowSmokeScript,
  createSmokeDatabaseKeyHex,
  defaultArchitectures,
  formatMacosOpenArgsForLog,
  formatGatekeeperStatus,
  hasExpectedArchitectures,
  macosSmokeDataPaths,
  parseArgs,
  parseCgWindowSmokeOutput,
  parseLipoArchitectures,
  parseLsappinfoCoalition,
  parsePsRssSample,
  parseSha256Checksum,
  smokeDataPermissionViolations,
  verifyLocalDmgChecksum,
} from "../../release/verify-macos-package.mjs";

function expectedParsedArgs(dmgPath, verifyChecksum) {
  return {
    appName: "JobSentinel.app",
    dmgPath,
    expectedBundleMetadata: {
      bundleIdentifier: undefined,
      iconFile: undefined,
      minimumSystemVersion: undefined,
      productName: undefined,
      version: undefined,
    },
    expectedArchitectures: ["arm64"],
    installSmoke: false,
    launchSmoke: false,
    requireChecksum: false,
    requireGatekeeper: false,
    runtimeProfile: "essentials",
    verifyChecksum,
    smokeSeconds: 12,
  };
}

test("macOS verifier resolves bundle icon resource names", () => {
  assert.deepEqual(bundleIconResourceNames("icon.icns"), ["icon.icns"]);
  assert.deepEqual(bundleIconResourceNames("AppIcon"), ["AppIcon", "AppIcon.icns"]);
  assert.deepEqual(bundleIconResourceNames(""), []);
});

test("macOS verifier parses positional and flagged DMG arguments", () => {
  assert.deepEqual(parseArgs(["build.dmg"], "arm64"), expectedParsedArgs("build.dmg", true));

  assert.deepEqual(
    parseArgs([
      "--dmg",
      "universal.dmg",
      "--expected-architectures",
      "x86_64,arm64",
      "--expected-bundle-id",
      "com.jobsentinel.main",
      "--expected-product-name",
      "JobSentinel",
      "--expected-version",
      "2.6.4",
      "--expected-icon-file",
      "icon.icns",
      "--expected-minimum-system-version",
      "13.0",
      "--launch-smoke",
      "--smoke-seconds",
      "3",
      "--install-smoke",
      "--require-checksum",
      "--require-gatekeeper",
    ], "x64"),
    {
      appName: "JobSentinel.app",
      dmgPath: "universal.dmg",
      expectedBundleMetadata: {
        bundleIdentifier: "com.jobsentinel.main",
        iconFile: "icon.icns",
        minimumSystemVersion: "13.0",
        productName: "JobSentinel",
        version: "2.6.4",
      },
      expectedArchitectures: ["x86_64", "arm64"],
      installSmoke: true,
      launchSmoke: true,
      requireChecksum: true,
      requireGatekeeper: true,
      runtimeProfile: "essentials",
      verifyChecksum: true,
      smokeSeconds: 3,
    },
  );

  assert.deepEqual(
    parseArgs(["--dmg", "build.dmg", "--no-checksum"], "arm64"),
    expectedParsedArgs("build.dmg", false),
  );
});

test("macOS verifier maps local Node architectures to Mach-O architectures", () => {
  assert.deepEqual(defaultArchitectures("arm64"), ["arm64"]);
  assert.deepEqual(defaultArchitectures("x64"), ["x86_64"]);
});

test("macOS verifier binds the selected main process to its exact launch coalition", () => {
  const output = `
75) "JobSentinel" ASN:0x0-0x4062060:
    bundleID="com.jobsentinel.main"
    pid = 78020
    coalition: 1001  { 78020 78021 }
77) "JobSentinel" ASN:0x0-0x4064060:
    bundleID="com.jobsentinel.main"
    pid = 78038
    coalition: 1002  { 78038 78057 78058 78059 }
`;

  assert.deepEqual(parseLsappinfoCoalition(output, 78038), {
    asn: "ASN:0x0-0x4064060:",
    pids: [78038, 78057, 78058, 78059],
  });
  assert.deepEqual(parseLsappinfoCoalition("", 78038), {
    asn: undefined,
    pids: [78038],
  });
});

test("macOS verifier shuts down by launch ASN instead of historical process IDs", () => {
  assert.deepEqual(buildMacosCoalitionKillArgs("ASN:0x0-0x4064060:", false), [
    "kill",
    "-coalition",
    "ASN:0x0-0x4064060:",
  ]);
  assert.deepEqual(buildMacosCoalitionKillArgs("ASN:0x0-0x4064060:", true), [
    "kill",
    "-coalition",
    "-hard",
    "ASN:0x0-0x4064060:",
  ]);
});

test("macOS verifier sums RSS only for the selected process IDs", () => {
  assert.deepEqual(
    parsePsRssSample("78038 115072\n78057 29392\n78058 63104\n99999 900000\n", [
      78038,
      78057,
      78058,
    ]),
    {
      processCount: 3,
      rssKib: 207568,
    },
  );
});

test("macOS verifier parses lipo output for universal and thin binaries", () => {
  assert.deepEqual(
    parseLipoArchitectures("Architectures in the fat file: JobSentinel are: x86_64 arm64\n"),
    ["x86_64", "arm64"],
  );
  assert.deepEqual(
    parseLipoArchitectures("Non-fat file: JobSentinel is architecture: arm64\n"),
    ["arm64"],
  );
});

test("macOS verifier checks required architectures as a subset", () => {
  assert.equal(hasExpectedArchitectures(["x86_64", "arm64"], ["arm64"]), true);
  assert.equal(hasExpectedArchitectures(["arm64"], ["x86_64", "arm64"]), false);
});

test("macOS verifier distinguishes optional and required Gatekeeper rejection", () => {
  assert.deepEqual(buildGatekeeperAssessArgs("JobSentinel.dmg", "open"), [
    "--assess",
    "--type",
    "open",
    "--verbose=4",
    "JobSentinel.dmg",
  ]);
  assert.deepEqual(buildStaplerValidateArgs("JobSentinel.dmg"), [
    "stapler",
    "validate",
    "-v",
    "JobSentinel.dmg",
  ]);
  assert.equal(
    formatGatekeeperStatus({
      accepted: true,
      requireGatekeeper: true,
      subject: "JobSentinel.app",
    }),
    "Gatekeeper accepted: JobSentinel.app",
  );
  assert.match(
    formatGatekeeperStatus({
      accepted: false,
      requireGatekeeper: false,
      subject: "JobSentinel.app",
    }),
    /Developer ID signing and notarization/,
  );
  assert.equal(
    formatGatekeeperStatus({
      accepted: false,
      requireGatekeeper: true,
      subject: "JobSentinel.app",
    }),
    "Gatekeeper rejected required public release artifact: JobSentinel.app",
  );
});

test("macOS verifier resolves launch smoke data paths under isolated smoke root", () => {
  assert.deepEqual(macosSmokeDataPaths("/tmp/jobsentinel-macos-smoke-root"), {
    dataDir: "/tmp/jobsentinel-macos-smoke-root/home/Library/Application Support/JobSentinel",
    dbPath: "/tmp/jobsentinel-macos-smoke-root/home/Library/Application Support/JobSentinel/jobs.db",
  });
});

test("macOS verifier checks launch smoke visible window evidence", () => {
  assert.match(buildCgWindowSmokeScript(), /CGWindowListCopyWindowInfo/);
  assert.deepEqual(parseCgWindowSmokeOutput("visible-window pid=123 width=1200 height=801\n"), {
    height: 801,
    pid: 123,
    width: 1200,
  });
  assert.equal(parseCgWindowSmokeOutput("no visible app window for pid 123\n"), undefined);
});

test("macOS verifier launches app bundles fresh with isolated smoke paths", () => {
  assert.deepEqual(
    buildMacosOpenArgs({
      appPath: "/tmp/JobSentinel.app",
      stderrPath: "/tmp/stderr.log",
      stdoutPath: "/tmp/stdout.log",
      smokeDatabaseKeyHex: "58ffd25e23c63a6fcab6baffe23e9a667c4a1504ae07454573607f036017a9c4",
      smokeRoot: "/tmp/jobsentinel-macos-smoke-root",
    }),
    [
      "-F",
      "-n",
      "--env",
      "ApplePersistenceIgnoreState=YES",
      "--env",
      "JOBSENTINEL_MACOS_PACKAGE_SMOKE_ROOT=/tmp/jobsentinel-macos-smoke-root",
      "--env",
      "JOBSENTINEL_MACOS_PACKAGE_SMOKE_DATABASE_KEY_HEX=58ffd25e23c63a6fcab6baffe23e9a667c4a1504ae07454573607f036017a9c4",
      "-o",
      "/tmp/stdout.log",
      "--stderr",
      "/tmp/stderr.log",
      "/tmp/JobSentinel.app",
    ],
  );
});

test("macOS verifier creates bounded smoke database keys", () => {
  const key = createSmokeDatabaseKeyHex();
  assert.match(key, /^[a-f0-9]{64}$/);
});

test("macOS verifier redacts smoke database keys in launch logs", () => {
  const key = "58ffd25e23c63a6fcab6baffe23e9a667c4a1504ae07454573607f036017a9c4";
  const loggedArgs = formatMacosOpenArgsForLog(
    buildMacosOpenArgs({
      appPath: "/tmp/JobSentinel.app",
      stderrPath: "/tmp/stderr.log",
      stdoutPath: "/tmp/stdout.log",
      smokeDatabaseKeyHex: key,
      smokeRoot: "/tmp/jobsentinel-macos-smoke-root",
    }),
  );

  assert.equal(loggedArgs.includes(`JOBSENTINEL_MACOS_PACKAGE_SMOKE_DATABASE_KEY_HEX=${key}`), false);
  assert.equal(
    loggedArgs.includes("JOBSENTINEL_MACOS_PACKAGE_SMOKE_DATABASE_KEY_HEX=<redacted>"),
    true,
  );
});

test("macOS verifier requires private launch smoke data permissions", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-macos-permissions-"));
  const dataPaths = macosSmokeDataPaths(root);
  const backupDir = join(dataPaths.dataDir, "backups");
  const backupPath = join(backupDir, "backup_pre_migration_20260605_120000.db");
  const externalPath = join(root, "external.txt");
  const linkPath = join(dataPaths.dataDir, "linked.txt");

  try {
    mkdirSync(dataPaths.dataDir, { recursive: true, mode: 0o700 });
    mkdirSync(backupDir, { recursive: true, mode: 0o700 });
    writeFileSync(dataPaths.dbPath, "");
    writeFileSync(backupPath, "");
    writeFileSync(externalPath, "not app data");
    symlinkSync(externalPath, linkPath);
    chmodSync(dataPaths.dataDir, 0o700);
    chmodSync(backupDir, 0o700);
    chmodSync(dataPaths.dbPath, 0o600);
    chmodSync(backupPath, 0o600);
    chmodSync(externalPath, 0o644);

    assert.deepEqual(smokeDataPermissionViolations(dataPaths, { platform: "darwin" }), []);

    chmodSync(dataPaths.dataDir, 0o755);
    chmodSync(backupDir, 0o755);
    chmodSync(dataPaths.dbPath, 0o644);
    chmodSync(backupPath, 0o644);
    assert.deepEqual(smokeDataPermissionViolations(dataPaths, { platform: "darwin" }), [
      `app data directory permissions expected 700, found 755: ${dataPaths.dataDir}`,
      `app data/backups directory permissions expected 700, found 755: ${backupDir}`,
      `app data/backups/backup_pre_migration_20260605_120000.db file permissions expected 600, found 644: ${backupPath}`,
      `app data/jobs.db file permissions expected 600, found 644: ${dataPaths.dbPath}`,
    ]);
    assert.deepEqual(smokeDataPermissionViolations(dataPaths, { platform: "win32" }), []);
    assert.equal(
      smokeDataPermissionViolations(dataPaths, { platform: "darwin" }).some((violation) =>
        violation.includes(externalPath),
      ),
      false,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("macOS verifier parses SHA-256 checksum files", () => {
  assert.equal(
    parseSha256Checksum(
      "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  JobSentinel.dmg\n",
      "JobSentinel.dmg",
    ),
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  );
  assert.throws(() => parseSha256Checksum("not a checksum"), /exactly one digest line/);
  assert.throws(
    () =>
      parseSha256Checksum(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  Other.dmg\n",
        "JobSentinel.dmg",
      ),
    /filename expected JobSentinel.dmg/,
  );
  assert.throws(
    () =>
      parseSha256Checksum(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  JobSentinel.dmg\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  Other.dmg\n",
      ),
    /exactly one digest line/,
  );
});

test("macOS verifier validates local SHA-256 sidecar when present or required", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-macos-checksum-"));
  const dmgPath = join(root, "JobSentinel.dmg");

  try {
    writeFileSync(dmgPath, "test dmg bytes");
    writeFileSync(
      `${dmgPath}.sha256`,
      "3afd4f5c931bc10f99b884b7588dde1d43e6f823e1137439264f378ae1339895  JobSentinel.dmg\n",
    );

    assert.equal(verifyLocalDmgChecksum(dmgPath, { requireChecksum: true }), true);

    writeFileSync(`${dmgPath}.sha256`, "0000000000000000000000000000000000000000000000000000000000000000  JobSentinel.dmg\n");
    assert.throws(() => verifyLocalDmgChecksum(dmgPath), /SHA-256 checksum mismatch/);

    rmSync(`${dmgPath}.sha256`, { force: true });
    assert.equal(verifyLocalDmgChecksum(dmgPath), false);
    assert.throws(() => verifyLocalDmgChecksum(dmgPath, { requireChecksum: true }), /checksum sidecar missing/);
    assert.equal(verifyLocalDmgChecksum(dmgPath, { verifyChecksum: false, requireChecksum: true }), false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("macOS verifier rejects non-Developer-ID signatures in strict mode", () => {
  const developerIdDetails = parseCodesignDetails(`
Signature size=9062
Authority=Developer ID Application: Example LLC (ABCDE12345)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Timestamp=Jun 4, 2026 at 12:00:00 PM
TeamIdentifier=ABCDE12345
Runtime Version=15.0.0
`);

  assert.deepEqual(
    developerIdSignatureViolations(developerIdDetails, {
      expectedTeamId: "ABCDE12345",
      requireHardenedRuntime: true,
    }),
    [],
  );

  assert.deepEqual(
    developerIdSignatureViolations(parseCodesignDetails("Signature=adhoc\nTeamIdentifier=not set\n")),
    [
      "signature is ad-hoc or missing",
      "Developer ID Application authority is missing",
      "TeamIdentifier is missing",
      "secure timestamp is missing",
    ],
  );

  assert.deepEqual(
    developerIdSignatureViolations(
      parseCodesignDetails(`
Signature size=9000
Authority=Apple Development: Example
Timestamp=Jun 4, 2026 at 12:00:00 PM
TeamIdentifier=ABCDE12345
`),
      { expectedTeamId: "ZZZZZ99999", requireHardenedRuntime: true },
    ),
    [
      "Developer ID Application authority is missing",
      "TeamIdentifier expected ZZZZZ99999, found ABCDE12345",
      "hardened runtime is missing",
    ],
  );
});

test("macOS verifier validates required bundle metadata and expected identity", () => {
  const metadata = {
    bundleIdentifier: "com.jobsentinel.main",
    bundleName: "JobSentinel",
    bundleVersion: "2.6.4",
    displayName: "JobSentinel",
    executable: "jobsentinel",
    iconFile: "icon.icns",
    iconResourceExists: true,
    minimumSystemVersion: "13.0",
    shortVersion: "2.6.4",
  };

  assert.deepEqual(
    bundleMetadataViolations(metadata, {
      bundleIdentifier: "com.jobsentinel.main",
      iconFile: "icon.icns",
      minimumSystemVersion: "13.0",
      productName: "JobSentinel",
      version: "2.6.4",
    }),
    [],
  );

  assert.deepEqual(
    bundleMetadataViolations(
      { ...metadata, bundleIdentifier: "", displayName: "Other", shortVersion: "2.6.3" },
      {
        bundleIdentifier: "com.jobsentinel.main",
        iconFile: "icon.icns",
        minimumSystemVersion: "13.0",
        productName: "JobSentinel",
        version: "2.6.4",
      },
    ),
    [
      "CFBundleIdentifier is missing or empty",
      "CFBundleIdentifier expected com.jobsentinel.main, found (empty)",
      "CFBundleDisplayName expected JobSentinel, found Other",
      "CFBundleShortVersionString expected 2.6.4, found 2.6.3",
    ],
  );

  assert.deepEqual(
    bundleMetadataViolations(
      { ...metadata, minimumSystemVersion: "10.13" },
      { minimumSystemVersion: "13.0" },
    ),
    ["LSMinimumSystemVersion expected 13.0, found 10.13"],
  );

  assert.deepEqual(bundleMetadataViolations({ ...metadata, iconResourceExists: false }), [
    "CFBundleIconFile resource missing from Contents/Resources: icon.icns",
  ]);
});
