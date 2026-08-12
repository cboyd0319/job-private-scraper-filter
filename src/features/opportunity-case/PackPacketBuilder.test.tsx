/** Proves exact Packet Builder preview, approval, execution, and local-only output. */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { safeInvoke } from "../../platform/tauri";
import { pack, release } from "../settings/packs/packManagementTestData";
import { PackPacketBuilder } from "./PackPacketBuilder";

vi.mock("../../platform/tauri", () => ({ safeInvoke: vi.fn() }));

const mockSafeInvoke = vi.mocked(safeInvoke);
const packetRelease = release({
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
const packetPack = pack({
  publisherKeyId: "jobsentinel-packet-builder-publisher-v1",
  packId: "jobsentinel.agent.packet-builder",
  currentRelease: packetRelease,
  releases: [packetRelease],
});
const packet = {
  jobTitle: "Care Coordinator",
  company: "Example Health",
  resumeName: "Healthcare resume",
  evidenceReview: { requirements: [{ requirement: "Patient scheduling" }] },
  reviewedClaims: [
    { claim_id: "a".repeat(64), reviewed_text: "Reviewed scheduling claim" },
  ],
  attachmentChecklist: [
    "Confirm the selected resume",
    "Review every attachment",
  ],
  questionsToReview: ["Confirm current scheduling experience"],
  screeningAnswersRequireManualReview: true,
  finalSubmissionByUser: true,
};
const task = {
  runId: "pack-task:550e8400-e29b-41d4-a716-446655440010",
  approvalReference: "pack-task-approval:550e8400-e29b-41d4-a716-446655440011",
  taskKind: "draft_packet",
  plan: [
    { action: "read_selected_case_file", label: "Read selected case" },
    {
      action: "read_selected_resume_evidence",
      label: "Read selected resume evidence",
    },
    { action: "read_public_job_posting", label: "Read saved public posting" },
    {
      action: "create_draft_application_packet",
      label: "Create local draft packet",
    },
    { action: "write_local_event", label: "Write local audit event" },
  ],
  privacyLabels: ["local_only", "sensitive"],
  dataCategories: ["public_job_posting", "resume_evidence"],
  maxDurationSeconds: 30,
  maxOutputBytes: 262144,
};

describe("PackPacketBuilder", () => {
  beforeEach(() => mockSafeInvoke.mockReset());

  it("previews the exact draft before approval and returns only a local packet", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([packetPack]);
      if (command === "get_active_resume")
        return Promise.resolve({ id: 7, name: "Healthcare resume" });
      if (command === "prepare_packet_builder")
        return Promise.resolve({ task, packet });
      if (command === "execute_packet_builder") {
        return Promise.resolve({
          packet,
          receiptId: "pack-task-receipt:550e8400-e29b-41d4-a716-446655440012",
        });
      }
      return Promise.resolve(null);
    });

    render(<PackPacketBuilder jobHash="saved-job-a" />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Packet Builder" }),
    );

    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "prepare_packet_builder",
      {
        publisherKeyId: packetPack.publisherKeyId,
        packId: packetPack.packId,
        expectedGeneration: packetPack.generation,
        jobHash: "saved-job-a",
        resumeId: 7,
      },
      expect.objectContaining({
        logContext: "Prepare Packet Builder pack task",
      }),
    );
    expect(screen.getByText("Reviewed scheduling claim")).toBeInTheDocument();
    expect(screen.getByText("Patient scheduling")).toBeInTheDocument();
    expect(
      screen.getByText("Confirm current scheduling experience"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Nothing is sent or submitted/),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("opaque-run");
    expect(screen.getByLabelText("Packet Builder approval plan")).toHaveFocus();

    await user.click(
      screen.getByRole("button", { name: "Approve local packet" }),
    );

    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "execute_packet_builder",
      {
        runId: task.runId,
        approvalReference: task.approvalReference,
        jobHash: "saved-job-a",
        resumeId: 7,
      },
      expect.objectContaining({ logContext: "Run Packet Builder pack task" }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Local draft packet is ready",
    );
    expect(
      screen.getByText(/Final submission stays with you\./),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("pack-task-receipt");
  });

  it("cancels an unapproved packet when the opportunity changes", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([packetPack]);
      if (command === "get_active_resume") return Promise.resolve({ id: 7 });
      if (command === "prepare_packet_builder")
        return Promise.resolve({ task, packet });
      return Promise.resolve(null);
    });

    const { rerender } = render(<PackPacketBuilder jobHash="saved-job-a" />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Packet Builder" }),
    );
    await screen.findByRole("button", { name: "Approve local packet" });
    rerender(<PackPacketBuilder jobHash="saved-job-b" />);

    await waitFor(() => {
      expect(mockSafeInvoke).toHaveBeenCalledWith(
        "cancel_reviewed_pack_task",
        { runId: task.runId },
        expect.objectContaining({
          logContext: "Cancel Packet Builder pack task",
        }),
      );
    });
  });

  it("rejects a packet plan with a non-UUID approval identity", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([packetPack]);
      if (command === "get_active_resume") return Promise.resolve({ id: 7 });
      if (command === "prepare_packet_builder")
        return Promise.resolve({
          task: { ...task, approvalReference: "pack-task-approval:not-a-uuid" },
          packet,
        });
      return Promise.resolve(null);
    });

    render(<PackPacketBuilder jobHash="saved-job-a" />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Packet Builder" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Packet Builder could not prepare a current local draft.",
    );
    expect(
      screen.queryByRole("button", { name: "Approve local packet" }),
    ).not.toBeInTheDocument();
  });

  it("does not report packet completion without an exact durable receipt", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([packetPack]);
      if (command === "get_active_resume") return Promise.resolve({ id: 7 });
      if (command === "prepare_packet_builder")
        return Promise.resolve({ task, packet });
      if (command === "execute_packet_builder")
        return Promise.resolve({ packet });
      return Promise.resolve(null);
    });

    render(<PackPacketBuilder jobHash="saved-job-a" />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Packet Builder" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Approve local packet" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "stopped safely",
    );
    expect(
      screen.queryByText("Local draft packet is ready."),
    ).not.toBeInTheDocument();
  });

  it("shows the active-resume prerequisite instead of silently disappearing", async () => {
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([packetPack]);
      if (command === "get_active_resume") return Promise.resolve(null);
      return Promise.resolve(null);
    });

    render(<PackPacketBuilder jobHash="saved-job-a" />);

    expect(
      await screen.findByText(
        "Choose an active saved resume before preparing a packet.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Prepare Packet Builder" }),
    ).not.toBeInTheDocument();
  });
});
