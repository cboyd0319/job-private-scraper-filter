/** Proves explicit reviewed Evidence Reviewer preparation, execution, and cancellation. */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { safeInvoke } from "../../../platform/tauri";
import { pack, release } from "../../settings/packs/packManagementTestData";
import { PackEvidenceReviewer } from "./PackEvidenceReviewer";

vi.mock("../../../platform/tauri", () => ({ safeInvoke: vi.fn() }));

const mockSafeInvoke = vi.mocked(safeInvoke);
const match = { job_hash: "saved-match-a", resume_id: 1 };
const runId = "pack-task:550e8400-e29b-41d4-a716-446655440000";
const approvalReference =
  "pack-task-approval:550e8400-e29b-41d4-a716-446655440001";
const pendingRunId = "pack-task:550e8400-e29b-41d4-a716-446655440002";
const pendingApproval =
  "pack-task-approval:550e8400-e29b-41d4-a716-446655440003";
const reviewerRelease = release({
  packType: "agent",
  executionClass: "reviewed_typed_workflow",
  purpose: "resume_evidence_review",
  privacyLabels: ["local_only", "sensitive"],
  allowedDataCategories: ["resume_evidence"],
  allowedTaskKinds: ["evidence_review"],
  allowedActions: ["read_selected_resume_evidence", "write_local_event"],
  approvalGates: ["per_execution_review"],
});
const reviewerPack = pack({
  publisherKeyId: "jobsentinel-evidence-reviewer-publisher-v1",
  packId: "jobsentinel.agent.evidence-reviewer",
  currentRelease: reviewerRelease,
  releases: [reviewerRelease],
});

describe("PackEvidenceReviewer", () => {
  beforeEach(() => mockSafeInvoke.mockReset());

  it("shows the reviewed plan before one exact local execution", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([reviewerPack]);
      if (command === "prepare_evidence_reviewer") {
        return Promise.resolve({
          runId,
          approvalReference,
          taskKind: "evidence_review",
          plan: [
            {
              action: "read_selected_resume_evidence",
              label: "Read selected evidence",
            },
            { action: "write_local_event", label: "Write local audit event" },
          ],
          failureMessage: "Evidence review failed safely.",
          privacyLabels: ["local_only", "sensitive"],
          dataCategories: ["resume_evidence"],
          maxDurationSeconds: 10,
          maxOutputBytes: 32768,
        });
      }
      if (command === "execute_evidence_reviewer") {
        return Promise.resolve({
          review: {
            debugger_id: "a".repeat(64),
            requirements: [{ requirement: "Patient scheduling" }],
          },
          receiptId: "pack-task-receipt:550e8400-e29b-41d4-a716-446655440004",
        });
      }
      return Promise.resolve(null);
    });

    render(<PackEvidenceReviewer match={match} />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Evidence Reviewer" }),
    );

    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "prepare_evidence_reviewer",
      {
        publisherKeyId: reviewerPack.publisherKeyId,
        packId: reviewerPack.packId,
        expectedGeneration: reviewerPack.generation,
        jobHash: match.job_hash,
        resumeId: match.resume_id,
      },
      expect.objectContaining({
        logContext: "Prepare Evidence Reviewer pack task",
      }),
    );
    expect(screen.getByText("Read selected evidence")).toBeInTheDocument();
    expect(screen.getByText("Write local audit event")).toBeInTheDocument();
    expect(screen.getByText(/Local only, Sensitive/)).toBeInTheDocument();
    expect(
      screen.getByLabelText("Evidence Reviewer approval plan"),
    ).toHaveFocus();
    expect(document.body).not.toHaveTextContent("opaque-run");
    expect(document.body).not.toHaveTextContent("opaque-review");

    await user.click(
      screen.getByRole("button", { name: "Run Evidence Reviewer" }),
    );

    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "execute_evidence_reviewer",
      {
        runId,
        approvalReference,
        jobHash: match.job_hash,
        resumeId: match.resume_id,
      },
      expect.objectContaining({
        logContext: "Run Evidence Reviewer pack task",
      }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Evidence Reviewer completed locally for 1 requirement",
    );
    expect(document.body).not.toHaveTextContent("opaque");
  });

  it("cancels an unapproved task when the selected match changes", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([reviewerPack]);
      if (command === "prepare_evidence_reviewer") {
        return Promise.resolve({
          runId: pendingRunId,
          approvalReference: pendingApproval,
          taskKind: "evidence_review",
          plan: [
            {
              action: "read_selected_resume_evidence",
              label: "Read selected evidence",
            },
            { action: "write_local_event", label: "Write local audit event" },
          ],
          failureMessage: "Failed safely.",
          privacyLabels: ["local_only", "sensitive"],
          dataCategories: ["resume_evidence"],
          maxDurationSeconds: 10,
          maxOutputBytes: 32768,
        });
      }
      return Promise.resolve(null);
    });

    const { rerender } = render(<PackEvidenceReviewer match={match} />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Evidence Reviewer" }),
    );
    await screen.findByRole("button", { name: "Run Evidence Reviewer" });
    rerender(
      <PackEvidenceReviewer
        match={{ job_hash: "saved-match-b", resume_id: 2 }}
      />,
    );

    await waitFor(() => {
      expect(mockSafeInvoke).toHaveBeenCalledWith(
        "cancel_reviewed_pack_task",
        { runId: pendingRunId },
        expect.objectContaining({
          logContext: "Cancel Evidence Reviewer pack task",
        }),
      );
    });
  });

  it("rejects a prepared plan with a non-UUID task identity", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([reviewerPack]);
      if (command === "prepare_evidence_reviewer")
        return Promise.resolve({
          runId: "pack-task:not-a-uuid",
          approvalReference,
          taskKind: "evidence_review",
          plan: [
            {
              action: "read_selected_resume_evidence",
              label: "Read selected evidence",
            },
            { action: "write_local_event", label: "Write local audit event" },
          ],
          privacyLabels: ["local_only", "sensitive"],
          dataCategories: ["resume_evidence"],
          maxDurationSeconds: 10,
          maxOutputBytes: 32768,
        });
      return Promise.resolve(null);
    });

    render(<PackEvidenceReviewer match={match} />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Evidence Reviewer" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Evidence Reviewer could not prepare a current local review.",
    );
    expect(
      screen.queryByRole("button", { name: "Run Evidence Reviewer" }),
    ).not.toBeInTheDocument();
  });

  it("does not report success without an exact durable receipt", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockImplementation((command) => {
      if (command === "list_pack_management")
        return Promise.resolve([reviewerPack]);
      if (command === "prepare_evidence_reviewer")
        return Promise.resolve({
          runId,
          approvalReference,
          taskKind: "evidence_review",
          plan: [
            {
              action: "read_selected_resume_evidence",
              label: "Read selected evidence",
            },
            { action: "write_local_event", label: "Write local audit event" },
          ],
          privacyLabels: ["local_only", "sensitive"],
          dataCategories: ["resume_evidence"],
          maxDurationSeconds: 10,
          maxOutputBytes: 32768,
        });
      if (command === "execute_evidence_reviewer")
        return Promise.resolve({
          review: { requirements: [{ requirement: "Scheduling" }] },
        });
      return Promise.resolve(null);
    });

    render(<PackEvidenceReviewer match={match} />);
    await user.click(
      await screen.findByRole("button", { name: "Prepare Evidence Reviewer" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Run Evidence Reviewer" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "stopped safely",
    );
    expect(screen.queryByText(/completed locally/)).not.toBeInTheDocument();
  });

  it("keeps pack availability failures visible and retryable", async () => {
    mockSafeInvoke.mockRejectedValueOnce(new Error("private detail"));

    render(<PackEvidenceReviewer match={match} />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Evidence Reviewer availability could not be checked.",
    );
    expect(
      screen.getByRole("button", { name: "Retry availability" }),
    ).toBeEnabled();
    expect(document.body).not.toHaveTextContent("private detail");
  });
});
