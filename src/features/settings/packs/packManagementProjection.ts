/** Validates the desktop pack-management projection before the renderer trusts it. */

export type PackState =
  | "needs_review"
  | "ready"
  | "disabled"
  | "quarantined"
  | "removed";

type ReleaseState =
  | "staged"
  | "self_tested"
  | "ready"
  | "quarantined"
  | "removed";

export type QuarantineReason =
  | "self_test_failed"
  | "trust_revoked"
  | "interrupted"
  | "artifact_missing"
  | "integrity_failed";

export interface PackReleaseReview {
  releaseSequence: number;
  packVersion: string;
  packType: string;
  executionClass: string;
  state: ReleaseState;
  quarantineReason: QuarantineReason | null;
  artifactCleanupPending: boolean;
  isActive: boolean;
  isRollback: boolean;
  isHighWater: boolean;
  publisherName: string;
  license: string;
  minimumAppVersion: string;
  maximumAppVersion: string;
  payloadBytes: number;
  fixtureSummary: string;
  privacyLabels: string[];
  allowedDataCategories: string[];
  allowedTaskKinds: string[];
  allowedActions: string[];
  approvalGates: string[];
  gatewayPolicyId: string | null;
  externalDestinations: string[];
  usesExternalAi: boolean;
}

export interface PackManagementReview {
  publisherKeyId: string;
  publisherPublicKeySha256: string;
  packId: string;
  state: PackState;
  updateAvailable: boolean;
  cleanupPending: boolean;
  generation: number;
  currentRelease: PackReleaseReview;
  releases: PackReleaseReview[];
}

const RELEASE_FIELDS: readonly (keyof PackReleaseReview)[] = [
  "releaseSequence", "packVersion", "packType", "executionClass", "state",
  "quarantineReason", "artifactCleanupPending", "isActive", "isRollback",
  "isHighWater", "publisherName", "license", "minimumAppVersion",
  "maximumAppVersion", "payloadBytes", "fixtureSummary", "privacyLabels",
  "allowedDataCategories", "allowedTaskKinds", "allowedActions",
  "approvalGates", "gatewayPolicyId", "externalDestinations", "usesExternalAi",
];
const PACK_STATES = new Set<PackState>([
  "needs_review",
  "ready",
  "disabled",
  "quarantined",
  "removed",
]);
const RELEASE_STATES = new Set<ReleaseState>([
  "staged",
  "self_tested",
  "ready",
  "quarantined",
  "removed",
]);
const QUARANTINE_REASONS = new Set<QuarantineReason>([
  "self_test_failed",
  "trust_revoked",
  "interrupted",
  "artifact_missing",
  "integrity_failed",
]);
const PACK_TYPES = new Set([
  "skill", "agent", "workflow", "role", "region", "source", "rubric",
  "evaluation", "template", "os_helper",
]);
const EXECUTION_CLASSES = new Set([
  "static_content", "reviewed_typed_workflow",
]);
const PRIVACY_LABELS = new Set([
  "local_only", "external_ai_optional", "sensitive", "public_data_only",
]);
const DATA_CATEGORIES = new Set([
  "public_job_posting", "resume_evidence", "application_history", "career_goals",
  "pay_preferences", "location_preferences", "military_service",
  "clearance_claim", "protected_veteran_answer",
]);
const TASK_KINDS = new Set([
  "source_check", "evidence_review", "draft_packet", "backup", "export",
  "pack_install",
]);
const ACTIONS = new Set([
  "read_selected_case_file", "read_selected_resume_evidence",
  "read_public_job_posting", "create_draft_local_note",
  "create_draft_application_packet", "create_reminder", "open_browser_link",
  "request_source_check", "request_external_ai", "write_local_event",
]);
const APPROVAL_GATES = new Set(["per_execution_review"]);
const EXTERNAL_AI_GATEWAY_POLICY = "jobsentinel.external-ai-gateway.v1";

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isKnownArray(value: unknown, allowed: Set<string>): value is string[] {
  return (
    isStringArray(value) &&
    value.every((item) => allowed.has(item)) &&
    new Set(value).size === value.length
  );
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isSha256(value: unknown): value is string {
  return typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
}

function isSafeCount(value: unknown, positive = false): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    (positive ? value > 0 : value >= 0)
  );
}

function isRelease(value: unknown): value is PackReleaseReview {
  if (!isRecord(value)) return false;
  const reason = value.quarantineReason;
  return (
    isSafeCount(value.releaseSequence, true) &&
    isNonEmptyString(value.packVersion) &&
    typeof value.packType === "string" &&
    PACK_TYPES.has(value.packType) &&
    typeof value.executionClass === "string" &&
    EXECUTION_CLASSES.has(value.executionClass) &&
    typeof value.state === "string" &&
    RELEASE_STATES.has(value.state as ReleaseState) &&
    (reason === null ||
      (typeof reason === "string" &&
        QUARANTINE_REASONS.has(reason as QuarantineReason))) &&
    typeof value.artifactCleanupPending === "boolean" &&
    typeof value.isActive === "boolean" &&
    typeof value.isRollback === "boolean" &&
    typeof value.isHighWater === "boolean" &&
    isNonEmptyString(value.publisherName) &&
    isNonEmptyString(value.license) &&
    isNonEmptyString(value.minimumAppVersion) &&
    isNonEmptyString(value.maximumAppVersion) &&
    isSafeCount(value.payloadBytes) &&
    isNonEmptyString(value.fixtureSummary) &&
    isKnownArray(value.privacyLabels, PRIVACY_LABELS) &&
    isKnownArray(value.allowedDataCategories, DATA_CATEGORIES) &&
    isKnownArray(value.allowedTaskKinds, TASK_KINDS) &&
    isKnownArray(value.allowedActions, ACTIONS) &&
    isKnownArray(value.approvalGates, APPROVAL_GATES) &&
    (value.gatewayPolicyId === null ||
      value.gatewayPolicyId === EXTERNAL_AI_GATEWAY_POLICY) &&
    isStringArray(value.externalDestinations) &&
    new Set(value.externalDestinations).size ===
      value.externalDestinations.length &&
    typeof value.usesExternalAi === "boolean" &&
    value.usesExternalAi ===
      value.allowedActions.includes("request_external_ai") &&
    (value.usesExternalAi
      ? value.gatewayPolicyId === EXTERNAL_AI_GATEWAY_POLICY &&
        value.externalDestinations.length === 1 &&
        value.externalDestinations[0] === EXTERNAL_AI_GATEWAY_POLICY
      : value.gatewayPolicyId === null &&
        value.externalDestinations.length === 0) &&
    (value.executionClass === "static_content"
      ? value.allowedTaskKinds.length === 0 &&
        value.allowedActions.length === 0 &&
        value.approvalGates.length === 0
      : value.allowedDataCategories.length > 0 &&
        value.allowedTaskKinds.length > 0 &&
        value.allowedActions.length > 0 &&
        value.approvalGates.includes("per_execution_review"))
  );
}

function sameRelease(left: PackReleaseReview, right: PackReleaseReview): boolean {
  return RELEASE_FIELDS.every(
    (field) => JSON.stringify(left[field]) === JSON.stringify(right[field]),
  );
}

function parsePack(value: unknown): PackManagementReview | null {
  if (
    !isRecord(value) ||
    !isNonEmptyString(value.publisherKeyId) ||
    !isSha256(value.publisherPublicKeySha256) ||
    !isNonEmptyString(value.packId) ||
    typeof value.state !== "string" ||
    !PACK_STATES.has(value.state as PackState) ||
    typeof value.updateAvailable !== "boolean" ||
    typeof value.cleanupPending !== "boolean" ||
    !isSafeCount(value.generation) ||
    !isRelease(value.currentRelease) ||
    !Array.isArray(value.releases) ||
    !value.releases.every(isRelease)
  ) return null;

  const currentResponse = value.currentRelease as PackReleaseReview;
  const releases = value.releases as PackReleaseReview[];
  const currentMatches = releases.filter(
    (release) => release.releaseSequence === currentResponse.releaseSequence,
  );
  const highWater = releases.filter((release) => release.isHighWater);
  const active = releases.filter((release) => release.isActive);
  const currentRelease = currentMatches[0];
  const highWaterRelease = highWater[0];
  if (
    !currentRelease ||
    !highWaterRelease ||
    !sameRelease(currentResponse, currentRelease) ||
    currentMatches.length !== 1 ||
    highWater.length !== 1 ||
    active.length > 1 ||
    releases.filter((release) => release.isRollback).length > 1 ||
    new Set(releases.map((release) => release.releaseSequence)).size !==
      releases.length ||
    highWaterRelease.releaseSequence !==
      Math.max(...releases.map((release) => release.releaseSequence))
  ) return null;

  const state = value.state as PackState;
  const stateIsValid =
    ((state === "ready" || state === "disabled") &&
      currentRelease.isActive && currentRelease.state === "ready") ||
    (state === "needs_review" && active.length === 0 &&
      currentRelease.isHighWater && currentRelease.state === "self_tested" &&
      currentRelease.quarantineReason === null) ||
    (state === "quarantined" && active.length === 0 &&
      currentRelease.isHighWater &&
      ["staged", "quarantined"].includes(currentRelease.state)) ||
    (state === "removed" && active.length === 0 &&
      currentRelease.isHighWater && currentRelease.state === "removed");
  const updateAvailable =
    (state === "ready" || state === "disabled") &&
    !highWaterRelease.isActive &&
    highWaterRelease.state === "self_tested" &&
    highWaterRelease.quarantineReason === null;
  if (
    !stateIsValid ||
    value.updateAvailable !== updateAvailable ||
    value.cleanupPending !==
      releases.some((release) => release.artifactCleanupPending)
  ) return null;

  return { ...(value as unknown as PackManagementReview), currentRelease };
}

export function parsePackManagementReviews(
  value: unknown,
): PackManagementReview[] {
  const packs = Array.isArray(value) ? value.map(parsePack) : [];
  if (!Array.isArray(value) || packs.some((pack) => !pack)) {
    throw new Error("Pack information is invalid");
  }
  return packs as PackManagementReview[];
}
