import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  mockAnalysis,
  mockInvoke,
  mockInvokeResponses,
  mockReadStorageValue,
  mockRemoveStorageValue,
  mockToast,
  mockWriteStorageValue,
  openResumeAppImport,
  setupResumeMatchMocks,
  validResume,
} from "./ResumeMatchPage.testSupport";
import ResumeMatch from "./ResumeMatchPage";

const mockJobAnalysis = {
  ...mockAnalysis,
  keyword_matches: [
    {
      keyword: "onboarding",
      importance: "Required" as const,
      found_in: ["experience"],
      frequency: 2,
    },
    {
      keyword: "retention",
      importance: "Preferred" as const,
      found_in: ["summary"],
      frequency: 1,
    },
  ],
  missing_keywords: ["account management"],
  missing_keyword_details: [
    {
      keyword: "account management",
      importance: "Required" as const,
    },
  ],
};

const mockActiveResume = {
  id: 42,
  name: "Customer Success Resume",
  is_active: true,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-02T00:00:00Z",
  format_label: "PDF",
  has_readable_text: true,
  readable_text_chars: 1520,
};

describe("ResumeMatch", () => {
  beforeEach(setupResumeMatchMocks);

  it("exposes named back and bullet draft controls", async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();

    render(<ResumeMatch onBack={onBack} />);

    await user.click(screen.getByRole("button", { name: /back to dashboard/i }));
    expect(onBack).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: /draft alternative bullet/i }));

    expect(
      screen.getByRole("textbox", { name: /current bullet point/i }),
    ).toBeInTheDocument();
  });

  it("starts with choose or add before copied resume details import", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<ResumeMatch onBack={vi.fn()} onNavigate={onNavigate} />);

    expect(screen.getByText(/choose a saved resume or add one/i)).toBeInTheDocument();
    expect(screen.queryByLabelText(/copied resume details/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/for a PDF resume/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/choose a saved resume or upload one/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /choose or add resume/i }));

    await waitFor(() => {
      expect(onNavigate).toHaveBeenCalledWith("resume");
    });

    await openResumeAppImport(user);

    expect(screen.getByLabelText(/copied resume details/i)).toBeInTheDocument();
  });

  it("restores local review draft after navigating to add a resume", async () => {
    mockReadStorageValue.mockReturnValueOnce(JSON.stringify({
      jobDescription: "Need onboarding and retention experience",
      resumeJson: JSON.stringify(validResume),
      analysisResult: mockJobAnalysis,
      analysisInputSource: "copied",
      showAdvancedResumeImport: true,
      showComparison: true,
    }));

    render(<ResumeMatch onBack={vi.fn()} onNavigate={vi.fn()} />);

    expect(screen.getByLabelText(/^job post$/i)).toHaveValue(
      "Need onboarding and retention experience",
    );
    expect(screen.getByLabelText(/copied resume details/i)).toHaveValue(
      JSON.stringify(validResume),
    );
    expect(screen.getByText("Resume Fit")).toBeInTheDocument();
    expect(mockRemoveStorageValue).toHaveBeenCalledWith(
      "session",
      "jobsentinel-resume-match-draft-v1",
    );
  });

  it("discloses same-session copied resume draft restore behavior", async () => {
    const user = userEvent.setup();

    render(<ResumeMatch onBack={vi.fn()} onNavigate={vi.fn()} />);

    await openResumeAppImport(user);

    expect(
      screen.getByText(/can be restored during this app session/i),
    ).toBeInTheDocument();
  });

  it("saves local review draft before opening the resume page", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();
    render(<ResumeMatch onBack={vi.fn()} onNavigate={onNavigate} />);

    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Need onboarding and retention experience" },
    });
    await openResumeAppImport(user);
    fireEvent.change(screen.getByLabelText(/copied resume details/i), {
      target: { value: JSON.stringify(validResume) },
    });

    await user.click(screen.getByRole("button", { name: /choose or add resume/i }));

    expect(mockWriteStorageValue).toHaveBeenCalledWith(
      "session",
      "jobsentinel-resume-match-draft-v1",
      expect.stringContaining("Need onboarding and retention experience"),
    );
    expect(onNavigate).toHaveBeenCalledWith("resume");
  });

  it("reviews a job against the active saved resume without copied details", async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") return Promise.resolve(mockActiveResume);
      if (command === "analyze_active_resume_for_job") return Promise.resolve(mockJobAnalysis);
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: /choose or add resume/i }));

    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    expect(screen.getByText(mockActiveResume.name)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Need onboarding, retention, and account management experience" },
    });
    await user.click(screen.getByRole("button", { name: /review match/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("analyze_active_resume_for_job", {
        jobDescription: "Need onboarding, retention, and account management experience",
      });
    });
    expect(mockInvoke).not.toHaveBeenCalledWith("analyze_resume_for_job", expect.anything());
    expect(await screen.findByText("Resume Fit")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /show comparison/i })).not.toBeInTheDocument();
  });

  it("uses only an explicitly selected role and regional matching profile", async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") return Promise.resolve(mockActiveResume);
      if (command === "analyze_active_resume_for_job") {
        return Promise.resolve({
          ...mockJobAnalysis,
          matching_profile: { profession: "operations", region: "uk" },
        });
      }
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    await user.selectOptions(
      screen.getByRole("combobox", { name: /role evidence focus/i }),
      "operations",
    );
    await user.selectOptions(
      screen.getByRole("combobox", { name: /job market wording/i }),
      "uk",
    );
    expect(
      screen.getByText(/does not claim complete regional terminology or employer coverage/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /role evidence focus changes only same-priority review order.*regional spellings can change recognized matches and scores/i,
      ),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Required: programme evaluation" },
    });
    await user.click(screen.getByRole("button", { name: /review match/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("analyze_active_resume_for_job", {
        jobDescription: "Required: programme evaluation",
        matchingProfile: { profession: "operations", region: "uk" },
      });
    });
  });

  it("locks the selected profile while its review is in flight", async () => {
    const user = userEvent.setup();
    let resolveAnalysis!: (result: typeof mockJobAnalysis) => void;
    const pendingAnalysis = new Promise<typeof mockJobAnalysis>((resolve) => {
      resolveAnalysis = resolve;
    });
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") return Promise.resolve(mockActiveResume);
      if (command === "analyze_active_resume_for_job") return pendingAnalysis;
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    const role = screen.getByRole("combobox", { name: /role evidence focus/i });
    const region = screen.getByRole("combobox", { name: /job market wording/i });
    await user.selectOptions(role, "operations");
    await user.selectOptions(region, "uk");
    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Required: programme evaluation" },
    });
    await user.click(screen.getByRole("button", { name: /review match/i }));

    expect(role).toBeDisabled();
    expect(region).toBeDisabled();
    expect(region).toHaveValue("uk");

    resolveAnalysis(mockJobAnalysis);
    expect(await screen.findByText("Resume Fit")).toBeInTheDocument();
    expect(role).toBeEnabled();
    expect(region).toBeEnabled();
    expect(region).toHaveValue("uk");
  });

  it("shows selected resume readable-text status before match review", async () => {
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") return Promise.resolve(mockActiveResume);
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    expect(screen.getByText("PDF")).toBeInTheDocument();
    expect(screen.getByText("1,520 readable characters for local review.")).toBeInTheDocument();
    expect(screen.queryByText(/file_path|parsed_text|resume=private-file/i)).not.toBeInTheDocument();
  });

  it("shows unreadable selected resume status before match review", async () => {
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") {
        return Promise.resolve({
          ...mockActiveResume,
          format_label: "DOCX",
          has_readable_text: false,
          readable_text_chars: 0,
        });
      }
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    expect(screen.getByText("DOCX")).toBeInTheDocument();
    expect(
      screen.getByText(
        "No readable text found. Follow employer file instructions first, then choose a readable PDF, DOCX, TXT, Markdown, or HTML resume.",
      ),
    ).toBeInTheDocument();
  });

  it("loads the active saved resume on page open and reviews without an extra resume click", async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((command) => {
      if (command === "get_active_resume") return Promise.resolve(mockActiveResume);
      if (command === "analyze_active_resume_for_job") return Promise.resolve(mockJobAnalysis);
      return Promise.resolve(mockAnalysis);
    });
    render(<ResumeMatch onBack={vi.fn()} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_active_resume");
    });
    expect(await screen.findByText(/selected resume:/i)).toBeInTheDocument();
    expect(screen.getByText(mockActiveResume.name)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Need onboarding, retention, and account management experience" },
    });
    await user.click(screen.getByRole("button", { name: /review match/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("analyze_active_resume_for_job", {
        jobDescription: "Need onboarding, retention, and account management experience",
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith(
      "Review ready",
      "Use the details below as a guide before you apply.",
    );
    expect(mockToast.success).not.toHaveBeenCalledWith(
      "Resume selected",
      expect.any(String),
    );
  });

  it("validates copied resume details before format review", async () => {
    const user = userEvent.setup();
    render(<ResumeMatch onBack={vi.fn()} />);

    await openResumeAppImport(user);

    fireEvent.change(screen.getByLabelText(/copied resume details/i), {
      target: { value: JSON.stringify({ contact_info: { name: "Jane" } }) },
    });

    await user.click(screen.getByRole("button", { name: /review format only/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Could not read copied resume details",
      "Choose or add a resume instead, or paste copied resume details from JobSentinel or another resume app.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith("analyze_resume_format", expect.anything());
  });

  it("validates copied resume details before match review", async () => {
    const user = userEvent.setup();
    render(<ResumeMatch onBack={vi.fn()} />);

    await openResumeAppImport(user);

    fireEvent.change(screen.getByLabelText(/^job post$/i), {
      target: { value: "Need onboarding and retention experience" },
    });
    fireEvent.change(screen.getByLabelText(/copied resume details/i), {
      target: { value: JSON.stringify("not a resume") },
    });

    await user.click(screen.getByRole("button", { name: /review match/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Could not read copied resume details",
      "Choose or add a resume instead, or paste copied resume details from JobSentinel or another resume app.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith("analyze_resume_for_job", expect.anything());
  });

  it("uses action-first validation copy before match review without a job post", async () => {
    const user = userEvent.setup();
    render(<ResumeMatch onBack={vi.fn()} />);

    await openResumeAppImport(user);
    fireEvent.change(screen.getByLabelText(/copied resume details/i), {
      target: { value: JSON.stringify(validResume) },
    });

    await user.click(screen.getByRole("button", { name: /review match/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Add job post",
      "Paste the job post, then review again.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith("analyze_resume_for_job", expect.anything());
  });

  it("submits valid copied resume details for format review", async () => {
    const user = userEvent.setup();
    render(<ResumeMatch onBack={vi.fn()} />);

    await openResumeAppImport(user);

    fireEvent.change(screen.getByLabelText(/copied resume details/i), {
      target: { value: JSON.stringify(validResume) },
    });

    await user.click(screen.getByRole("button", { name: /review format only/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("analyze_resume_format", {
        resume: validResume,
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith(
      "Format review complete",
      "Review the format details below.",
    );
  });

  it("explains strong resume words without screening-tool framing", async () => {
    const user = userEvent.setup();
    mockInvokeResponses({ get_ats_power_words: ["Led", "Improved"] });
    render(<ResumeMatch onBack={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: /view action words/i }));

    expect(await screen.findByText("Action Words for Clarity")).toBeInTheDocument();
    expect(screen.getByText(/make bullet points easier to scan/i)).toBeInTheDocument();
    expect(screen.queryByText(/Strong Resume Words/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/ATS systems/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/screening tools/i)).not.toBeInTheDocument();
  });

});
