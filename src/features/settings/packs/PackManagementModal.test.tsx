/** Verifies pack management loading, lifecycle review, and safe failure states. */

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "../../../platform/tauri";
import { SettingsHelpStatusSection } from "../support/SettingsSupportSections";
import { PackManagementModal } from "./PackManagementModal";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../support/ErrorLogPanel", () => ({
  ErrorLogPanel: () => <div />,
}));

const mockInvoke = vi.mocked(invoke);

function release(overrides: Record<string, unknown> = {}) {
  return {
    releaseSequence: 2,
    packVersion: "3.0.2",
    packType: "source",
    executionClass: "static_content",
    state: "ready",
    quarantineReason: null,
    artifactCleanupPending: false,
    isActive: true,
    isRollback: false,
    isHighWater: true,
    publisherName: "JobSentinel",
    license: "MIT",
    minimumAppVersion: "3.0.0",
    maximumAppVersion: "3.2.0",
    payloadBytes: 2048,
    fixtureSummary: "Three reviewed source fixtures",
    privacyLabels: ["local_only"],
    allowedDataCategories: ["public_job_posting"],
    allowedTaskKinds: [],
    allowedActions: [],
    approvalGates: [],
    gatewayPolicyId: null,
    externalDestinations: [],
    usesExternalAi: false,
    ...overrides,
  };
}

function pack(overrides: Record<string, unknown> = {}) {
  const currentRelease = overrides.currentRelease ?? release();
  return {
    publisherKeyId: "jobsentinel-release-v1",
    publisherPublicKeySha256:
      "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef0123",
    packId: "jobsentinel.sources.us",
    state: "ready",
    updateAvailable: false,
    cleanupPending: false,
    generation: 4,
    currentRelease,
    releases: overrides.releases ?? [currentRelease],
    ...overrides,
  };
}

describe("PackManagementModal", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("shows the empty installed-pack state", async () => {
    mockInvoke.mockResolvedValueOnce([]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(
      await screen.findByText("No packs are installed."),
    ).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("list_pack_management");
  });

  it("loads only after the user opens the read-only pack view from settings", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([]);

    render(<SettingsHelpStatusSection onShowHealthDashboard={vi.fn()} />);

    expect(mockInvoke).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "View Packs" }));

    expect(await screen.findByRole("dialog", { name: "Packs" })).toBeInTheDocument();
    expect(screen.getByText(/This view is read-only/)).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("list_pack_management");
  });

  it("shows review facts, updates, quarantine failures, cleanup, and history", async () => {
    const reviewedSource = {
      allowedActions: ["read_public_job_posting", "request_source_check"],
      allowedTaskKinds: ["source_check"],
      approvalGates: ["per_execution_review"],
      executionClass: "reviewed_typed_workflow",
    };
    const outsideAiUpdate = {
      ...reviewedSource,
      allowedActions: ["read_public_job_posting", "request_external_ai"],
      externalDestinations: ["jobsentinel.external-ai-gateway.v1"],
      gatewayPolicyId: "jobsentinel.external-ai-gateway.v1",
      privacyLabels: ["external_ai_optional", "public_data_only"],
      usesExternalAi: true,
    };
    mockInvoke.mockResolvedValueOnce([
      pack({
        cleanupPending: true,
        updateAvailable: true,
        currentRelease: release({
          ...reviewedSource,
          isHighWater: false,
          releaseSequence: 1,
          packVersion: "3.0.1",
        }),
        releases: [
          release({
            ...reviewedSource,
            releaseSequence: 1,
            packVersion: "3.0.1",
            isHighWater: false,
          }),
          release({
            ...reviewedSource,
            artifactCleanupPending: true,
            isActive: false,
            isHighWater: false,
            releaseSequence: 2,
            quarantineReason: "self_test_failed",
            state: "quarantined",
          }),
          release({
            ...outsideAiUpdate,
            isActive: false,
            releaseSequence: 3,
            state: "self_tested",
          }),
        ],
      }),
      pack({
        packId: "jobsentinel.sources.review",
        state: "quarantined",
        cleanupPending: true,
        currentRelease: release({
          artifactCleanupPending: true,
          isActive: false,
          quarantineReason: "integrity_failed",
          state: "quarantined",
        }),
      }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    const readyPack = await screen.findByRole("article", {
      name: "JobSentinel jobsentinel.sources.us",
    });
    expect(
      within(readyPack).getByLabelText("Pack status: Ready"),
    ).toBeInTheDocument();
    expect(within(readyPack).getByText("Update available")).toBeInTheDocument();
    const currentReview = within(readyPack).getByRole("region", {
      name: "Current release review",
    });
    expect(
      within(currentReview).getByText("Three reviewed source fixtures"),
    ).toBeInTheDocument();
    expect(
      within(currentReview).getByText("Does not use outside AI"),
    ).toBeInTheDocument();
    expect(
      within(currentReview).getByText("Reviewed Typed Workflow"),
    ).toBeInTheDocument();
    expect(
      within(currentReview).getByText(
        "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef0123",
      ),
    ).toBeInTheDocument();
    expect(
      within(currentReview).getByText("Publisher key fingerprint (SHA-256)"),
    ).toBeInTheDocument();
    expect(
      within(currentReview).getByText(/Tasks:/),
    ).toHaveTextContent("Tasks: Source Check");
    expect(
      within(currentReview).getByText(/Approval:/),
    ).toHaveTextContent("Approval: Per Execution Review");
    expect(
      within(currentReview).getByText("Adds source support"),
    ).toBeInTheDocument();

    const quarantinedPack = screen.getByRole("article", {
      name: "JobSentinel jobsentinel.sources.review",
    });
    expect(
      within(quarantinedPack).getAllByText(
        "Installed file did not pass verification",
      ),
    ).toHaveLength(2);
    expect(
      within(quarantinedPack).getByText("Artifact cleanup needs another attempt"),
    ).toBeInTheDocument();

    await userEvent.click(within(readyPack).getByText("Release history"));
    expect(within(readyPack).getByText("Release 2")).toBeInTheDocument();
    expect(
      within(readyPack).getByText("Pack self-test failed"),
    ).toBeInTheDocument();
    expect(within(readyPack).getByText("Cleanup pending")).toBeInTheDocument();

    const update = within(readyPack).getByRole("listitem", {
      name: "Release 3",
    });
    await userEvent.click(
      within(update).getByText("Review signed permissions"),
    );
    expect(within(update).getByText(/Actions:/)).toHaveTextContent(
      "Request External AI",
    );
    expect(within(update).getByText(/Gateway policy:/)).toHaveTextContent(
      "jobsentinel.external-ai-gateway.v1",
    );
  });

  it.each([
    ["needs_review", "Needs review"],
    ["disabled", "Disabled"],
    ["removed", "Removed"],
  ])("shows the %s lifecycle state", async (state, accessibleState) => {
    const currentRelease = release({
      isActive: state === "disabled",
      state:
        state === "needs_review"
          ? "self_tested"
          : state === "removed"
            ? "removed"
            : "ready",
    });
    mockInvoke.mockResolvedValueOnce([
      pack({ state, currentRelease, releases: [currentRelease] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(
      await screen.findByLabelText(`Pack status: ${accessibleState}`),
    ).toBeInTheDocument();
  });

  it("fails closed when the response contains an unknown signed fact", async () => {
    const invalidRelease = release({ packType: "script" });
    mockInvoke.mockResolvedValueOnce([
      pack({ currentRelease: invalidRelease, releases: [invalidRelease] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack information could not be loaded.",
    );
    expect(screen.queryByText("Script")).not.toBeInTheDocument();
  });

  it.each([
    undefined,
    "A3B1C9D7E5F40123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123",
    "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef012",
    "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef012z",
  ])("fails closed when the publisher fingerprint is invalid: %s", async (publisherPublicKeySha256) => {
    mockInvoke.mockResolvedValueOnce([
      pack({ publisherPublicKeySha256 }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack information could not be loaded.",
    );
  });

  it("fails closed when static content claims executable capability", async () => {
    const invalidRelease = release({ allowedActions: ["write_local_event"] });
    mockInvoke.mockResolvedValueOnce([
      pack({ currentRelease: invalidRelease, releases: [invalidRelease] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack information could not be loaded.",
    );
  });

  it("fails closed when current review facts disagree with release history", async () => {
    mockInvoke.mockResolvedValueOnce([
      pack({
        currentRelease: release({ packVersion: "9.9.9" }),
        releases: [release()],
      }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack information could not be loaded.",
    );
  });

  it("fails closed when lifecycle flags contradict the pack state", async () => {
    const stagedRelease = release({ isActive: false, state: "staged" });
    mockInvoke.mockResolvedValueOnce([
      pack({ currentRelease: stagedRelease, releases: [stagedRelease] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toBeInTheDocument();
  });

  it("keeps load errors local and retries on request", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockRejectedValueOnce(new Error("raw database detail"))
      .mockResolvedValueOnce([]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(
      await screen.findByRole("alert"),
    ).toHaveTextContent("Pack information could not be loaded.");
    expect(screen.queryByText("raw database detail")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Try again" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
    expect(await screen.findByText("No packs are installed.")).toBeInTheDocument();
  });
});
