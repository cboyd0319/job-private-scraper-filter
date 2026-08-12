import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import { AtsLiveScorePanel } from "./AtsLiveScorePanel";
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

describe("AtsLiveScorePanel context and tips", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
    mockInvoke.mockResolvedValue(mockAnalysis);
  });

  afterEach(() => {
    window.sessionStorage.clear();
    vi.restoreAllMocks();
  });

  describe("tips section", () => {
    it("shows tips for contact step", () => {
      render(<AtsLiveScorePanel resumeData={null} currentStep={1} />);

      expect(screen.getByText("Quick Tips")).toBeInTheDocument();
      expect(screen.getByText("Include a professional email address")).toBeInTheDocument();
    });

    it("shows tips for summary step", () => {
      render(<AtsLiveScorePanel resumeData={null} currentStep={2} />);

      expect(screen.getByText("Keep summary to 2-3 sentences")).toBeInTheDocument();
    });

    it("shows tips for experience step", () => {
      render(<AtsLiveScorePanel resumeData={null} currentStep={3} />);

      expect(screen.getByText("Use action verbs to start bullet points")).toBeInTheDocument();
    });

    it("shows tips for education step", () => {
      render(<AtsLiveScorePanel resumeData={null} currentStep={4} />);

      expect(screen.getByText("Include degree, institution, and graduation date")).toBeInTheDocument();
    });

    it("shows tips for skills step", () => {
      render(<AtsLiveScorePanel resumeData={null} currentStep={5} />);

      expect(screen.getByText("Match skills to job requirements")).toBeInTheDocument();
      expect(screen.getByText("Include tools, workplace, and role-specific skills")).toBeInTheDocument();
      expect(screen.queryByText("Include technical, workplace, and role-specific skills")).not.toBeInTheDocument();
    });

    it("shows analysis-based tips when format score is low", async () => {
      mockInvoke.mockResolvedValue({ ...mockAnalysis, format_score: 50 });

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();

      await waitFor(() => {
        expect(screen.getByText("Use a simpler format so hiring systems can read it")).toBeInTheDocument();
      });
    });

    it("frames missing job words as truthful review", async () => {
      mockInvoke.mockResolvedValue({
        ...mockAnalysis,
        missing_keywords: ["Spanish", "Zendesk", "Scheduling", "Inventory"],
      });

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();

      await waitFor(() => {
        expect(screen.getByText(/Review job-post words for truthful fit:/)).toBeInTheDocument();
      });
      expect(screen.queryByText(/Add words from the job post/)).not.toBeInTheDocument();
    });
  });

  describe("debouncing", () => {
    it("uses format-only analysis when no job description", async () => {
      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "analyze_resume_format",
          expect.any(Object)
        );
      });
    });

    it("respects custom debounce time", async () => {
      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      expect(mockInvoke).not.toHaveBeenCalled();

      await waitForAnalysis();

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalled();
      });
    });

    it("uses a new evidence revision after an unsaved edit", async () => {
      const { rerender } = render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();
      const firstSnapshot = mockInvoke.mock.calls[0]?.[1].resume.evidence_snapshot;
      mockInvoke.mockClear();

      rerender(
        <AtsLiveScorePanel
          resumeData={{ ...mockResumeData, summary: "Updated unsaved summary" }}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();
      await waitFor(() => expect(mockInvoke).toHaveBeenCalledOnce());
      const secondSnapshot = mockInvoke.mock.calls[0]?.[1].resume.evidence_snapshot;
      expect(secondSnapshot).toEqual({
        source_id: "resume-draft:7",
        revision: expect.any(String),
      });
      expect(secondSnapshot.revision).not.toBe(firstSnapshot.revision);
    });
  });

  describe("job context", () => {
    it("does not show Saved Job badge without job description", () => {
      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      expect(screen.queryByText("Saved Job")).not.toBeInTheDocument();
    });

    it("uses valid stored job context for full analysis", async () => {
      window.sessionStorage.setItem(
        "jobContext",
        JSON.stringify({
          timestamp: Date.now(),
          description: "Bilingual customer support role",
        }),
      );

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitFor(() => {
        expect(screen.getByText("Saved Job")).toBeInTheDocument();
      });
      await waitForAnalysis();

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "analyze_resume_for_job",
          expect.objectContaining({
            jobDescription: "Bilingual customer support role",
            resume: expect.objectContaining({
              evidence_snapshot: {
                source_id: "resume-draft:7",
                revision: expect.any(String),
              },
            }),
          }),
        );
      });
      expect(window.sessionStorage.getItem("jobContext")).toBeNull();
    });

    it("ignores malformed stored job context", async () => {
      window.sessionStorage.setItem(
        "jobContext",
        JSON.stringify({
          timestamp: Date.now(),
          description: { text: "not a string" },
        }),
      );

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      expect(screen.queryByText("Saved Job")).not.toBeInTheDocument();
      await waitForAnalysis();

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "analyze_resume_format",
          expect.any(Object),
        );
      });
    });

    it("ignores expired job context after 30 minutes", () => {
      window.sessionStorage.setItem(
        "jobContext",
        JSON.stringify({
          timestamp: Date.now() - 31 * 60 * 1000,
          description: "Expired context",
        }),
      );

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      expect(screen.queryByText("Saved Job")).not.toBeInTheDocument();
      expect(window.sessionStorage.getItem("jobContext")).toBeNull();
    });
  });

  describe("empty analysis sections", () => {
    it("hides keyword matches when empty", async () => {
      mockInvoke.mockResolvedValue({
        ...mockAnalysis,
        keyword_matches: [],
        missing_keywords: [],
        format_issues: [],
      });

      render(
        <AtsLiveScorePanel
          resumeData={mockResumeData}
          currentStep={1}
          debounceMs={10}
        />
      );

      await waitForAnalysis();

      await waitFor(() => {
        expect(screen.getAllByText("Some evidence").length).toBeGreaterThan(0);
      });

      expect(screen.queryByText(/job words found/)).not.toBeInTheDocument();
      expect(screen.queryByText(/missing/)).not.toBeInTheDocument();
      expect(screen.queryByText(/issues/)).not.toBeInTheDocument();
    });
  });
});
