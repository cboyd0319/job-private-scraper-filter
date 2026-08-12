import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import {
  mockSafeInvoke,
  resetResumeLibraryMocks,
} from "./ResumeLibraryPage.testSupport";
import { MilitaryTransitionReviewModal } from "./MilitaryTransitionReviewModal";

const match = {
  id: 10,
  resume_id: 1,
  job_hash: "saved-match-a",
  job_title: "Network Support Specialist",
  company: "Example",
  overall_match_score: 0.8,
  matching_skills: [],
  missing_skills: [],
  gap_analysis: null,
  feedback: null,
  created_at: "2026-07-21T00:00:00Z",
};

function renderModal(
  overrides: Partial<{ match: typeof match; isOpen: boolean }> = {},
) {
  return render(
    <MilitaryTransitionReviewModal
      isOpen={overrides.isOpen ?? true}
      match={overrides.match ?? match}
      onClose={() => undefined}
    />,
  );
}

describe("MilitaryTransitionReviewModal", () => {
  beforeEach(resetResumeLibraryMocks);

  it("requires review, keeps the token hidden, and renders only the safe confirmation projection", async () => {
    const user = userEvent.setup();
    const token = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    mockSafeInvoke.mockImplementation((command: string) => {
      if (command === "confirm_saved_match_military_evidence")
        return Promise.resolve(true);
      if (command === "prepare_saved_match_military_transition_review")
        return Promise.resolve(token);
      if (command === "confirm_saved_match_military_transition_review") {
        return Promise.resolve({
          civilian_role: "Network support specialist",
          civilian_responsibilities: [
            "Configured and supported network infrastructure",
          ],
          credential_wording: ["CompTIA Security+ certification"],
          user_confirmed_current_clearance: "Secret clearance",
          boundary: "suggestion_only",
          clearance_currentness: "not_verified",
          military_civilian_equivalence: "not_verified",
        });
      }
      return Promise.resolve(null);
    });

    renderModal();
    expect(screen.getByText(/manual review required/i)).toBeInTheDocument();
    expect(
      screen.getByText(/O\*NET and DoD COOL do not author or verify wording/i),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Confirm military service evidence" }),
    );
    await user.selectOptions(screen.getByLabelText("Military branch"), "army");
    await user.type(screen.getByLabelText("Occupation code"), "25B");
    await user.type(
      screen.getByLabelText("Proposed civilian role"),
      "Network support specialist",
    );
    await user.type(
      screen.getByLabelText("Optional user-confirmed current clearance"),
      "Secret clearance",
    );
    await user.click(
      screen.getByRole("button", {
        name: "Confirm current clearance evidence",
      }),
    );
    await user.click(
      screen.getByRole("button", { name: "Add responsibility mapping" }),
    );
    await user.type(
      screen.getByLabelText("Responsibility source phrase 1"),
      "Configured tactical networks",
    );
    await user.type(
      screen.getByLabelText("Responsibility civilian wording 1"),
      "Configured and supported network infrastructure",
    );
    await user.click(
      screen.getByRole("button", { name: "Add credential mapping" }),
    );
    await user.type(
      screen.getByLabelText("Credential source phrase 1"),
      "CompTIA Security+",
    );
    await user.type(
      screen.getByLabelText("Credential civilian wording 1"),
      "CompTIA Security+ certification",
    );
    expect(
      screen.getByRole("button", { name: "Prepare final review" }),
    ).toBeEnabled();
    await user.click(
      screen.getByRole("button", { name: "Prepare final review" }),
    );

    await waitFor(() => {
      expect(mockSafeInvoke).toHaveBeenCalledWith(
        "prepare_saved_match_military_transition_review",
        expect.any(Object),
        expect.any(Object),
      );
    });
    await waitFor(() => {
      expect(
        screen.getByText("Configured tactical networks"),
      ).toBeInTheDocument();
    });
    expect(
      screen.getByText("Configured and supported network infrastructure"),
    ).toBeInTheDocument();
    expect(screen.getByText("CompTIA Security+")).toBeInTheDocument();
    expect(
      screen.getByText("CompTIA Security+ certification"),
    ).toBeInTheDocument();
    expect(screen.queryByText(token)).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Confirm suggestion" }),
    );

    await waitFor(() => {
      expect(
        screen.getByText("Network support specialist"),
      ).toBeInTheDocument();
    });
    expect(
      screen.queryByText("Configured tactical networks"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Suggestion only. Clearance currentness and military/civilian equivalence are not verified.",
      ),
    ).toBeInTheDocument();
    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "confirm_saved_match_military_transition_review",
      { token },
      expect.objectContaining({
        logContext: "Confirm military transition review",
      }),
    );
  });

  it("discards a late preparation response after the user switches saved matches", async () => {
    const user = userEvent.setup();
    let resolvePreparation: ((token: string) => void) | undefined;
    mockSafeInvoke.mockImplementation((command: string) => {
      if (command === "confirm_saved_match_military_evidence")
        return Promise.resolve(true);
      if (command === "prepare_saved_match_military_transition_review") {
        return new Promise<string>((resolve) => {
          resolvePreparation = resolve;
        });
      }
      return Promise.resolve(null);
    });

    const view = renderModal();
    await user.click(
      screen.getByRole("button", { name: "Confirm military service evidence" }),
    );
    await user.selectOptions(screen.getByLabelText("Military branch"), "army");
    await user.type(screen.getByLabelText("Occupation code"), "25B");
    await user.type(
      screen.getByLabelText("Proposed civilian role"),
      "Network support specialist",
    );
    await user.click(
      screen.getByRole("button", { name: "Prepare final review" }),
    );
    view.rerender(
      <MilitaryTransitionReviewModal
        isOpen
        match={{ ...match, id: 11, job_hash: "saved-match-b" }}
        onClose={() => undefined}
      />,
    );
    resolvePreparation?.("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb");

    await waitFor(() => {
      expect(screen.getByLabelText("Occupation code")).toHaveValue("");
    });
    expect(
      screen.queryByRole("button", { name: "Confirm suggestion" }),
    ).not.toBeInTheDocument();
  });

  it("freezes same-match wording while a preparation request is pending", async () => {
    const user = userEvent.setup();
    let resolvePreparation: ((token: string) => void) | undefined;
    mockSafeInvoke.mockImplementation((command: string) => {
      if (command === "confirm_saved_match_military_evidence") {
        return Promise.resolve(true);
      }
      if (command === "prepare_saved_match_military_transition_review") {
        return new Promise<string>((resolve) => {
          resolvePreparation = resolve;
        });
      }
      return Promise.resolve(null);
    });

    renderModal();
    await user.click(
      screen.getByRole("button", { name: "Confirm military service evidence" }),
    );
    await user.selectOptions(screen.getByLabelText("Military branch"), "army");
    await user.type(screen.getByLabelText("Occupation code"), "25B");
    const roleInput = screen.getByLabelText("Proposed civilian role");
    await user.type(roleInput, "Network support specialist");
    await user.click(
      screen.getByRole("button", { name: "Prepare final review" }),
    );

    expect(roleInput).toBeDisabled();
    await user.type(roleInput, " changed");
    expect(roleInput).toHaveValue("Network support specialist");
    resolvePreparation?.("cccccccc-cccc-4ccc-8ccc-cccccccccccc");

    expect(
      await screen.findByText(
        "Proposed civilian role: Network support specialist",
      ),
    ).toBeInTheDocument();
  });

  it("shows an action error without treating failed evidence as confirmed", async () => {
    const user = userEvent.setup();
    mockSafeInvoke.mockRejectedValue(new Error("unavailable"));
    renderModal();

    await user.click(
      screen.getByRole("button", { name: "Confirm military service evidence" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not confirm this evidence",
    );
    expect(
      screen.getByRole("button", { name: "Confirm military service evidence" }),
    ).toBeEnabled();
  });
});
