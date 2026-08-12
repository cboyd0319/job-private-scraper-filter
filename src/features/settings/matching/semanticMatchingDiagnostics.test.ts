import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  cancelSemanticMatchingModelDownload,
  downloadSemanticMatchingModels,
  getSemanticMatchingDiagnostics,
  normalizeSemanticMatchingDiagnostics,
  removeSemanticMatchingModels,
  repairSemanticMatchingModelCache,
} from "./semanticMatchingDiagnostics";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

function diagnosticPayload() {
  return {
    build_enabled: true,
    runtime_status: "needs_model_download",
    active_profile: "Qwen3 embedding plus Qwen3 reranker",
    privacy_mode: "Local only",
    manifest_hash: "a".repeat(64),
    models: [
      {
        id: "qwen3-embedding-0.6b",
        role: "Default embedding",
        repo: "Qwen/Qwen3-Embedding-0.6B",
        revision: "97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3",
        backend: "qwen3-candle",
        license: "Apache-2.0",
        dimension: 768,
        max_tokens: 32768,
        required_files: 3,
        required_files_present: 2,
        locked_size_bytes: 123456,
        downloaded: false,
        cache_present: true,
        health: "incomplete",
        required_for_qwen3_runtime: true,
      },
    ],
    scoring_signals: [
      {
        id: "exact_skills",
        label: "Exact skills",
        state: "Always on",
        explanation: "Matches visible skills.",
      },
    ],
    eval_contract: ["Direct evidence must outrank keyword-only near misses."],
    user_action: "Download the pinned local models.",
  };
}

describe("semantic matching diagnostics service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads diagnostics through the Tauri command", async () => {
    mockInvoke.mockResolvedValueOnce(diagnosticPayload());

    const diagnostics = await getSemanticMatchingDiagnostics();

    expect(mockInvoke).toHaveBeenCalledWith("get_semantic_matching_diagnostics");
    expect(diagnostics.runtime_status).toBe("needs_model_download");
    expect(diagnostics.models[0].id).toBe("qwen3-embedding-0.6b");
    expect(diagnostics.models[0].dimension).toBe(768);
  });

  it("rejects an unknown status", () => {
    expect(() =>
      normalizeSemanticMatchingDiagnostics({
        ...diagnosticPayload(),
        runtime_status: "surprise",
      }),
    ).toThrow("unknown status");
  });

  it("rejects an unknown model cache health state", () => {
    const payload = diagnosticPayload();
    payload.models[0].health = "surprise";

    expect(() => normalizeSemanticMatchingDiagnostics(payload)).toThrow(
      "unknown model cache health",
    );
  });

  it("requests native-reviewed repair for one model id", async () => {
    mockInvoke.mockResolvedValueOnce(true);

    await expect(
      repairSemanticMatchingModelCache("qwen3-embedding-0.6b"),
    ).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith(
      "repair_semantic_matching_model_cache",
      { modelId: "qwen3-embedding-0.6b" },
    );
  });

  it.each([
    ["download_ml_model", downloadSemanticMatchingModels],
    ["cancel_ml_model_download", cancelSemanticMatchingModelDownload],
    ["remove_ml_models", removeSemanticMatchingModels],
  ])("validates the %s command response", async (command, action) => {
    mockInvoke.mockResolvedValueOnce(true);

    await expect(action()).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith(command);

    mockInvoke.mockResolvedValueOnce("yes");
    await expect(action()).rejects.toThrow("unreadable response");
  });

  it("rejects an unreadable repair response", async () => {
    mockInvoke.mockResolvedValueOnce("yes");

    await expect(
      repairSemanticMatchingModelCache("qwen3-embedding-0.6b"),
    ).rejects.toThrow("unreadable repair response");
  });
});
