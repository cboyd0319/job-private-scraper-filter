import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  makeResumeSummary,
  mockSafeInvoke,
  resetResumeLibraryMocks,
} from "./ResumeLibraryPage.testSupport";
import { MatchDebuggerModal } from "./MatchDebuggerModal";
import ResumeLibraryPage from "./ResumeLibraryPage";

type DebuggerView = {
  debugger_id: string;
  requirements: Array<{
    requirement: string;
    importance: string;
    match_state: string;
    hard_constraint: boolean;
    evidence: Array<{ evidence_id: string; confirmed: boolean }>;
    why_not: string | null;
    blocking: boolean;
  }>;
};

const matches = [
  {
    id: 10,
    resume_id: 1,
    job_hash: "saved-match-a",
    job_title: "Care Coordinator",
    company: "Community Health Partners",
    overall_match_score: 0.82,
    matching_skills: [],
    missing_skills: [],
    gap_analysis: null,
    feedback: null,
    created_at: "2026-05-21T12:00:00Z",
  },
  {
    id: 11,
    resume_id: 1,
    job_hash: "saved-match-b",
    job_title: "Support Lead",
    company: "Community Health Partners",
    overall_match_score: 0.75,
    matching_skills: [],
    missing_skills: [],
    gap_analysis: null,
    feedback: null,
    created_at: "2026-05-21T12:00:00Z",
  },
];

function configurePage(
  debuggerResponse: (jobHash: string) => DebuggerView | Promise<DebuggerView>,
  handlers: {
    confirmEvidence?: () => Promise<unknown>;
    savePacketClaim?: () => Promise<unknown>;
  } = {},
) {
  const packetClaims: Array<{ claim_id: string; reviewed_text: string }> = [];
  mockSafeInvoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "get_saved_match_debugger") {
      return debuggerResponse(String(args?.jobHash));
    }
    if (command === "get_active_resume") return Promise.resolve(makeResumeSummary());
    if (command === "get_recent_matches") return Promise.resolve(matches);
    if (command === "list_all_resumes" || command === "get_user_skills") return Promise.resolve([]);
    if (command === "list_saved_match_evidence_packets") return Promise.resolve(packetClaims);
    if (command === "confirm_saved_match_evidence" && handlers.confirmEvidence) {
      return handlers.confirmEvidence();
    }
    if (command === "save_saved_match_evidence_packet") {
      if (handlers.savePacketClaim) return handlers.savePacketClaim();
      const claim = {
        claim_id: "f".repeat(64),
        reviewed_text: String(args?.reviewedText),
      };
      packetClaims.push(claim);
      return Promise.resolve(claim);
    }
    return Promise.resolve(null);
  });
}

describe("Resume page match debugger", () => {
  beforeEach(resetResumeLibraryMocks);

  it("confirms reviewed evidence without rendering opaque references", async () => {
    const user = userEvent.setup();
    const debuggerId = "a".repeat(64);
    const evidenceId = "b".repeat(64);
    const confirmedEvidenceId = "c".repeat(64);
    const pending: DebuggerView = {
      debugger_id: debuggerId,
      requirements: [{
        requirement: "Patient scheduling",
        importance: "Required",
        match_state: "Partial",
        hard_constraint: true,
        evidence: [
          { evidence_id: evidenceId, confirmed: false },
          { evidence_id: confirmedEvidenceId, confirmed: true },
        ],
        why_not: "partial_evidence",
        blocking: true,
      }],
    };
    const confirmed: DebuggerView = {
      ...pending,
      requirements: [{
        ...pending.requirements[0],
        evidence: pending.requirements[0]?.evidence.map((evidence) => ({
          ...evidence,
          confirmed: true,
        })) ?? [],
      }],
    };
    let loads = 0;
    configurePage(() => Promise.resolve(loads++ === 0 ? pending : confirmed));

    render(<ResumeLibraryPage onBack={vi.fn()} />);
    await user.click((await screen.findAllByRole("button", { name: "Inspect evidence" }))[0]);

    expect(await screen.findByRole("dialog", { name: "Match Debugger" })).toBeInTheDocument();
    expect(screen.getByText("Patient scheduling")).toBeInTheDocument();
    expect(screen.getByText("Hard blocker")).toBeInTheDocument();
    expect(screen.getByText("Why not: partial evidence")).toBeInTheDocument();
    expect(screen.getByText("Evidence 1")).toBeInTheDocument();
    expect(screen.getByText("Evidence 2")).toBeInTheDocument();
    expect(screen.getByText("Confirmed")).toBeInTheDocument();
    expect(screen.getByText("No reviewed claims saved yet.")).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(debuggerId);
    expect(document.body).not.toHaveTextContent(evidenceId);
    expect(document.body).not.toHaveTextContent(confirmedEvidenceId);

    await user.click(screen.getByRole("button", { name: "Confirm evidence" }));

    await waitFor(() => {
      expect(mockSafeInvoke).toHaveBeenCalledWith(
        "confirm_saved_match_evidence",
        { jobHash: "saved-match-a", resumeId: 1, debuggerId, evidenceId },
        expect.objectContaining({ logContext: "Confirm saved match evidence" }),
      );
    });
    expect(await screen.findAllByText("Confirmed")).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Confirm evidence" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("checkbox", { name: "Select Evidence 1 for Patient scheduling reviewed claim" }));
    await user.type(screen.getByLabelText("Reviewed claim"), "I reviewed the scheduling evidence.");
    expect(screen.getByText("Clearance currentness is not verified. Military/civilian equivalence is not verified.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save reviewed claim" }));
    await waitFor(() => {
      expect(mockSafeInvoke).toHaveBeenCalledWith(
        "save_saved_match_evidence_packet",
        {
          jobHash: "saved-match-a",
          resumeId: 1,
          reviewedText: "I reviewed the scheduling evidence.",
          evidenceIds: [evidenceId],
        },
        expect.objectContaining({ logContext: "Save saved match reviewed claim" }),
      );
    });
    expect(await screen.findByText("I reviewed the scheduling evidence.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.click((await screen.findAllByRole("button", { name: "Inspect evidence" }))[0]);
    expect(await screen.findAllByText("Confirmed")).toHaveLength(2);
  });

  it("keeps a newer saved match visible when an older debugger request resolves last", async () => {
    const user = userEvent.setup();
    const resolvers: Array<(value: DebuggerView) => void> = [];
    configurePage(() => new Promise((resolve) => resolvers.push(resolve)));

    render(<ResumeLibraryPage onBack={vi.fn()} />);
    const inspect = await screen.findAllByRole("button", { name: "Inspect evidence" });
    await user.click(inspect[0]);
    await waitFor(() => expect(resolvers).toHaveLength(1));
    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.click((await screen.findAllByRole("button", { name: "Inspect evidence" }))[1]);
    await waitFor(() => expect(resolvers).toHaveLength(2));

    resolvers[1]?.({
      debugger_id: "d".repeat(64),
      requirements: [{
        requirement: "Second requirement",
        importance: "Required",
        match_state: "Direct",
        hard_constraint: false,
        evidence: [],
        why_not: null,
        blocking: false,
      }],
    });
    expect(await screen.findByText("Second requirement")).toBeInTheDocument();

    resolvers[0]?.({
      debugger_id: "e".repeat(64),
      requirements: [{
        requirement: "First requirement",
        importance: "Required",
        match_state: "Direct",
        hard_constraint: false,
        evidence: [],
        why_not: null,
        blocking: false,
      }],
    });
    await waitFor(() => {
      expect(screen.getByText("Second requirement")).toBeInTheDocument();
    });
    expect(screen.queryByText("First requirement")).not.toBeInTheDocument();
  });

  it("does not let an older evidence confirmation reload a newer saved match", async () => {
    const user = userEvent.setup();
    let resolveConfirmation: (() => void) | undefined;
    const evidenceId = "b".repeat(64);
    configurePage(
      (jobHash) => ({
        debugger_id: jobHash === "saved-match-a" ? "a".repeat(64) : "d".repeat(64),
        requirements: [{
          requirement: jobHash === "saved-match-a" ? "First requirement" : "Second requirement",
          importance: "Required",
          match_state: "Direct",
          hard_constraint: false,
          evidence: [{ evidence_id: evidenceId, confirmed: jobHash === "saved-match-b" }],
          why_not: null,
          blocking: false,
        }],
      }),
      { confirmEvidence: () => new Promise((resolve) => { resolveConfirmation = () => resolve(true); }) },
    );

    const { rerender } = render(<MatchDebuggerModal isOpen match={matches[0]} onClose={vi.fn()} />);
    await screen.findByText("First requirement");
    await user.click(await screen.findByRole("button", { name: "Confirm evidence" }));
    rerender(<MatchDebuggerModal isOpen match={matches[1]} onClose={vi.fn()} />);
    const secondCheckbox = await screen.findByRole("checkbox", {
      name: "Select Evidence 1 for Second requirement reviewed claim",
    });
    await user.click(secondCheckbox);
    await user.type(screen.getByLabelText("Reviewed claim"), "Keep this newer draft.");

    resolveConfirmation?.();

    await waitFor(() => {
      expect(screen.getByText("Second requirement")).toBeInTheDocument();
      expect(screen.getByLabelText("Reviewed claim")).toHaveValue("Keep this newer draft.");
      expect(secondCheckbox).toBeChecked();
    });
    expect(screen.queryByText("First requirement")).not.toBeInTheDocument();
  });

  it("does not let an older packet save clear a newer saved match draft", async () => {
    const user = userEvent.setup();
    let resolveSave: (() => void) | undefined;
    const evidenceId = "b".repeat(64);
    configurePage(
      (jobHash) => ({
        debugger_id: jobHash === "saved-match-a" ? "a".repeat(64) : "d".repeat(64),
        requirements: [{
          requirement: jobHash === "saved-match-a" ? "First requirement" : "Second requirement",
          importance: "Required",
          match_state: "Direct",
          hard_constraint: false,
          evidence: [{ evidence_id: evidenceId, confirmed: true }],
          why_not: null,
          blocking: false,
        }],
      }),
      { savePacketClaim: () => new Promise((resolve) => { resolveSave = () => resolve({ claim_id: "f".repeat(64) }); }) },
    );

    const { rerender } = render(<MatchDebuggerModal isOpen match={matches[0]} onClose={vi.fn()} />);
    await screen.findByText("First requirement");
    await user.click(await screen.findByRole("checkbox", {
      name: "Select Evidence 1 for First requirement reviewed claim",
    }));
    await user.type(screen.getByLabelText("Reviewed claim"), "Older draft.");
    await user.click(screen.getByRole("button", { name: "Save reviewed claim" }));
    rerender(<MatchDebuggerModal isOpen match={matches[1]} onClose={vi.fn()} />);
    const secondCheckbox = await screen.findByRole("checkbox", {
      name: "Select Evidence 1 for Second requirement reviewed claim",
    });
    await user.click(secondCheckbox);
    await user.type(screen.getByLabelText("Reviewed claim"), "Keep this packet draft.");

    resolveSave?.();

    await waitFor(() => {
      expect(screen.getByText("Second requirement")).toBeInTheDocument();
      expect(screen.getByLabelText("Reviewed claim")).toHaveValue("Keep this packet draft.");
      expect(secondCheckbox).toBeChecked();
    });
    expect(screen.queryByText("Older draft.")).not.toBeInTheDocument();
  });
});
