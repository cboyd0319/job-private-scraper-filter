import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  mockAtsDetection,
  mockInvoke,
  mockJob,
  makeFormFillResult,
  renderWithToast,
  setupApplyButtonMocks,
} from "./ApplyButton.testSupport";
import { ApplyButton } from "./ApplyButton";

async function openSubmitConfirmation(
  attemptId: string,
  trackSubmission = false,
) {
  const user = userEvent.setup();
  vi.mocked(localStorage.getItem).mockReturnValue(attemptId);
  mockInvoke.mockImplementation((cmd) => {
    if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
    if (cmd === "has_application_profile") return Promise.resolve(true);
    if (cmd === "is_browser_running") return Promise.resolve(true);
    if (cmd === "close_automation_browser") return Promise.resolve(null);
    if (trackSubmission && cmd === "mark_attempt_submitted") {
      return Promise.resolve(null);
    }
    return Promise.resolve(null);
  });
  renderWithToast(<ApplyButton job={mockJob} />);
  await waitFor(() => {
    expect(
      screen.getByRole("button", { name: /close browser/i }),
    ).toBeInTheDocument();
  });
  await user.click(screen.getByRole("button", { name: /close browser/i }));
  await waitFor(() => {
    expect(
      screen.getByText("Did you submit the application?"),
    ).toBeInTheDocument();
  });
  return user;
}

describe("ApplyButton lifecycle behavior", () => {
  beforeEach(setupApplyButtonMocks);

  describe("browser management", () => {
    it("shows Close Browser button when browser is running", async () => {
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /close browser/i })).toBeInTheDocument();
      });
    });

    it("switches from prepare form to Close Browser after form fill", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        if (cmd === "fill_application_form") {
          return Promise.resolve({
            filledFields: ["name"],
            unfilledFields: [],
            captchaDetected: false,
            readyForReview: true,
            errorMessage: null,
            attemptId: 123,
            durationMs: 1000,
            atsPlatform: "greenhouse",
          });
        }
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /prepare details/i }));

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /close browser/i })).toBeInTheDocument();
      });
    });

    it("closes browser when clicking Close Browser button", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(true);
        if (cmd === "close_automation_browser") return Promise.resolve(null);
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /close browser/i })).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /close browser/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("close_automation_browser", undefined);
      });
    });
  });

  describe("human-check handling", () => {
    it("shows human-check warning when the site asks for one", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        if (cmd === "fill_application_form") {
          return Promise.resolve(
            makeFormFillResult({ captchaDetected: true }),
          );
        }
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).not.toBeDisabled();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /prepare details/i }));

      await waitFor(() => {
        expect(screen.getByText("Site asked for a human check")).toBeInTheDocument();
        expect(screen.queryByText(/captcha detected/i)).not.toBeInTheDocument();
      });
    });
  });

  describe("screening questions", () => {
    it("reports screening questions filled", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        if (cmd === "fill_application_form") {
          return Promise.resolve({
            filledFields: ["name", "email", "screening:years_experience", "screening:work_auth"],
            screeningAnswerTopics: ["work authorization", "education"],
            unfilledFields: [],
            captchaDetected: false,
            readyForReview: true,
            errorMessage: null,
            attemptId: 123,
            durationMs: 2000,
            atsPlatform: "greenhouse",
          });
        }
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).not.toBeDisabled();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /prepare details/i }));

      await waitFor(() => {
        expect(
          screen.getByText(/prepared 2 profile fields and 2 saved screening answers/i),
        ).toBeInTheDocument();
        expect(
          screen.getByText(/check saved answers for work authorization and education/i),
        ).toBeInTheDocument();
      });
    });

    it("reports voluntary or sensitive personal questions left untouched without exposing their text", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        if (cmd === "fill_application_form") {
          return Promise.resolve({
            filledFields: ["name"],
            manualReviewTopics: ["voluntary or sensitive personal questions"],
            unfilledFields: [],
            captchaDetected: false,
            readyForReview: true,
            errorMessage: null,
            attemptId: 123,
            durationMs: 2000,
            atsPlatform: "greenhouse",
          });
        }
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).not.toBeDisabled();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));
      await user.click(screen.getByRole("button", { name: /prepare details/i }));

      await waitFor(() => {
        expect(
          screen.getByText(/voluntary or sensitive personal questions were left untouched/i),
        ).toBeInTheDocument();
      });
      expect(screen.queryByText(/protected veteran/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/decline to answer/i)).not.toBeInTheDocument();
    });
  });

  describe("localStorage persistence", () => {
    it("stores attempt ID after successful form fill", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        if (cmd === "fill_application_form") {
          return Promise.resolve({
            filledFields: ["name"],
            unfilledFields: [],
            captchaDetected: false,
            readyForReview: true,
            errorMessage: null,
            attemptId: 456,
            durationMs: 1000,
            atsPlatform: "greenhouse",
          });
        }
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).not.toBeDisabled();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /prepare details/i }));

      await waitFor(() => {
        expect(localStorage.setItem).toHaveBeenCalledWith("lastAttempt_test-hash-123", "456");
      });
    });

    it("loads previous attempt ID on mount", () => {
      vi.mocked(localStorage.getItem).mockReturnValue("789");

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      expect(localStorage.getItem).toHaveBeenCalledWith("lastAttempt_test-hash-123");
    });
  });

  describe("submit confirmation modal", () => {
    it("shows submit confirmation after closing browser with pending attempt", async () => {
      await openSubmitConfirmation("999");
    });

    it("marks attempt as submitted when clicking Yes", async () => {
      const user = await openSubmitConfirmation("888", true);

      await user.click(screen.getByRole("button", { name: /yes, i submitted it/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("mark_attempt_submitted", { attemptId: 888 });
      });
    });

    it("removes localStorage entry after marking submitted", async () => {
      const user = await openSubmitConfirmation("777", true);

      await user.click(screen.getByRole("button", { name: /yes, i submitted it/i }));

      await waitFor(() => {
        expect(localStorage.removeItem).toHaveBeenCalledWith("lastAttempt_test-hash-123");
      });
    });

    it("skips tracking when clicking No", async () => {
      const user = await openSubmitConfirmation("666");

      await user.click(screen.getByRole("button", { name: /no, skip/i }));

      await waitFor(() => {
        expect(localStorage.removeItem).toHaveBeenCalledWith("lastAttempt_test-hash-123");
        expect(screen.getByText("Not added to your board")).toBeInTheDocument();
      });
    });
  });

  describe("accessibility", () => {
    it("button has proper role", async () => {
      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).toBeInTheDocument();
      });
    });

    it("modal has proper role and title", async () => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation((cmd) => {
        if (cmd === "detect_ats_platform") return Promise.resolve(mockAtsDetection);
        if (cmd === "has_application_profile") return Promise.resolve(true);
        if (cmd === "is_browser_running") return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithToast(<ApplyButton job={mockJob} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /prepare form/i })).not.toBeDisabled();
      });

      await user.click(screen.getByRole("button", { name: /prepare form/i }));

      await waitFor(() => {
        const dialog = screen.getByRole("dialog");
        expect(dialog).toBeInTheDocument();
        expect(screen.getByText("Review Application")).toBeInTheDocument();
      });
    });
  });
});
