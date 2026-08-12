import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScreeningAnswersForm } from "./ScreeningAnswersForm";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockToast = {
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  warning: vi.fn(),
};
vi.mock("../../shared/toast/useToast", () => ({
  useToast: () => mockToast,
}));

describe("ScreeningAnswersForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockReset();
  });

  describe("answer input handling", () => {
    beforeEach(() => {
      mockInvoke.mockResolvedValue([]);
    });

    it("allows typing in question wording to look for field", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      fireEvent.change(patternInput, {
        target: { value: "salary" },
      });

      expect(patternInput).toHaveValue("salary");
    });

    it("shows hard screening guidance when typed question wording needs exact review", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      fireEvent.change(patternInput, {
        target: { value: "US citizen" },
      });

      expect(screen.getByTestId("hard-screening-answer-guidance")).toHaveTextContent(
        "Use only what is true and backed by your resume or records.",
      );
    });

    it("shows hard screening guidance for auto-insurance wording", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      fireEvent.change(patternInput, {
        target: { value: "proof of auto insurance" },
      });

      expect(screen.getByTestId("hard-screening-answer-guidance")).toHaveTextContent(
        "Use only what is true and backed by your resume or records.",
      );
    });

    it("keeps protected-answer patterns local and manual-only", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      fireEvent.change(screen.getByLabelText(/question wording to look for/i), {
        target: { value: "Do you identify as a protected veteran?" },
      });

      expect(screen.getByTestId("hard-screening-answer-guidance")).toHaveTextContent(
        "This voluntary or sensitive personal question stays local and manual-only. Review and answer it yourself.",
      );
    });

    it("does not treat civilian role or embedded word fragments as protected-answer patterns", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      fireEvent.change(screen.getByLabelText(/question wording to look for/i), {
        target: { value: "Care Coordinator" },
      });

      expect(screen.queryByTestId("hard-screening-answer-guidance")).not.toBeInTheDocument();

      fireEvent.change(screen.getByLabelText(/question wording to look for/i), {
        target: { value: "Distributed trace experience" },
      });

      expect(screen.queryByTestId("hard-screening-answer-guidance")).not.toBeInTheDocument();
    });

    it("allows typing in answer field", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const answerInput = screen.getByLabelText(/your answer/i);
      // Use fireEvent.change for controlled inputs — userEvent.type can drop
      // characters when React re-renders between keystrokes in test environments
      fireEvent.change(answerInput, {
        target: { value: "$100,000 - $120,000" },
      });

      expect(answerInput).toHaveValue("$100,000 - $120,000");
    });

    it("allows typing in notes field", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const notesInput = screen.getByLabelText(/notes/i);
      fireEvent.change(notesInput, { target: { value: "Test notes" } });

      expect(notesInput).toHaveValue("Test notes");
    });

    it("allows changing answer type", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const answerTypeSelect = screen.getByLabelText(/answer type/i);
      await user.selectOptions(answerTypeSelect, "yes_no");

      expect(answerTypeSelect).toHaveValue("yes_no");
    });

    it("switches input type when answer type changes", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      let answerInput = screen.getByLabelText(/your answer/i);
      expect(answerInput.tagName.toLowerCase()).toBe("input");

      const answerTypeSelect = screen.getByLabelText(/answer type/i);
      await user.selectOptions(answerTypeSelect, "textarea");

      await waitFor(() => {
        answerInput = screen.getByLabelText(/your answer/i);
        expect(answerInput.tagName.toLowerCase()).toBe("textarea");
      });
    });
  });

  describe("form validation", () => {
    beforeEach(() => {
      mockInvoke.mockResolvedValue([]);
    });

    it("shows error when submitting with empty question wording to look for", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const answerInput = screen.getByLabelText(/your answer/i);
      await user.type(answerInput, "Test answer");

      await user.click(screen.getByRole("button", { name: /save answer/i }));

      await waitFor(() => {
        expect(mockToast.error).toHaveBeenCalledWith(
          "Check highlighted fields",
          "Add the missing details, then save again.",
        );
      });
    });

    it("shows error when submitting with empty answer", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      await user.type(patternInput, "test.*pattern");

      await user.click(screen.getByRole("button", { name: /save answer/i }));

      await waitFor(() => {
        expect(mockToast.error).toHaveBeenCalledWith(
          "Check highlighted fields",
          "Add the missing details, then save again.",
        );
      });
    });

    it("validates question text match format", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      await user.type(patternInput, "   ");
      fireEvent.blur(patternInput);

      await waitFor(() => {
        expect(screen.getByText(/add question wording/i)).toBeInTheDocument();
      });
    });

    it("displays error styling for invalid pattern", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      await user.type(patternInput, "   ");
      fireEvent.blur(patternInput);

      await waitFor(() => {
        expect(screen.getByText(/add question wording/i)).toBeInTheDocument();
      });
    });

    it("clears error when valid input is entered", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const patternInput = screen.getByLabelText(/question wording to look for/i);
      await user.type(patternInput, "   ");
      fireEvent.blur(patternInput);

      await waitFor(() => {
        expect(screen.getByText(/add question wording/i)).toBeInTheDocument();
      });

      await user.clear(patternInput);
      await user.type(patternInput, "valid.*pattern");

      await waitFor(() => {
        expect(
          screen.queryByText(/add question wording/i),
        ).not.toBeInTheDocument();
      });
    });

    it("validates on blur", async () => {
      const user = userEvent.setup();
      render(<ScreeningAnswersForm />);

      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: /add answer/i }),
        ).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add answer/i }));

      const answerInput = screen.getByLabelText(/your answer/i);
      await user.click(answerInput);
      fireEvent.blur(answerInput);

      await waitFor(() => {
        expect(screen.getByText("Add answer.")).toBeInTheDocument();
      });
    });
  });

});
