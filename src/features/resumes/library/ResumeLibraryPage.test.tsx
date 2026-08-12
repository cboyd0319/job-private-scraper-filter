/** Verifies resume library rendering, feedback, and local match interactions. */

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  makeResumeSummary,
  mockResumeLibraryResponses,
  mockSafeInvoke,
  mockSafeInvokeWithToast,
  resetResumeLibraryMocks,
} from "./ResumeLibraryPage.testSupport";
import ResumeLibraryPage from "./ResumeLibraryPage";

function makeFeedbackMatch() {
  return {
    id: 10,
    resume_id: 1,
    job_hash: "job-feedback",
    job_title: "Care Coordinator",
    company: "Community Health Partners",
    overall_match_score: 0.82,
    skills_match_score: 0.75,
    experience_match_score: 0.5,
    education_match_score: 0.25,
    matching_skills: ["Scheduling"],
    missing_skills: ["Case Management"],
    gap_analysis: null,
    feedback: null,
    created_at: "2026-05-21T12:00:00Z",
  };
}

describe("Resume page", () => {
  beforeEach(resetResumeLibraryMocks);

  it("labels structured resume import as a resume-app file path", async () => {
    mockResumeLibraryResponses();

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("No Resume Added")).toBeInTheDocument();
    });

    expect(screen.getAllByRole("button", { name: "Import from resume app" })).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Import Resume Data" })).not.toBeInTheDocument();
    expect(screen.queryByText("No Resume Uploaded")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /upload resume/i })).not.toBeInTheDocument();
  });

  it("keeps resume action buttons stacked on narrow screens", async () => {
    mockResumeLibraryResponses();

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("No Resume Added")).toBeInTheDocument();
    });

    const headerAddButton = screen.getAllByRole("button", { name: "Add Resume" })[0];
    const emptyStateCard = screen.getByText("No Resume Added").closest(".rounded-card");
    expect(emptyStateCard).not.toBeNull();

    const emptyStateAddButton = within(emptyStateCard as HTMLElement).getByRole("button", {
      name: "Add Resume",
    });

    expect(headerAddButton.parentElement).toHaveClass("flex-col", "sm:flex-row");
    expect(headerAddButton).toHaveClass("w-full", "sm:w-auto");
    expect(emptyStateAddButton.parentElement).toHaveClass("flex-col", "sm:flex-row");
    expect(emptyStateAddButton).toHaveClass("w-full", "sm:w-auto");
  });

  it("renders resume match sub-scores as percentages from backend fractions", async () => {
    mockResumeLibraryResponses({
      get_active_resume: makeResumeSummary(),
      get_recent_matches: [
        {
          id: 10,
          resume_id: 1,
          job_hash: "job-hash",
          job_title: "Care Coordinator",
          company: "Community Health Partners",
          overall_match_score: 0.82,
          skills_match_score: 0.75,
          experience_match_score: 0.5,
          education_match_score: 0.25,
          matching_skills: ["Scheduling"],
          missing_skills: ["Case Management"],
          gap_analysis:
            "Why not: Score limited because skill evidence was not found: Case Management\nScoring sources: skill coverage, required coverage",
          created_at: "2026-05-21T12:00:00Z",
        },
      ],
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Care Coordinator")).toBeInTheDocument();
    });

    expect(screen.getByText("Recent Resume Matches")).toBeInTheDocument();
    expect(screen.getByText("Saved Skills (0)")).toBeInTheDocument();
    expect(screen.getByText("Fit Details")).toBeInTheDocument();
    expect(screen.getByText("Skills fit")).toBeInTheDocument();
    expect(screen.getByText("Experience fit")).toBeInTheDocument();
    expect(screen.getByText("Education fit")).toBeInTheDocument();
    expect(screen.getByText("Skills found in both (1)")).toBeInTheDocument();
    expect(screen.getByText("Skills to review (1)")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Why not: Score limited because skill evidence was not found: Case Management",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Scoring sources: skill coverage, required coverage"),
    ).toBeInTheDocument();
    expect(screen.getByText("75%")).toBeInTheDocument();
    expect(screen.getByText("50%")).toBeInTheDocument();
    expect(screen.getByText("25%")).toBeInTheDocument();
    expect(screen.getByText("No skills saved yet")).toBeInTheDocument();
    expect(
      screen.getByText("JobSentinel can suggest skills from a resume, or you can add them yourself."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/score breakdown/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/matched skills|missing skills/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/skills extracted|extract skills automatically/i)).not.toBeInTheDocument();
  });

  it("does not show invalid percentages when recent matches omit optional sub-scores", async () => {
    mockResumeLibraryResponses({
      get_active_resume: makeResumeSummary(),
      get_recent_matches: [
        {
          id: 10,
          resume_id: 1,
          job_hash: "job-hash",
          job_title: "Care Coordinator",
          company: "Community Health Partners",
          overall_match_score: 0.82,
          skills_match_score: 0.75,
          matching_skills: ["Scheduling"],
          missing_skills: ["Case Management"],
          gap_analysis: null,
          created_at: "2026-05-21T12:00:00Z",
        },
      ],
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Care Coordinator")).toBeInTheDocument();
    });

    expect(screen.getByText("Fit Details")).toBeInTheDocument();
    expect(screen.getByText("Skills fit")).toBeInTheDocument();
    expect(screen.getByText("75%")).toBeInTheDocument();
    expect(screen.queryByText("Experience fit")).not.toBeInTheDocument();
    expect(screen.queryByText("Education fit")).not.toBeInTheDocument();
    expect(screen.queryByText("NaN%")).not.toBeInTheDocument();
  });
  it("records and clears a local saved-match feedback label without sending match content", async () => {
    const user = userEvent.setup();
    mockResumeLibraryResponses({
      get_active_resume: makeResumeSummary(),
      get_recent_matches: [makeFeedbackMatch()],
    });
    mockSafeInvokeWithToast
      .mockResolvedValueOnce({
        match_id: 10,
        label: "useful",
        recorded_at: "2026-07-20T00:00:00Z",
      })
      .mockResolvedValueOnce(null);

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    const useful = await screen.findByRole("button", { name: "Useful" });
    expect(useful).toHaveAttribute("aria-pressed", "false");

    await user.click(useful);

    expect(mockSafeInvokeWithToast).toHaveBeenNthCalledWith(
      1,
      "set_resume_match_feedback",
      { matchId: 10, label: "useful" },
      expect.anything(),
      expect.objectContaining({ logContext: "Save resume match feedback" }),
    );
    expect(useful).toHaveAttribute("aria-pressed", "true");

    await user.click(useful);

    expect(mockSafeInvokeWithToast).toHaveBeenNthCalledWith(
      2,
      "set_resume_match_feedback",
      { matchId: 10, label: null },
      expect.anything(),
      expect.objectContaining({ logContext: "Clear resume match feedback" }),
    );
    expect(useful).toHaveAttribute("aria-pressed", "false");
    const sentArgs = mockSafeInvokeWithToast.mock.calls.map((call) => call[1]);
    expect(JSON.stringify(sentArgs)).not.toContain("Care Coordinator");
    expect(JSON.stringify(sentArgs)).not.toContain("Community Health Partners");
    expect(JSON.stringify(sentArgs)).not.toContain("Scheduling");
  });

  it("prevents another saved-match feedback action while one is pending", async () => {
    const user = userEvent.setup();
    const match = makeFeedbackMatch();
    mockResumeLibraryResponses({
      get_active_resume: makeResumeSummary(),
      get_recent_matches: [
        match,
        { ...match, id: 11, job_hash: "job-feedback-2", job_title: "Support Lead" },
      ],
    });
    let finishSave: (value: unknown) => void = () => {};
    mockSafeInvokeWithToast.mockReturnValueOnce(
      new Promise((resolve) => {
        finishSave = resolve;
      }),
    );

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    const useful = await screen.findAllByRole("button", { name: "Useful" });
    await user.click(useful[0]);

    expect(useful).toHaveLength(2);
    for (const button of [
      ...useful,
      ...screen.getAllByRole("button", { name: "Not relevant" }),
    ]) {
      expect(button).toBeDisabled();
    }

    finishSave({
      match_id: 10,
      label: "useful",
      recorded_at: "2026-07-20T00:00:00Z",
    });
    await waitFor(() => expect(useful[0]).toBeEnabled());
  });

  it("shows a local readable-text preview without exposing a resume path", async () => {
    const user = userEvent.setup();

    mockResumeLibraryResponses({
      get_active_resume: makeResumeSummary(),
      get_resume_text_preview: {
        resume_id: 1,
        name: "Care Coordinator Resume",
        has_text: true,
        text_preview: "Care coordinator\nPatient scheduling\nCase management",
        text_chars: 52,
        is_truncated: false,
      },
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Care Coordinator Resume")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "See what JobSentinel read" }));

    expect(mockSafeInvoke).toHaveBeenCalledWith(
      "get_resume_text_preview",
      { resumeId: 1 },
      { logContext: "Preview resume text" },
    );
    expect(await screen.findByText("Readable Resume Text")).toBeInTheDocument();
    expect(screen.getByText("Text found")).toBeInTheDocument();
    expect(screen.getByText("Employer format first")).toBeInTheDocument();
    expect(screen.getByText("Important details need text")).toBeInTheDocument();
    expect(screen.getByText("Preview complete")).toBeInTheDocument();
    expect(screen.getByText(/Patient scheduling/)).toBeInTheDocument();
    expect(screen.queryByText(/resume=private-file/)).not.toBeInTheDocument();
    expect(screen.queryByText(/file_path/)).not.toBeInTheDocument();
  });

  it("shows sanitized resume format and readable-text status before review", async () => {
    const resume = makeResumeSummary({
      format_label: "PDF",
      has_readable_text: true,
      readable_text_chars: 1234,
    });
    mockResumeLibraryResponses({
      get_active_resume: resume,
      list_all_resumes: [resume],
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Care Coordinator Resume")).toBeInTheDocument();
    });

    expect(screen.getByText("PDF")).toBeInTheDocument();
    expect(screen.getByText("Readable text ready")).toBeInTheDocument();
    expect(screen.getByText("1,234 characters available for local review.")).toBeInTheDocument();
    expect(screen.queryByText(/file_path|parsed_text|resume=private-file/)).not.toBeInTheDocument();
  });

  it("shows employer-format-first guidance when readable text is missing", async () => {
    const resume = makeResumeSummary({
      name: "Scanned Resume",
      format_label: "PDF",
      has_readable_text: false,
      readable_text_chars: 0,
    });
    mockResumeLibraryResponses({
      get_active_resume: resume,
      list_all_resumes: [resume],
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Scanned Resume")).toBeInTheDocument();
    });

    expect(screen.getByText("No readable text found")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Follow employer file instructions first. This PDF may be scanned or image-only, so JobSentinel could not find selectable text. If no format is named, export a readable PDF, DOCX, TXT, Markdown, or HTML resume.",
      ),
    ).toBeInTheDocument();
  });

  it("shows employer-format-first guidance in the empty readable-text preview", async () => {
    const user = userEvent.setup();

    const resume = makeResumeSummary({
      name: "Scanned Resume",
      format_label: "PDF",
      has_readable_text: false,
      readable_text_chars: 0,
    });
    mockResumeLibraryResponses({
      get_active_resume: resume,
      list_all_resumes: [resume],
      get_resume_text_preview: {
        resume_id: 1,
        name: "Scanned Resume",
        has_text: false,
        text_preview: "",
        text_chars: 0,
        is_truncated: false,
      },
    });

    render(<ResumeLibraryPage onBack={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Scanned Resume")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "See what JobSentinel read" }));

    expect(await screen.findByText("Readable Resume Text")).toBeInTheDocument();
    expect(screen.getByText("Needs readable text")).toBeInTheDocument();
    expect(screen.getByText("Employer format first")).toBeInTheDocument();
    expect(screen.getByText("Preview not available")).toBeInTheDocument();
    expect(
      screen.getByText(
        "No selectable text found in this PDF. Follow employer file instructions first. If no format is named, try exporting a readable PDF, DOCX, TXT, Markdown, or HTML resume, or use a resume app export.",
      ),
    ).toBeInTheDocument();
  });

});
