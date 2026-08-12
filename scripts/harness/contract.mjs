// Aggregates canonical repository, state, startup, and verification contract checks.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join } from "node:path";

import { collectRepositoryFileSizeViolations } from "./checks/repo-file-size.mjs";
import { collectFileDescriptionViolations } from "./checks/file-descriptions.mjs";
import { collectV3PlanViolations } from "./checks/v3-plan.mjs";
import { collectStateViolations } from "./state.mjs";
import { checkRepositoryArchitecture } from "../checks/repository-architecture.mjs";

export const harnessManifestPath = "scripts/harness/contracts/harness.json";
export const harnessManifestSchema = 1;
export const noCiExceptionId = "pre-alpha-private-no-ci";

const noCiAffectedRequirements = [
  "HE-SK-WC-11",
  "HE-SK-BD-09",
  "HE-SK-BD-17",
  "HE-SK-BD-18",
  "HE-SHC-GA-07",
  "HE-SHC-RP-01",
  "HE-RS-TS-05",
  "HE-ET-PV-02",
  "HE-VF-FD-06",
  "HE-VF-CI-01",
  "HE-VF-CI-02",
  "HE-VF-CI-04",
  "HE-VF-CI-05",
];

function readJson(root, path, violations) {
  try {
    return JSON.parse(readFileSync(join(root, path), "utf8"));
  } catch (error) {
    violations.push(`fix invalid ${path}: ${error instanceof Error ? error.message : String(error)}`);
    return null;
  }
}

function physicalLines(text) {
  return text.length === 0 ? 0 : text.split(/\r?\n/).length - (/\r?\n$/.test(text) ? 1 : 0);
}

function validateStartupBudgets(root, manifest, violations) {
  const startup = manifest.startup ?? {};
  const startupFiles = Array.isArray(startup.files) ? startup.files : [];
  let totalBytes = 0;
  for (const path of startupFiles) {
    if (!existsSync(join(root, path))) {
      violations.push(`missing declared startup file: ${path}`);
      continue;
    }
    totalBytes += statSync(join(root, path)).size;
  }
  if (!Number.isInteger(startup.max_bytes) || startup.max_bytes < 0) violations.push("startup.max_bytes must be a measured nonnegative integer budget");
  else if (totalBytes > startup.max_bytes) violations.push(`startup context exceeds total byte budget: ${totalBytes} > ${startup.max_bytes}`);
  const stateReads = startupFiles.filter((path) => path === "docs/harness/current-status.md" || path === "scripts/harness/state/feature-list.json").length;
  if (!Number.isInteger(startup.max_state_reads) || stateReads > startup.max_state_reads) violations.push(`startup state reads exceed budget: ${stateReads} > ${String(startup.max_state_reads)}`);
  const featureState = readJson(root, "scripts/harness/state/feature-list.json", violations);
  const featureItems = Array.isArray(featureState?.features) ? featureState.features : [];
  const stateBudgets = startup.state_budgets ?? {};
  if (!Number.isInteger(stateBudgets.max_feature_items) || featureItems.length > stateBudgets.max_feature_items) {
    violations.push(`feature state item count exceeds budget: ${featureItems.length} > ${String(stateBudgets.max_feature_items)}`);
  }
  const maxFeatureRecordBytes = featureItems.reduce((max, feature) => Math.max(max, Buffer.byteLength(JSON.stringify(feature))), 0);
  if (!Number.isInteger(stateBudgets.max_feature_record_bytes) || maxFeatureRecordBytes > stateBudgets.max_feature_record_bytes) {
    violations.push(`feature state record exceeds byte budget: ${maxFeatureRecordBytes} > ${String(stateBudgets.max_feature_record_bytes)}`);
  }
  const instructions = existsSync(join(root, "AGENTS.md")) ? readFileSync(join(root, "AGENTS.md"), "utf8") : "";
  const measuredHops = startupFiles.filter((path) => path !== "AGENTS.md").every((path) => instructions.includes(`\`${path}\``)) ? 1 : 2;
  if (!Number.isInteger(startup.max_hops) || measuredHops > startup.max_hops) violations.push(`startup loading hops exceed budget: ${measuredHops} > ${String(startup.max_hops)}`);
  for (const [path, budget] of Object.entries(startup.per_file_budgets ?? {})) {
    if (!existsSync(join(root, path))) continue;
    const text = readFileSync(join(root, path), "utf8");
    if (physicalLines(text) > budget.max_lines) {
      violations.push(`${path} exceeds startup line budget: ${physicalLines(text)} > ${budget.max_lines}`);
    }
    if (Buffer.byteLength(text) > budget.max_bytes) {
      violations.push(`${path} exceeds startup byte budget: ${Buffer.byteLength(text)} > ${budget.max_bytes}`);
    }
    const maxLineBytes = text.split(/\r?\n/).reduce((max, line) => Math.max(max, Buffer.byteLength(line)), 0);
    if (maxLineBytes > budget.max_line_bytes) violations.push(`${path} exceeds startup maximum line-byte budget: ${maxLineBytes} > ${budget.max_line_bytes}`);
  }
}

function validateOwners(root, manifest, violations) {
  const required = {
    "instructions.canonical": manifest.owners?.instructions?.canonical,
    "tools.registry": manifest.owners?.tools?.registry,
    "environment.manifest": manifest.owners?.environment?.manifest,
    "state.current": manifest.owners?.state?.current,
    "state.features": manifest.owners?.state?.features,
    "feedback.sensor_registry": manifest.owners?.feedback?.sensor_registry,
    "feedback.acceptance_contract": manifest.owners?.feedback?.acceptance_contract,
    "feedback.hosted_workflow_root": manifest.owners?.feedback?.hosted_workflow_root,
    "architecture.documentation": manifest.owners?.architecture?.documentation,
    "architecture.policy": manifest.owners?.architecture?.policy,
    "architecture.rust_graph": manifest.owners?.architecture?.rust_graph,
  };
  for (const [owner, path] of Object.entries(required)) {
    if (typeof path !== "string" || !path.trim()) violations.push(`harness owner ${owner} is required`);
    else if (!existsSync(join(root, path))) violations.push(`harness owner ${owner} does not exist: ${path}`);
  }
  for (const path of [
    ...(manifest.owners?.instructions?.projections ?? []),
    ...(manifest.owners?.tools?.roots ?? []),
    ...(manifest.owners?.environment?.bootstrap ?? []),
    ...(manifest.owners?.environment?.native_equivalents ?? []),
    manifest.owners?.state?.evidence,
    manifest.owners?.feedback?.local_hook_root,
  ].filter(Boolean)) {
    if (!existsSync(join(root, path))) violations.push(`declared harness owner path does not exist: ${path}`);
  }
}

function validatePackageScripts(root, manifest, violations) {
  const packageJson = readJson(root, "package.json", violations);
  if (!packageJson) return;
  const required = [
    "harness:check", "harness:plan", "harness:session", "lint:file-description", "lint:file-size",
    "clean:generated", "typecheck", "test:smoke", "verify:full",
  ];
  for (const name of required) {
    if (typeof packageJson.scripts?.[name] !== "string" || !packageJson.scripts[name].trim()) {
      violations.push(`package.json must define ${name}`);
    }
  }
  for (const name of manifest.retired_commands ?? []) {
    if (Object.hasOwn(packageJson.scripts ?? {}, name)) violations.push(`remove retired package script: ${name}`);
  }
  const full = String(packageJson.scripts?.["verify:full"] ?? "");
  for (const command of ["harness:check", "lint:architecture", "test:run", "build", "verify:rust"]) {
    if (!full.includes(command)) violations.push(`verify:full must include ${command}`);
  }
  if (full.includes("clean:generated")) violations.push("verify:full must not delete diagnostic or generated evidence; cleanup is a separate explicit command");
}

function validateNativeLaunchers(root, violations) {
  const shell = readFileSync(join(root, "init.sh"), "utf8");
  const powershell = readFileSync(join(root, "init.ps1"), "utf8");
  if (!shell.includes('node scripts/harness/init.mjs "$@"')) violations.push("init.sh must be a thin harness-init.mjs launcher");
  if (!powershell.includes("node scripts/harness/init.mjs @args")) violations.push("init.ps1 must be a thin harness-init.mjs launcher");
  if (process.platform !== "win32" && (statSync(join(root, "init.sh")).mode & 0o111) === 0) {
    violations.push("init.sh must be executable");
  }
}

function validateInstructionProjections(root, manifest, violations) {
  for (const path of manifest.owners?.instructions?.projections ?? []) {
    if (!existsSync(join(root, path))) {
      violations.push(`missing instruction compatibility projection: ${path}`);
      continue;
    }
    const text = readFileSync(join(root, path), "utf8");
    if (physicalLines(text) > 8 || Buffer.byteLength(text) > 600) violations.push(`${path} must remain a thin instruction pointer`);
    if (!text.includes("sole repository instruction owner")) violations.push(`${path} must point to AGENTS.md as the sole repository instruction owner`);
    if (!text.includes("intentionally adds no repository guidance")) violations.push(`${path} must not duplicate repository guidance`);
  }
}

export function validateHostedWorkflows(root, manifest, violations) {
  const policy = manifest.hosted_workflows;
  const authoritative = Array.isArray(policy?.authoritative_ci) ? policy.authoritative_ci : [];
  if (policy?.ci_enabled === true) {
    if (authoritative.length === 0) violations.push("hosted_workflows.authoritative_ci must name the shared CI workflow when CI is enabled");
    for (const path of authoritative) {
      if (!existsSync(join(root, path))) {
        violations.push(`missing authoritative CI workflow: ${path}`);
        continue;
      }
      const text = readFileSync(join(root, path), "utf8");
      for (const trigger of ["pull_request:", "push:", "workflow_dispatch:"]) if (!text.includes(`  ${trigger}`)) violations.push(`${path} must enable ${trigger.slice(0, -1)} verification`);
      for (const command of ["npm run harness:check", "npm run lint:security", "npm run lint:skills", "npm run typecheck", "cargo test --workspace", "npm run test:e2e:smoke:budget"]) if (!text.includes(command)) violations.push(`${path} must run shared gate: ${command}`);
      for (const label of ["macos-26", "windows-11-arm"]) if (!text.includes(label)) violations.push(`${path} must define native portability coverage for ${label}`);
      if (text.includes("HOSTED_EXECUTION_DISABLED") || text.includes('if: ${{ false }}')) violations.push(`${path} must not contain a hosted-execution disable guard`);
    }
  } else if (policy?.ci_enabled === false) {
    if (authoritative.length !== 0) violations.push("hosted_workflows.authoritative_ci must be empty while CI is disabled");
    const exception = (manifest.canonical_requirement_exceptions ?? []).find(
      (candidate) => candidate?.id === noCiExceptionId,
    );
    if (!exception) {
      violations.push(`disabled CI requires named exception: ${noCiExceptionId}`);
    } else {
      if (exception.status !== "active") violations.push(`${noCiExceptionId} must be active while CI is disabled`);
      if (exception.authority !== "explicit user directive") violations.push(`${noCiExceptionId} must record explicit user authority`);
      if (!String(exception.canonical_status ?? "").includes("nonconforming user override")) {
        violations.push(`${noCiExceptionId} must not claim canonical harness-engineering conformance`);
      }
      for (const requirement of noCiAffectedRequirements) {
        if (!exception.affected_requirements?.includes(requirement)) {
          violations.push(`${noCiExceptionId} must name affected canonical requirement: ${requirement}`);
        }
      }
      for (const field of ["condition", "scope", "review_trigger", "retirement_condition"]) {
        if (typeof exception[field] !== "string" || !exception[field].trim()) {
          violations.push(`${noCiExceptionId}.${field} must be a nonempty string`);
        }
      }
    }

    const workflowRoot = join(root, manifest.owners?.feedback?.hosted_workflow_root ?? ".github/workflows");
    const automaticTriggers = /^ {2}(?:push|pull_request|pull_request_target|merge_group|schedule):/m;
    if (existsSync(workflowRoot)) {
      for (const file of readdirSync(workflowRoot).filter((name) => [".yml", ".yaml"].includes(extname(name))).sort()) {
        const path = `.github/workflows/${file}`;
        if (automaticTriggers.test(readFileSync(join(workflowRoot, file), "utf8"))) {
          violations.push(`${path} must not define automatic CI triggers while ${noCiExceptionId} is active`);
        }
      }
    }

    const projections = Array.isArray(policy.no_ci_projections) ? policy.no_ci_projections : [];
    if (projections.length === 0) violations.push("hosted_workflows.no_ci_projections must name current automation documentation");
    for (const path of projections) {
      if (!existsSync(join(root, path))) {
        violations.push(`missing no-CI documentation projection: ${path}`);
        continue;
      }
      const text = readFileSync(join(root, path), "utf8");
      if (!/(?:pre-alpha-private-no-ci|no hosted continuous integration)/i.test(text)) {
        violations.push(`${path} must state the active ${noCiExceptionId} posture`);
      }
      if (/\btests run in CI\b/i.test(text)) {
        violations.push(`${path} must not claim tests run in hosted CI while ${noCiExceptionId} is active`);
      }
    }
  } else {
    violations.push("hosted_workflows.ci_enabled must be a boolean");
  }
  for (const path of policy?.manual_release ?? []) {
    if (!existsSync(join(root, path))) {
      violations.push(`missing manual release workflow: ${path}`);
      continue;
    }
    const text = readFileSync(join(root, path), "utf8");
    if (!text.includes("  workflow_dispatch:")) violations.push(`${path} must require explicit manual dispatch`);
    if (text.includes("HOSTED_EXECUTION_DISABLED") || text.includes('if: ${{ false }}')) violations.push(`${path} must not retain a false execution projection`);
  }
}

function validateRetirement(root, manifest, violations) {
  for (const path of manifest.retired_paths ?? []) {
    if (existsSync(join(root, path))) violations.push(`remove retired harness path: ${path}`);
  }
  const retiredPhrases = (manifest.retired_commands ?? []).map((name) => `npm run ${name}`);
  for (const path of manifest.canonical_reference_paths ?? []) {
    if (!existsSync(join(root, path))) continue;
    const text = readFileSync(join(root, path), "utf8");
    for (const phrase of retiredPhrases) {
      if (text.includes(phrase)) violations.push(`${path} references retired command: ${phrase}`);
    }
  }
}

export function collectHarnessContractViolations(root) {
  const violations = [];
  const manifest = readJson(root, harnessManifestPath, violations);
  if (!manifest) return violations;
  if (manifest.schema_version !== harnessManifestSchema) violations.push(`harness manifest schema_version must be ${harnessManifestSchema}`);
  for (const path of manifest.required_pack ?? []) {
    if (!existsSync(join(root, path))) violations.push(`add required harness pack file: ${path}`);
  }
  validateOwners(root, manifest, violations);
  validateStartupBudgets(root, manifest, violations);
  validatePackageScripts(root, manifest, violations);
  validateNativeLaunchers(root, violations);
  validateInstructionProjections(root, manifest, violations);
  validateHostedWorkflows(root, manifest, violations);
  validateRetirement(root, manifest, violations);
  violations.push(...collectStateViolations(root));
  violations.push(...collectV3PlanViolations(root));
  violations.push(...collectFileDescriptionViolations(root));
  violations.push(...collectRepositoryFileSizeViolations(root));
  violations.push(...checkRepositoryArchitecture(root));
  return [...new Set(violations)].sort();
}
