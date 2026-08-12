import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  buildTauriArgs,
  getMacBuildPaths,
  getRuntimeProfile,
  staleDmgArtifactNames,
} from "../../platform/build-macos-dmg.mjs";
import { verifyMacosRuntimeProfile } from "../../platform/macos-runtime-profile.mjs";
import {
  modelPayloadFiles,
  parseArgs,
  runtimeProfileArtifactViolations,
  runtimeProfileCommandViolations,
} from "../../release/verify-macos-package.mjs";

test("macOS DMG builder exposes an explicit stronger-local embedded-ML path", () => {
  const packageJson = JSON.parse(readFileSync(new URL("../../../package.json", import.meta.url), "utf8"));

  assert.equal(
    packageJson.scripts["tauri:build:macos:stronger-local"],
    "node scripts/platform/build-macos-dmg.mjs --runtime-profile stronger-local",
  );
  assert.equal(
    packageJson.scripts["tauri:verify:macos:stronger-local"],
    "node scripts/release/verify-macos-package.mjs --runtime-profile stronger-local",
  );
  assert.equal(getRuntimeProfile([]), "essentials");
  assert.equal(getRuntimeProfile(["--runtime-profile", "stronger-local"]), "stronger-local");
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--target", "aarch64-apple-darwin"]),
    ["build", "--target", "aarch64-apple-darwin", "--features", "embedded-ml", "--bundles", "app"],
  );
  assert.deepEqual(buildTauriArgs([]), ["build", "--bundles", "app"]);
});

test("macOS DMG builder labels stronger-local artifacts for stale cleanup isolation", () => {
  const paths = getMacBuildPaths("/repo", ["--runtime-profile=stronger-local"], {
    productName: "JobSentinel",
    version: "2.6.4",
  }, {});

  assert.equal(paths.runtimeProfile, "stronger-local");
  assert.equal(paths.dmgName, "JobSentinel_2.6.4_stronger-local_aarch64.dmg");
  assert.deepEqual(
    Array.from(staleDmgArtifactNames(paths.dmgName)).sort(),
    [
      "JobSentinel_2.6.4_stronger-local_aarch64.dmg",
      "JobSentinel_2.6.4_stronger-local_aarch64.dmg.sha256",
      "JobSentinel_2.6.4_stronger-local_aarch64_no-account_macos.dmg",
      "JobSentinel_2.6.4_stronger-local_aarch64_no-account_macos.dmg.sha256",
    ],
  );
});

test("macOS verifier binds the stronger-local profile to its labeled artifact and native commands", () => {
  assert.equal(
    parseArgs(["--dmg", "JobSentinel_2.6.4_stronger-local_aarch64.dmg", "--runtime-profile", "stronger-local"]).runtimeProfile,
    "stronger-local",
  );
  assert.deepEqual(
    runtimeProfileArtifactViolations("JobSentinel_2.6.4_stronger-local_aarch64.dmg", "stronger-local"),
    [],
  );
  assert.deepEqual(
    runtimeProfileArtifactViolations("JobSentinel_2.6.4_aarch64.dmg", "stronger-local"),
    ["stronger-local artifact name must contain _stronger-local_"],
  );
  assert.deepEqual(
    runtimeProfileCommandViolations(
      [
        "download_ml_model",
        "cancel_ml_model_download",
        "remove_ml_models",
        "get_ml_status",
        "semantic_match_skills",
        "match_resume_semantic",
      ].join("\n"),
      "stronger-local",
    ),
    [],
  );
  assert.deepEqual(
    runtimeProfileCommandViolations("download_ml_model\nsemantic_match_skills", "stronger-local"),
    [
      "stronger-local native command missing: cancel_ml_model_download",
      "stronger-local native command missing: remove_ml_models",
      "stronger-local native command missing: get_ml_status",
      "stronger-local native command missing: match_resume_semantic",
    ],
  );
  assert.deepEqual(runtimeProfileCommandViolations("download_ml_model", "essentials"), [
    "Essentials binary unexpectedly contains stronger-local native command: download_ml_model",
  ]);
});

test("macOS package verifier defaults imported callers without a runtime profile to Essentials", () => {
  assert.deepEqual(runtimeProfileCommandViolations("", undefined), []);
});

test("macOS Essentials builder rejects embedded-ML feature forwarding", () => {
  assert.throws(
    () => buildTauriArgs(["--features", "embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--features", "ocr", "embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["-f", "embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--features", "ocr embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["-fembedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["-vfembedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--features", "jobsentinel/embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["-fjobsentinel/embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--features", "jobsentinel-local-ai/embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--all-features"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--", "--all-features"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--", "-F", "embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--", "-Fembedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  assert.throws(
    () => buildTauriArgs(["--", "-F=embedded-ml"]),
    /Essentials macOS builds cannot enable embedded-ml/,
  );
  for (const groupedFlag of ["-qFembedded-ml", "-rFembedded-ml", "-vFembedded-ml"]) {
    assert.throws(
      () => buildTauriArgs(["--", groupedFlag]),
      /Essentials macOS builds cannot enable embedded-ml/,
    );
  }
  assert.doesNotThrow(() => buildTauriArgs(["-vembedded-ml"]));
});

test("macOS stronger-local builder preserves existing activation without duplication", () => {
  assert.deepEqual(
    buildTauriArgs([
      "--runtime-profile",
      "stronger-local",
      "--features",
      "jobsentinel/embedded-ml",
    ]),
    ["build", "--features", "jobsentinel/embedded-ml", "--bundles", "app"],
  );
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--all-features"]),
    ["build", "--all-features", "--bundles", "app"],
  );
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--", "-F", "embedded-ml"]),
    ["build", "--bundles", "app", "--", "-F", "embedded-ml"],
  );
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--", "-Fjobsentinel/embedded-ml"]),
    ["build", "--bundles", "app", "--", "-Fjobsentinel/embedded-ml"],
  );
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--", "-F=embedded-ml"]),
    ["build", "--bundles", "app", "--", "-F=embedded-ml"],
  );
  assert.deepEqual(
    buildTauriArgs([
      "--runtime-profile",
      "stronger-local",
      "--features",
      "jobsentinel-local-ai/embedded-ml",
    ]),
    [
      "build",
      "--features",
      "jobsentinel-local-ai/embedded-ml",
      "--features",
      "embedded-ml",
      "--bundles",
      "app",
    ],
  );
});

test("macOS builder inserts its default bundle before forwarded Cargo arguments", () => {
  assert.deepEqual(
    buildTauriArgs(["--runtime-profile", "stronger-local", "--", "--all-features"]),
    ["build", "--bundles", "app", "--", "--all-features"],
  );
  assert.deepEqual(
    buildTauriArgs(["--", "--bundles", "dmg"]),
    ["build", "--bundles", "app", "--", "--bundles", "dmg"],
  );
});

test("macOS runtime verifier handles release-sized executable strings", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-macos-large-binary-"));
  const executable = join(root, "jobsentinel");

  try {
    writeFileSync(executable, "a".repeat(2 * 1024 * 1024));
    assert.doesNotThrow(() => verifyMacosRuntimeProfile(root, executable, "essentials"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("macOS package verifier rejects case-insensitive model payload suffixes", () => {
  const root = mkdtempSync(join(tmpdir(), "jobsentinel-macos-model-payload-"));

  try {
    mkdirSync(join(root, "Contents", "Resources", "models"), { recursive: true });
    writeFileSync(join(root, "Contents", "Resources", "models", "MODEL.SAFETENSORS"), "fixture");
    writeFileSync(join(root, "Contents", "Resources", "models", "tokenizer.json"), "fixture");

    assert.deepEqual(
      modelPayloadFiles(root),
      [join("Contents", "Resources", "models", "MODEL.SAFETENSORS")],
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("macOS package verifier rejects a missing runtime profile value", () => {
  assert.throws(
    () => parseArgs(["--dmg", "JobSentinel.dmg", "--runtime-profile"], "arm64"),
    /macOS runtime profile requires a value/,
  );
});
