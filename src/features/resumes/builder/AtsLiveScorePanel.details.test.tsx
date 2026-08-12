import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { AtsLiveScorePanel } from "./AtsLiveScorePanel";
import type { AtsAnalysisResult } from "./AtsLiveScorePanel";
import { mockAnalysis, mockResumeData } from "./AtsLiveScorePanel.testData";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("../../../shared/errorReporting/logger", () => ({
  logError: vi.fn(),
}));

const waitForAnalysis = async () => {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 100));
  });
};

describe("AtsLiveScorePanel details modal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
    mockInvoke.mockResolvedValue(mockAnalysis);
  });

  afterEach(() => {
    window.sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it("shows review details button", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
        showFullAnalysis={true}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^view details$/i })).not.toBeInTheDocument();
    });
  });

  it("hides review details button when showFullAnalysis is false", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
        showFullAnalysis={false}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getAllByText("Some evidence").length).toBeGreaterThan(0);
    });

    expect(screen.queryByRole("button", { name: /review details/i })).not.toBeInTheDocument();
  });

  it("opens modal when review details is clicked", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Resume Readability Review")).toBeInTheDocument();
    expect(screen.queryByText("Full Resume Readability Review")).not.toBeInTheDocument();
  });

  it("displays job words found in modal", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Words Found (2)")).toBeInTheDocument();
  });

  it("shows plain labels for live word evidence tooltips", async () => {
    mockInvoke.mockResolvedValue({
      ...mockAnalysis,
      keyword_matches: [
        {
          keyword: "Scheduling",
          importance: "Required",
          found_in: ["current experience", "skills"],
          frequency: 1,
        },
      ],
    } satisfies AtsAnalysisResult);

    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));
    fireEvent.mouseEnter(screen.getByText("Scheduling"));

    await waitFor(() => {
      expect(screen.getByRole("tooltip")).toHaveTextContent(
        "Found in: current role experience, skills list",
      );
    });
    expect(screen.queryByText("Found in: current experience, skills")).not.toBeInTheDocument();
  });

  it("shows keyboard-visible safe requirement evidence locations", async () => {
    const evidenceId = "e".repeat(64);
    const sourceRevision = "private-revision";
    mockInvoke.mockResolvedValue({
      ...mockAnalysis,
      requirement_reviews: [
        {
          keyword: "Scheduling",
          importance: "Required",
          match_state: "Direct",
          evidence_sections: ["current experience", "skills"],
          evidence_citations: [
            {
              evidence_id: evidenceId,
              source_revision: sourceRevision,
              field_path: "resume_text.3",
            },
            {
              evidence_id: "f".repeat(64),
              source_revision: sourceRevision,
              field_path: "skills.0",
            },
          ],
          hard_constraint: false,
          recommendation: "Keep the wording tied to your real experience.",
        },
      ],
    } satisfies AtsAnalysisResult);

    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );
    await waitForAnalysis();
    fireEvent.click(await screen.findByRole("button", { name: /review details/i }));

    expect(screen.getByText("Where Matching Wording Was Found")).toBeInTheDocument();
    const locations = screen.getByRole("list", {
      name: "Evidence locations for Scheduling",
    });
    const modalBody = locations.closest(".app-modal-body");
    expect(modalBody).toHaveClass("overflow-y-auto");
    expect(locations.parentElement?.closest(".overflow-y-auto")).toBe(modalBody);
    expect(within(locations).getByText("Resume text, line 4")).toBeInTheDocument();
    expect(within(locations).getByText("Skills list, item 1")).toBeInTheDocument();
    expect(
      screen.getByText("These locations show matching wording, not verification of a claim."),
    ).toBeInTheDocument();
    for (const internal of [evidenceId, sourceRevision, "resume_text.3", "skills.0"]) {
      expect(screen.queryByText(internal)).not.toBeInTheDocument();
    }
  });

  it("displays words to review in modal", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Words To Review (2)")).toBeInTheDocument();
    expect(screen.queryByText(/Words To Add/i)).not.toBeInTheDocument();
    expect(screen.getByText("Spanish")).toBeInTheDocument();
    expect(screen.getByText("Zendesk")).toBeInTheDocument();
  });

  it("displays format issues in modal", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Details to Check (1)")).toBeInTheDocument();
    expect(screen.getByText("Review")).toBeInTheDocument();
    expect(screen.getByText("Summary could be longer")).toBeInTheDocument();
    expect(screen.getByText(/Possible edit to review: Add more details/i)).toBeInTheDocument();
  });

  it("groups words to review by job-post importance", async () => {
    mockInvoke.mockResolvedValueOnce({
      ...mockAnalysis,
      missing_keywords: ["Spanish", "Zendesk", "Case management"],
      missing_keyword_details: [
        { keyword: "Spanish", importance: "Required" },
        { keyword: "Zendesk", importance: "Preferred" },
        { keyword: "Case management", importance: "Industry" },
      ],
    });

    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Words To Review (3)")).toBeInTheDocument();
    expect(screen.getByText("Required to Review")).toBeInTheDocument();
    expect(screen.getByText("Preferred to Review")).toBeInTheDocument();
    expect(screen.getByText("Nice-to-Have or Other to Review")).toBeInTheDocument();
    expect(screen.queryByText("Other Words to Review")).not.toBeInTheDocument();
    expect(screen.getByText("Spanish")).toBeInTheDocument();
    expect(screen.getByText("Zendesk")).toBeInTheDocument();
    expect(screen.getByText("Case management")).toBeInTheDocument();
    expect(screen.getByText(/Start with required job-post language/i)).toBeInTheDocument();
  });

  it("displays suggestions in modal", async () => {
    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Suggestions (1)")).toBeInTheDocument();
    expect(screen.getByText("Review job words")).toBeInTheDocument();
    expect(screen.getByText(/Review whether 'Spanish'/i)).toBeInTheDocument();
    expect(screen.queryByText("Add job words")).not.toBeInTheDocument();
    expect(screen.queryByText("AddKeyword")).not.toBeInTheDocument();
    expect(screen.getByText(/Why it helps: Required job-post language/i)).toBeInTheDocument();
  });

  it("displays safety suggestion category from backend format fixes", async () => {
    mockInvoke.mockResolvedValue({
      ...mockAnalysis,
      format_issues: [
        {
          severity: "Warning",
          issue: "Instruction-like or hidden resume text detected",
          fix: "Remove instructions aimed at screening tools and keep only truthful qualifications.",
        },
      ],
      suggestions: [
        {
          category: "FormatFix",
          suggestion:
            "Review the resume for prompt-injection-like instructions, hidden text, or invisible characters before using it.",
          impact:
            "Keeps the resume readable and avoids tactics that can backfire with employers or screening systems.",
        },
      ],
    } satisfies AtsAnalysisResult);

    render(
      <AtsLiveScorePanel
        resumeData={mockResumeData}
        currentStep={1}
        debounceMs={10}
      />,
    );

    await waitForAnalysis();

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review details/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review details/i }));

    expect(screen.getByText("Safety check")).toBeInTheDocument();
    expect(screen.getByText(/prompt-injection-like instructions/i)).toBeInTheDocument();
    expect(screen.queryByText("FormatFix")).not.toBeInTheDocument();
  });
});
