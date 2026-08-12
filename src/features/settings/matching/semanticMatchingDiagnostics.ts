import { invoke } from "../../../platform/tauri";

export type SemanticMatchingRuntimeStatus =
  | "ready"
  | "needs_model_download"
  | "disabled_in_this_build"
  | "misconfigured";

export type ModelCacheHealth =
  | "missing"
  | "incomplete"
  | "integrity_mismatch"
  | "ready";

export interface SemanticMatchingModelDiagnostic {
  id: string;
  role: string;
  repo: string;
  revision: string;
  backend: string;
  license: string;
  dimension: number | null;
  max_tokens: number;
  required_files: number;
  required_files_present: number;
  locked_size_bytes: number | null;
  downloaded: boolean;
  cache_present: boolean;
  health: ModelCacheHealth;
  required_for_qwen3_runtime: boolean;
}

export interface SemanticMatchingSignal {
  id: string;
  label: string;
  state: string;
  explanation: string;
}

export interface SemanticMatchingDiagnostics {
  build_enabled: boolean;
  runtime_status: SemanticMatchingRuntimeStatus;
  active_profile: string;
  privacy_mode: string;
  manifest_hash: string | null;
  models: SemanticMatchingModelDiagnostic[];
  scoring_signals: SemanticMatchingSignal[];
  eval_contract: string[];
  user_action: string | null;
}

const STATUSES = new Set<SemanticMatchingRuntimeStatus>([
  "ready",
  "needs_model_download",
  "disabled_in_this_build",
  "misconfigured",
]);
const CACHE_HEALTH = new Set<ModelCacheHealth>([
  "missing",
  "incomplete",
  "integrity_mismatch",
  "ready",
]);

export async function getSemanticMatchingDiagnostics(): Promise<SemanticMatchingDiagnostics> {
  const payload = await invoke<unknown>("get_semantic_matching_diagnostics");
  return normalizeSemanticMatchingDiagnostics(payload);
}

export async function repairSemanticMatchingModelCache(
  modelId: string,
): Promise<boolean> {
  const repaired = await invoke<unknown>(
    "repair_semantic_matching_model_cache",
    { modelId },
  );
  if (typeof repaired !== "boolean") {
    throw new Error("Local model repair returned an unreadable repair response.");
  }
  return repaired;
}

export async function downloadSemanticMatchingModels(): Promise<boolean> {
  return booleanCommand("download_ml_model");
}

export async function cancelSemanticMatchingModelDownload(): Promise<boolean> {
  return booleanCommand("cancel_ml_model_download");
}

export async function removeSemanticMatchingModels(): Promise<boolean> {
  return booleanCommand("remove_ml_models");
}

export function formatSemanticMatchingBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 MB";
  if (bytes >= 1_000_000_000) {
    return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  }
  const mb = bytes / 1_000_000;
  return `${mb.toFixed(mb >= 100 ? 0 : 1)} MB`;
}

async function booleanCommand(command: string): Promise<boolean> {
  const result = await invoke<unknown>(command);
  if (typeof result !== "boolean") {
    throw new Error("Local model command returned an unreadable response.");
  }
  return result;
}

export function normalizeSemanticMatchingDiagnostics(
  value: unknown,
): SemanticMatchingDiagnostics {
  const record = asRecord(value);
  const status = stringField(record, "runtime_status");

  if (!STATUSES.has(status as SemanticMatchingRuntimeStatus)) {
    throw new Error("Local matching diagnostics returned an unknown status.");
  }

  return {
    build_enabled: booleanField(record, "build_enabled"),
    runtime_status: status as SemanticMatchingRuntimeStatus,
    active_profile: stringField(record, "active_profile"),
    privacy_mode: stringField(record, "privacy_mode"),
    manifest_hash: nullableStringField(record, "manifest_hash"),
    models: arrayField(record, "models").map(normalizeModelDiagnostic),
    scoring_signals: arrayField(record, "scoring_signals").map(normalizeSignal),
    eval_contract: arrayField(record, "eval_contract").map((item) =>
      typeof item === "string" ? item : "",
    ).filter(Boolean),
    user_action: nullableStringField(record, "user_action"),
  };
}

function normalizeModelDiagnostic(value: unknown): SemanticMatchingModelDiagnostic {
  const record = asRecord(value);
  const health = stringField(record, "health");
  if (!CACHE_HEALTH.has(health as ModelCacheHealth)) {
    throw new Error(
      "Local matching diagnostics returned an unknown model cache health.",
    );
  }

  return {
    id: stringField(record, "id"),
    role: stringField(record, "role"),
    repo: stringField(record, "repo"),
    revision: stringField(record, "revision"),
    backend: stringField(record, "backend"),
    license: stringField(record, "license"),
    dimension: nullableNumberField(record, "dimension"),
    max_tokens: numberField(record, "max_tokens"),
    required_files: numberField(record, "required_files"),
    required_files_present: numberField(record, "required_files_present"),
    locked_size_bytes: nullableNumberField(record, "locked_size_bytes"),
    downloaded: booleanField(record, "downloaded"),
    cache_present: booleanField(record, "cache_present"),
    health: health as ModelCacheHealth,
    required_for_qwen3_runtime: booleanField(record, "required_for_qwen3_runtime"),
  };
}

function normalizeSignal(value: unknown): SemanticMatchingSignal {
  const record = asRecord(value);

  return {
    id: stringField(record, "id"),
    label: stringField(record, "label"),
    state: stringField(record, "state"),
    explanation: stringField(record, "explanation"),
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value as Record<string, unknown>;
}

function arrayField(record: Record<string, unknown>, key: string): unknown[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}

function stringField(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}

function nullableStringField(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  if (value === null || value === undefined) return null;
  if (typeof value !== "string") {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}

function numberField(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}

function nullableNumberField(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  if (value === null || value === undefined) return null;
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}

function booleanField(record: Record<string, unknown>, key: string): boolean {
  const value = record[key];
  if (typeof value !== "boolean") {
    throw new Error("Local matching diagnostics returned an unreadable response.");
  }
  return value;
}
