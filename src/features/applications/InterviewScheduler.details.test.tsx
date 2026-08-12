/** Verifies interview detail, debrief, feedback, and research behavior. */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import {
  mockApplications,
  mockInvoke,
  mockOnClose,
  mockToast,
  restoreInterviewSchedulerMocks,
  setupInterviewSchedulerMocks,
} from "./InterviewScheduler.testSupport";
import { InterviewScheduler } from "./InterviewScheduler";

describe("InterviewScheduler", () => {
  beforeEach(setupInterviewSchedulerMocks);
  afterEach(restoreInterviewSchedulerMocks);

  describe("add interview form", () => {
    it("opens form when Schedule button is clicked", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      // Find and click the Schedule button (first one, in header)
      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Schedule Interview")).toBeInTheDocument();
      });
    });

    it("shows application dropdown in form", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByLabelText(/Application/)).toBeInTheDocument();
      });
    });

    it("shows interview type dropdown in form", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByLabelText(/Interview style/)).toBeInTheDocument();
      });
    });

    it("shows date/time input in form", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByLabelText(/Date and time/)).toBeInTheDocument();
      });
    });

    it("shows duration dropdown in form", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByLabelText(/Duration/)).toBeInTheDocument();
      });
    });

    it("closes form when Cancel is clicked", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Schedule Interview")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Cancel"));

      await waitFor(() => {
        expect(screen.queryByText("Schedule Interview")).not.toBeInTheDocument();
      });
    });

    it("shows error when required fields are missing", async () => {
      render(
        <InterviewScheduler onClose={mockOnClose} applications={mockApplications} />
      );

      await waitFor(() => {
        expect(screen.getByText("Schedule")).toBeInTheDocument();
      });

      // Open the form
      const scheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(scheduleButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Schedule Interview")).toBeInTheDocument();
      });

      // Click the form's Schedule button (second one after modal opens)
      const allScheduleButtons = screen.getAllByText("Schedule");
      fireEvent.click(allScheduleButtons[allScheduleButtons.length - 1]);

      await waitFor(() => {
        expect(mockToast.error).toHaveBeenCalledWith(
          "Choose interview details",
          "Choose an application and time before scheduling.",
        );
      });
    });
  });

  describe("interview detail modal", () => {
    it("opens detail modal when interview is clicked", async () => {
      mockInvoke.mockResolvedValue([]);
      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      // Click on the interview card
      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        // Modal shows interview prep section
        expect(screen.getByText("Interview Prep")).toBeInTheDocument();
      });
    });

    it("shows interview prep checklist in modal", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Research company background")).toBeInTheDocument();
        expect(screen.getByText("Review job description")).toBeInTheDocument();
        expect(screen.getByText("Prepare questions to ask")).toBeInTheDocument();
      });
    });

    it("opens app-composed company research", async () => {
      mockInvoke.mockResolvedValue([]);
      const renderCompanyResearch = vi.fn(({ companyName }) => (
        <div>Research for {companyName}</div>
      ));

      render(
        <InterviewScheduler
          onClose={mockOnClose}
          renderCompanyResearch={renderCompanyResearch}
        />,
      );

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });
      fireEvent.click(screen.getByText("Customer Support Coordinator"));
      fireEvent.click(screen.getByRole("button", { name: "Research" }));

      expect(screen.getByText("Research for CareBridge Services")).toBeInTheDocument();
      expect(renderCompanyResearch).toHaveBeenCalledWith(
        expect.objectContaining({ companyName: "CareBridge Services", jobHash: "job-carebridge" }),
      );
    });

    it("shows Add to Calendar button in modal", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Add to Calendar")).toBeInTheDocument();
      });
    });

    it("shows Mark as Complete section in modal", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("How did it go?")).toBeInTheDocument();
      });
    });

    it("shows outcome buttons in modal", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Went well")).toBeInTheDocument();
        expect(screen.getByText("Not sure yet")).toBeInTheDocument();
        expect(screen.getByText("Not a fit")).toBeInTheDocument();
      });
    });

    it("shows feedback form when outcome button is clicked", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Went well")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Went well"));

      await waitFor(() => {
        expect(screen.getByText("Signal strength:")).toBeInTheDocument();
        expect(screen.getByText("Went well")).toBeInTheDocument();
      });
    });

    it("records a structured debrief through the existing interview owner", async () => {
      mockInvoke.mockResolvedValue([]);
      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });
      fireEvent.click(screen.getByText("Customer Support Coordinator"));
      fireEvent.click(await screen.findByRole("button", { name: "Went well" }));

      expect(screen.getByRole("heading", { name: "Post-interview debrief" })).toBeVisible();
      fireEvent.change(screen.getByLabelText("Questions asked"), {
        target: { value: "How would I handle an urgent customer escalation?" },
      });
      fireEvent.change(screen.getByLabelText("Concerns"), {
        target: { value: "The weekend rotation needs clarification." },
      });
      fireEvent.change(screen.getByLabelText("Promised next steps"), {
        target: { value: "The recruiter will confirm the final-round schedule." },
      });
      fireEvent.change(screen.getByLabelText("Follow-up deadline"), {
        target: { value: "2026-08-01" },
      });
      fireEvent.click(screen.getByRole("button", { name: "Save debrief" }));

      await waitFor(() =>
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_interview",
          {
            interviewId: 1,
            outcome: "passed",
            notes: [
              "Questions asked: How would I handle an urgent customer escalation?",
              "Concerns: The weekend rotation needs clarification.",
              "Promised next steps: The recruiter will confirm the final-round schedule.",
              "Follow-up deadline: 2026-08-01",
            ].join("\n"),
          },
          expect.anything(),
          expect.anything(),
        ),
      );
      expect(mockInvoke).not.toHaveBeenCalledWith(
        "update_application_status",
        expect.anything(),
        expect.anything(),
        expect.anything(),
      );
    });

    it("shows Delete button in modal", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });
    });

    it("shows delete confirmation when Delete is clicked", async () => {
      mockInvoke.mockResolvedValue([]);

      render(<InterviewScheduler onClose={mockOnClose} />);

      await waitFor(() => {
        expect(screen.getByText("Customer Support Coordinator")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Customer Support Coordinator"));

      await waitFor(() => {
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText("Delete"));

      await waitFor(() => {
        expect(screen.getByText("Delete this interview?")).toBeInTheDocument();
      });
    });
  });

});
