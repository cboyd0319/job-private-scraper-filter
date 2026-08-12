import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SemanticMatchingDiagnosticsPanel } from "./SemanticMatchingDiagnosticsPanel";
import {
  cancelSemanticMatchingModelDownload,
  downloadSemanticMatchingModels,
  getSemanticMatchingDiagnostics,
  removeSemanticMatchingModels,
  repairSemanticMatchingModelCache,
} from "./semanticMatchingDiagnostics";

vi.mock("../../../platform/tauri/events", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("./semanticMatchingDiagnostics", async (importOriginal) => ({
  ...(await importOriginal<
    typeof import("./semanticMatchingDiagnostics")
  >()),
  cancelSemanticMatchingModelDownload: vi.fn(),
  downloadSemanticMatchingModels: vi.fn(),
  getSemanticMatchingDiagnostics: vi.fn(),
  removeSemanticMatchingModels: vi.fn(),
  repairSemanticMatchingModelCache: vi.fn(),
}));

const mockGetDiagnostics = vi.mocked(getSemanticMatchingDiagnostics);
const mockRepairModel = vi.mocked(repairSemanticMatchingModelCache);
const mockDownloadModels = vi.mocked(downloadSemanticMatchingModels);
const mockCancelDownload = vi.mocked(cancelSemanticMatchingModelDownload);
const mockRemoveModels = vi.mocked(removeSemanticMatchingModels);

function disabledDiagnostics() {
  return {
    build_enabled: false,
    runtime_status: "disabled_in_this_build" as const,
    active_profile: "Built-in local matching rules",
    privacy_mode: "Local only. No resume or job text leaves this device.",
    manifest_hash: null,
    models: [],
    scoring_signals: [
      {
        id: "exact_skills",
        label: "Exact skills",
        state: "Always on",
        explanation: "Matches visible skills and aliases.",
      },
    ],
    eval_contract: ["Direct evidence must outrank keyword-only near misses."],
    user_action:
      "Advanced local model diagnostics appear in embedded-ML builds.",
  };
}

function readyDiagnostics() {
  return {
    build_enabled: true,
    runtime_status: "ready" as const,
    active_profile: "Qwen3 embedding plus Qwen3 reranker",
    privacy_mode: "Local only. Model downloads fetch model files only.",
    manifest_hash: "b".repeat(64),
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
        required_files_present: 3,
        locked_size_bytes: 641000000,
        downloaded: true,
        cache_present: true,
        health: "ready" as const,
        required_for_qwen3_runtime: true,
      },
      {
        id: "qwen3-reranker-0.6b",
        role: "Default reranker",
        repo: "Qwen/Qwen3-Reranker-0.6B",
        revision: "e61197ed45024b0ed8a2d74b80b4d909f1255473",
        backend: "qwen3-reranker-candle",
        license: "Apache-2.0",
        dimension: null,
        max_tokens: 32768,
        required_files: 4,
        required_files_present: 4,
        locked_size_bytes: 690000000,
        downloaded: true,
        cache_present: true,
        health: "ready" as const,
        required_for_qwen3_runtime: true,
      },
    ],
    scoring_signals: [
      {
        id: "qwen3_reranker",
        label: "Qwen3 reranker",
        state: "Embedded-ML builds",
        explanation: "Reranks a bounded set of candidate evidence.",
      },
    ],
    eval_contract: [
      "Generated advice must stay separate from real job evidence.",
    ],
    user_action: null,
  };
}

function missingDiagnostics() {
  const diagnostics = readyDiagnostics();
  diagnostics.runtime_status = "needs_model_download";
  diagnostics.active_profile = "Exact-only deterministic matching";
  diagnostics.models.forEach((model) => {
    model.downloaded = false;
    model.cache_present = false;
    model.health = "missing";
    model.required_files_present = 0;
  });
  return diagnostics;
}

describe("SemanticMatchingDiagnosticsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCancelDownload.mockResolvedValue(false);
  });

  it("shows the local fallback when embedded ML is not enabled", async () => {
    mockGetDiagnostics.mockResolvedValueOnce(disabledDiagnostics());

    render(<SemanticMatchingDiagnosticsPanel />);

    expect(
      await screen.findByText("Using built-in local matching rules"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Built-in local matching rules"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No resume or job text leaves this device/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Advanced local model details appear/),
    ).toBeInTheDocument();
  });

  it("shows Qwen3 model cache details and refreshes on request", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(readyDiagnostics());

    render(<SemanticMatchingDiagnosticsPanel />);

    expect(
      await screen.findByText("Advanced local matching is ready"),
    ).toBeInTheDocument();
    expect(screen.getByText("Qwen/Qwen3-Embedding-0.6B")).toBeInTheDocument();
    expect(screen.getByText("Qwen/Qwen3-Reranker-0.6B")).toBeInTheDocument();
    expect(screen.getAllByText("Ready")).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: /Refresh/ }));

    await waitFor(() => {
      expect(mockGetDiagnostics).toHaveBeenCalledTimes(2);
    });
  });

  it("offers native-reviewed repair only for an integrity-invalid default model", async () => {
    const user = userEvent.setup();
    const damaged = readyDiagnostics();
    damaged.runtime_status = "misconfigured";
    damaged.models[0].downloaded = false;
    damaged.models[0].health = "integrity_mismatch";
    mockGetDiagnostics.mockResolvedValue(damaged);
    mockRepairModel.mockResolvedValueOnce(true);

    render(<SemanticMatchingDiagnosticsPanel />);

    expect(await screen.findByText("Integrity check failed")).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Review local model repair" }),
    );

    expect(mockRepairModel).toHaveBeenCalledWith("qwen3-embedding-0.6b");
    await waitFor(() => {
      expect(mockGetDiagnostics).toHaveBeenCalledTimes(2);
    });
  });

  it("keeps built-in matching available while a governed download can be canceled", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(missingDiagnostics());
    mockDownloadModels.mockReturnValue(new Promise(() => {}));
    mockCancelDownload.mockResolvedValueOnce(true);

    render(<SemanticMatchingDiagnosticsPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Set up stronger local matching",
      }),
    );
    expect(
      screen.getByText(/Built-in matching remains available/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("progressbar", {
        name: "Local model download progress",
      }),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Cancel model download" }),
    );
    expect(mockCancelDownload).toHaveBeenCalledOnce();
  });

  it("cancels an active download when the settings panel unmounts", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(missingDiagnostics());
    mockDownloadModels.mockReturnValue(new Promise(() => {}));
    mockCancelDownload.mockResolvedValueOnce(true);
    const view = render(<SemanticMatchingDiagnosticsPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Set up stronger local matching",
      }),
    );
    view.unmount();

    await waitFor(() => expect(mockCancelDownload).toHaveBeenCalledOnce());
  });

  it("does not fire a cancel when a download completes normally", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(missingDiagnostics());
    mockDownloadModels.mockResolvedValueOnce(true);

    const view = render(<SemanticMatchingDiagnosticsPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Set up stronger local matching",
      }),
    );
    await waitFor(() => {
      expect(mockGetDiagnostics).toHaveBeenCalledTimes(2);
    });
    view.unmount();

    expect(mockCancelDownload).not.toHaveBeenCalled();
  });

  it("recovers from an orphaned backend download by canceling it", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(missingDiagnostics());
    mockDownloadModels.mockRejectedValueOnce(
      new Error("Another local model action is already running."),
    );
    mockCancelDownload.mockResolvedValueOnce(true);

    render(<SemanticMatchingDiagnosticsPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Set up stronger local matching",
      }),
    );

    expect(mockCancelDownload).toHaveBeenCalledOnce();
    expect(
      await screen.findByText(
        "A previous model download was still running and has been canceled. Try again.",
      ),
    ).toBeInTheDocument();
  });

  it("offers native-reviewed removal without removing the built-in fallback", async () => {
    const user = userEvent.setup();
    mockGetDiagnostics.mockResolvedValue(readyDiagnostics());
    mockRemoveModels.mockResolvedValueOnce(true);

    render(<SemanticMatchingDiagnosticsPanel />);

    await user.click(
      await screen.findByRole("button", { name: "Remove local models" }),
    );

    expect(mockRemoveModels).toHaveBeenCalledOnce();
    await waitFor(() => {
      expect(mockGetDiagnostics).toHaveBeenCalledTimes(2);
    });
  });
});
