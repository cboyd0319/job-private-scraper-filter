/** Verifies pack management loading, lifecycle review, and safe failure states. */

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "../../../platform/tauri";
import { SettingsHelpStatusSection } from "../support/SettingsSupportSections";
import { PackManagementModal } from "./PackManagementModal";
import { pack, release } from "./packManagementTestData";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../support/ErrorLogPanel", () => ({
  ErrorLogPanel: () => <div />,
}));

const mockInvoke = vi.mocked(invoke);

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

  it("discards an older refresh result after newer pack truth arrives", async () => {
    const user = userEvent.setup();
    let resolveOlder!: (value: unknown) => void;
    const older = new Promise<unknown>((resolve) => {
      resolveOlder = resolve;
    });
    mockInvoke
      .mockResolvedValueOnce([pack()])
      .mockImplementationOnce(() => older)
      .mockResolvedValueOnce([
        pack({ state: "disabled", generation: 5 }),
      ]);

    render(<PackManagementModal onClose={vi.fn()} />);
    expect(await screen.findByLabelText("Pack status: Ready")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Refresh" }));
    await user.click(screen.getByRole("button", { name: "Refresh" }));
    expect(
      await screen.findByLabelText("Pack status: Disabled"),
    ).toBeInTheDocument();

    await act(async () => {
      resolveOlder([pack()]);
      await older;
    });
    expect(screen.getByLabelText("Pack status: Disabled")).toBeInTheDocument();
  });

  it("loads only after the user opens the pack view from settings", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce([]);

    render(<SettingsHelpStatusSection onShowHealthDashboard={vi.fn()} />);

    expect(mockInvoke).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "View Packs" }));

    expect(await screen.findByRole("dialog", { name: "Packs" })).toBeInTheDocument();
    expect(screen.getByText(/Pack files stay local/)).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("list_pack_management");
  });

  it("shows review facts, updates, quarantine failures, cleanup, and history", async () => {
    mockInvoke.mockResolvedValueOnce([
      pack({
        cleanupPending: true,
        updateAvailable: true,
        currentRelease: release({
          isHighWater: false,
          releaseSequence: 1,
          packVersion: "3.0.1",
        }),
        releases: [
          release({
            releaseSequence: 1,
            packVersion: "3.0.1",
            isHighWater: false,
          }),
          release({
            artifactCleanupPending: true,
            isActive: false,
            isHighWater: false,
            releaseSequence: 2,
            quarantineReason: "self_test_failed",
            state: "quarantined",
            lastSelfTestedAt: null,
          }),
          release({
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
      within(currentReview).getByText("Static Content"),
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
    ).toHaveTextContent("Tasks: No runnable tasks");
    expect(
      within(currentReview).getByText(/Approval:/),
    ).toHaveTextContent("Approval: No execution approval gate");
    expect(within(currentReview).getByText("What it helps with")).toBeInTheDocument();
    expect(
      within(currentReview).getByText("Adds reviewed job-source support."),
    ).toBeInTheDocument();
    expect(within(currentReview).getByText("Model download")).toBeInTheDocument();
    expect(within(currentReview).getByText("Not required")).toBeInTheDocument();
    expect(
      within(currentReview).getByText("Last successful self-test"),
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
    expect(
      within(quarantinedPack).getAllByText("Last successful self-test").length,
    ).toBeGreaterThan(0);

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
      "No actions",
    );
    expect(within(update).getByText(/Gateway policy:/)).toHaveTextContent(
      "Not used",
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
    if (state === "removed") {
      expect(screen.getAllByText("Last successful self-test").length).toBeGreaterThan(0);
    }
  });

  it("uses the signed task kind to identify a packet-builder agent", async () => {
    const packetAgent = release({
      packType: "agent",
      executionClass: "reviewed_typed_workflow",
      purpose: "draft_application_packet",
      privacyLabels: ["local_only", "sensitive"],
      allowedDataCategories: ["public_job_posting", "resume_evidence"],
      allowedTaskKinds: ["draft_packet"],
      allowedActions: [
        "read_selected_case_file",
        "read_selected_resume_evidence",
        "read_public_job_posting",
        "create_draft_application_packet",
        "write_local_event",
      ],
      approvalGates: ["per_execution_review"],
    });
    mockInvoke.mockResolvedValueOnce([
      pack({ currentRelease: packetAgent, releases: [packetAgent] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    const currentReview = await screen.findByRole("region", {
      name: "Current release review",
    });
    expect(
      within(currentReview).getByText("Builds a draft application packet."),
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

  it.each([
    [
      "a source claims reviewed workflow authority",
      {
        executionClass: "reviewed_typed_workflow",
        allowedDataCategories: ["public_job_posting"],
        allowedTaskKinds: ["source_check"],
        allowedActions: ["request_source_check"],
        approvalGates: ["per_execution_review"],
      },
    ],
    [
      "a packet workflow omits its compiled action plan",
      {
        packType: "agent",
        executionClass: "reviewed_typed_workflow",
        purpose: "draft_application_packet",
        privacyLabels: ["local_only", "sensitive"],
        allowedDataCategories: ["public_job_posting", "resume_evidence"],
        allowedTaskKinds: ["draft_packet"],
        allowedActions: ["create_draft_application_packet"],
        approvalGates: ["per_execution_review"],
      },
    ],
  ])("fails closed when %s", async (_description, invalidFacts) => {
    const invalidRelease = release(invalidFacts);
    mockInvoke.mockResolvedValueOnce([
      pack({ currentRelease: invalidRelease, releases: [invalidRelease] }),
    ]);

    render(<PackManagementModal onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack information could not be loaded.",
    );
  });

  it.each([
    { purpose: "resume_evidence_review" },
    { modelDownloadRequired: true },
    { lastSelfTestedAt: "not-a-date" },
    { lastSelfTestedAt: null },
  ])("fails closed when a Pack UI fact is invalid: %j", async (invalidFact) => {
    const invalidRelease = release(invalidFact);
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
