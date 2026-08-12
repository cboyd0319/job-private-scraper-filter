// Verifies release readiness metadata, workflow, and platform publication gates.

import assert from "node:assert/strict";
import test from "node:test";

import {
  evaluateReleaseReadiness,
  evaluateReleaseReadinessFromInputs,
  formatReleaseReadinessReport,
  loadReleaseReadinessInputs,
  parseArgs,
} from "../../release/check-release-readiness.mjs";

test("release readiness accepts the current local v3.0.0 gate posture", () => {
  const report = evaluateReleaseReadiness({ env: {} });

  assert.equal(report.expectedVersion, "3.0.0");
  assert.deepEqual(
    report.criteria.filter((item) => !item.ok).map((item) => item.id),
    [],
  );
  assert.match(formatReleaseReadinessReport(report), /JobSentinel release readiness: PASS/);
  assert.match(formatReleaseReadinessReport(report), /INFO Windows: public asset upload gated/);
  assert.match(formatReleaseReadinessReport(report), /INFO Linux: public asset upload gated/);
});

test("release readiness rejects version metadata drift", () => {
  const report = evaluateReleaseReadiness({ version: "9.9.9", env: {} });

  assert(
    report.criteria.some(
      (item) => item.id === "release metadata matches expected version" && !item.ok,
    ),
  );
});

test("release readiness rejects missing release environment script", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    packageJson: {
      ...inputs.packageJson,
      scripts: {
        ...inputs.packageJson.scripts,
        "release:check-env": undefined,
      },
    },
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id === "release scripts expose environment, readiness, and public verification" &&
        !item.ok,
    ),
  );
});

test("release readiness rejects Windows upload without signed or unsigned-labeled gate", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("JOBSENTINEL_WINDOWS_UNSIGNED=true", ""),
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id ===
          "Windows public upload is signed or unsigned-labeled and checksum gated" &&
        !item.ok,
    ),
  );
});

test("release readiness rejects Windows uploads without NSIS bundle target", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    tauriConfig: {
      ...inputs.tauriConfig,
      bundle: {
        ...inputs.tauriConfig.bundle,
        targets: inputs.tauriConfig.bundle.targets.filter((target) => target !== "nsis"),
      },
    },
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id ===
          "Windows public upload is signed or unsigned-labeled and checksum gated" &&
        !item.ok,
    ),
  );
});

test("release readiness rejects Linux upload without package verification", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("dpkg-deb --contents", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "Linux public upload is package and checksum gated" && !item.ok,
    ),
  );
});

test("release readiness rejects release preflight without Node security sensors", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("npm run lint:security", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "release preflight blocks security scanners" && !item.ok,
    ),
  );
});

test("release readiness rejects release preflight without workflow static analysis", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("zizmorcore/zizmor-action@", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "release preflight blocks security scanners" && !item.ok,
    ),
  );
});

test("release readiness rejects prose lint without pinned Vale installation", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace(
      'VALE_ARCHIVE_SHA256: "c024d9c157874fb043d4f24a055d60050d1bb18755251f590593eed5bace1857"',
      'VALE_ARCHIVE_SHA256: ""',
    ),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "hosted prose lint installs pinned Vale" && !item.ok,
    ),
  );
});

test("release readiness rejects incomplete browser installation for the full E2E suite", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace(
      "playwright install --with-deps chromium webkit",
      "playwright install --with-deps chromium",
    ),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "full E2E preflight installs configured browsers" && !item.ok,
    ),
  );
});

test("release readiness rejects SQLx metadata checks without the pinned CLI", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace(
      "cargo install sqlx-cli --version 0.9.0",
      "cargo install sqlx-cli",
    ),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "Rust preflight installs pinned SQLx CLI" && !item.ok,
    ),
  );
});

test("release readiness rejects unverified GitHub release creation", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("--verify-tag", ""),
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id === "release workflow creates verified versioned staged release" && !item.ok,
    ),
  );
});

test("release readiness rejects staged release body without versioned release notes", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace(
      'release_notes_path="docs/releases/v${RELEASE_VERSION}.md"',
      'release_notes_path=""',
    ),
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id === "release workflow creates verified versioned staged release" && !item.ok,
    ),
  );
});

test("release readiness rejects staged release notes without create-release checkout", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const createReleaseCheckout = [
    "",
    "      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7.0.0",
    "        with:",
    "          persist-credentials: false",
    "",
    "      - name: Create staged release",
  ].join("\n");
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace(
      createReleaseCheckout,
      "\n      - name: Create staged release",
    ),
  });

  assert(
    report.criteria.some(
      (item) =>
        item.id === "release workflow creates verified versioned staged release" && !item.ok,
    ),
  );
});

test("release readiness rejects missing Agent Skills archive upload", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replace("package-agent-skills:", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "release workflow publishes downloadable Agent Skills" && !item.ok,
    ),
  );
});

test("release readiness rejects public verifier without Agent Skills checks", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    verifyPublicScript: inputs.verifyPublicScript.replaceAll("validateAgentSkillsArchiveContents", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "public release verifier covers installers and supply chain" && !item.ok,
    ),
  );
});

test("release readiness rejects public verifier without unsigned Windows label checks", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    verifyWorkflow: inputs.verifyWorkflow.replace("--require-windows-unsigned-label", ""),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "public release verifier covers installers and supply chain" && !item.ok,
    ),
  );
});

test("release readiness rejects Agent Skills packages without ZIP artifact", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    releaseWorkflow: inputs.releaseWorkflow.replaceAll("agent-skills.zip", "agent-skills.tar.gz"),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "release workflow publishes downloadable Agent Skills" && !item.ok,
    ),
  );
});

test("release readiness rejects Agent Skills docs without Windows ZIP guidance", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    readme: inputs.readme.replace("Use the ZIP archive on Windows", "Use either archive"),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "Agent Skills download docs stay Windows-portable" && !item.ok,
    ),
  );
});

test("release readiness rejects front-door docs without checksum guidance", () => {
  const inputs = loadReleaseReadinessInputs({ env: {} });
  const report = evaluateReleaseReadinessFromInputs({
    ...inputs,
    readme: inputs.readme.replace("verify the matching `.sha256` checksum", "open the download"),
  });

  assert(
    report.criteria.some(
      (item) => item.id === "front-door docs cover public release asset limits" && !item.ok,
    ),
  );
});

test("release readiness parses version flags", () => {
  assert.deepEqual(parseArgs(["--version", "v2.9.0"]), { version: "v2.9.0" });
  assert.deepEqual(parseArgs(["--tag=v2.9.0"]), { version: "v2.9.0" });
});
