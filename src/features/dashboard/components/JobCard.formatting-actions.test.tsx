/** Verifies Job Card formatting and user action routing. */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  mockJob,
  renderWithToast,
  setupJobCardMocks,
} from "./JobCard.testSupport";
import { JobCard } from "./JobCard";

describe("JobCard", () => {
  beforeEach(setupJobCardMocks);

  describe("salary formatting", () => {
    it("renders salary with both min and max", () => {
      renderWithToast(<JobCard job={mockJob} />);
      expect(screen.getByText("$55k - $72k")).toBeInTheDocument();
    });

    it("renders salary with only min", () => {
      const jobMinOnly = { ...mockJob, salary_min: 100000, salary_max: null };
      renderWithToast(<JobCard job={jobMinOnly} />);
      expect(screen.getByText("$100k+")).toBeInTheDocument();
      expect(screen.getByTestId("salary-range-quality-guidance")).toHaveTextContent(
        "Open-ended listed pay",
      );
      expect(
        screen.getByText(/confirm the realistic top range before tailoring/i),
      ).toBeInTheDocument();
    });

    it("renders salary with only max", () => {
      const jobMaxOnly = { ...mockJob, salary_min: null, salary_max: 150000 };
      renderWithToast(<JobCard job={jobMaxOnly} />);
      expect(screen.getByText("Up to $150k")).toBeInTheDocument();
      expect(screen.getByTestId("salary-range-quality-guidance")).toHaveTextContent(
        "Top-only listed pay",
      );
      expect(
        screen.getByText(/confirm the starting pay before tailoring/i),
      ).toBeInTheDocument();
    });

    it("shows a missing-pay cue when both salary fields are null", () => {
      const jobNoSalary = { ...mockJob, salary_min: null, salary_max: null };
      renderWithToast(<JobCard job={jobNoSalary} />);
      expect(screen.getByText("Pay not listed")).toBeInTheDocument();
      expect(screen.queryByText(/\$/)).not.toBeInTheDocument();
      expect(screen.queryByTestId("pay-floor-guidance")).not.toBeInTheDocument();
      expect(
        screen.getByRole("article", {
          name: /pay not listed/i,
        }),
      ).toBeInTheDocument();
    });

    it("shows regional pay-range guidance when a covered location has no listed pay", () => {
      const coloradoJobNoSalary = {
        ...mockJob,
        location: "Denver, CO",
        salary_min: null,
        salary_max: null,
      };

      renderWithToast(<JobCard job={coloradoJobNoSalary} />);

      expect(screen.getByTestId("pay-transparency-guidance")).toHaveTextContent(
        "Check pay range",
      );
      expect(screen.getByTestId("pay-transparency-guidance")).toHaveTextContent(
        "Colorado has pay-range posting rules for covered employers",
      );
      expect(
        screen.getByRole("article", {
          name: /pay range to check for colorado/i,
        }),
      ).toBeInTheDocument();
    });

    it("shows missing-pay floor guidance when salary floor exists", () => {
      const jobNoSalary = { ...mockJob, salary_min: null, salary_max: null };

      renderWithToast(<JobCard job={jobNoSalary} salaryFloorUsd={65000} />);

      const guidance = screen.getByTestId("pay-floor-guidance");
      expect(guidance).toHaveTextContent("Pay not listed");
      expect(guidance).toHaveTextContent(
        "No pay range is listed. Compare this role with your $65,000/year floor before tailoring.",
      );
      expect(
        screen.getByRole("article", {
          name: /pay not listed; compare before tailoring/i,
        }),
      ).toBeInTheDocument();
    });

    it("flags very wide listed pay ranges as weaker range evidence", () => {
      const wideRangeJob = {
        ...mockJob,
        salary_min: 45000,
        salary_max: 140000,
      };

      renderWithToast(<JobCard job={wideRangeJob} />);

      expect(screen.getByText("$45k - $140k")).toBeInTheDocument();
      expect(screen.getByTestId("salary-range-quality-guidance")).toHaveTextContent(
        "Very wide pay range",
      );
      expect(
        screen.getByText(/check the written level, schedule, and realistic pay/i),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("article", {
          name: /very wide pay range/i,
        }),
      ).toBeInTheDocument();
    });

    it("keeps normal listed pay ranges visually quiet", () => {
      renderWithToast(<JobCard job={mockJob} />);

      expect(screen.getByText("$55k - $72k")).toBeInTheDocument();
      expect(screen.queryByTestId("salary-range-quality-guidance")).not.toBeInTheDocument();
    });

    it("warns when listed pay is below the user's floor", () => {
      const belowFloorJob = {
        ...mockJob,
        salary_min: 45000,
        salary_max: 55000,
      };

      renderWithToast(
        <JobCard job={belowFloorJob} salaryFloorUsd={65000} />,
      );

      expect(screen.getByTestId("pay-floor-guidance")).toHaveTextContent(
        "Below the lowest pay you want",
      );
      expect(
        screen.getByText(/listed pay tops out below \$65,000\/year/i),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("article", {
          name: /below the lowest pay you want/i,
        }),
      ).toBeInTheDocument();
    });

    it("does not warn when listed pay starts at the user's floor", () => {
      const atFloorJob = {
        ...mockJob,
        salary_min: 65000,
        salary_max: 72000,
      };

      renderWithToast(<JobCard job={atFloorJob} salaryFloorUsd={65000} />);

      expect(screen.getByText("$65k - $72k")).toBeInTheDocument();
      expect(screen.queryByTestId("pay-floor-guidance")).not.toBeInTheDocument();
    });

    it("warns when listed pay starts below the user's floor", () => {
      const lowStartJob = {
        ...mockJob,
        salary_min: 55000,
        salary_max: 72000,
      };

      renderWithToast(<JobCard job={lowStartJob} salaryFloorUsd={65000} />);

      expect(screen.getByText("$55k - $72k")).toBeInTheDocument();
      expect(screen.getByTestId("pay-floor-guidance")).toHaveTextContent(
        "Starting pay below your floor",
      );
      expect(
        screen.getByText(/confirm where your experience would land before tailoring/i),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("article", {
          name: /starting pay below your floor; confirm range before tailoring/i,
        }),
      ).toBeInTheDocument();
    });

    it("shows a review cue when only starting pay is below the user's floor", () => {
      const minOnlyJob = {
        ...mockJob,
        salary_min: 45000,
        salary_max: null,
      };

      renderWithToast(<JobCard job={minOnlyJob} salaryFloorUsd={65000} />);

      expect(screen.getByText("$45k+")).toBeInTheDocument();
      expect(screen.getByTestId("pay-floor-guidance")).toHaveTextContent(
        "Open-ended listed pay",
      );
      expect(
        screen.getByText(/confirm the realistic top range before tailoring/i),
      ).toBeInTheDocument();
      expect(
        screen.queryByRole("article", {
          name: /below the lowest pay you want/i,
        }),
      ).not.toBeInTheDocument();
      expect(
        screen.getByRole("article", {
          name: /open-ended listed pay; confirm range before tailoring/i,
        }),
      ).toBeInTheDocument();
    });

    it("treats reversed listed pay as unavailable for floor guidance", () => {
      const reversedRangeJob = {
        ...mockJob,
        salary_min: 150000,
        salary_max: 80000,
      };

      renderWithToast(
        <JobCard job={reversedRangeJob} salaryFloorUsd={100000} />,
      );

      expect(screen.getByTestId("pay-floor-guidance")).toHaveTextContent(
        "Pay not listed",
      );
      expect(screen.queryByText("$150k - $80k")).not.toBeInTheDocument();
      expect(
        screen.queryByRole("article", {
          name: /below the lowest pay you want/i,
        }),
      ).not.toBeInTheDocument();
    });
  });

  describe("date formatting", () => {
    it('shows "Just now" for recent jobs', () => {
      const recentJob = { ...mockJob, created_at: new Date().toISOString() };
      renderWithToast(<JobCard job={recentJob} />);
      expect(screen.getByText("Just now")).toBeInTheDocument();
    });

    it("shows hours ago for jobs within 24 hours", () => {
      const date = new Date();
      date.setHours(date.getHours() - 5);
      const jobHoursAgo = { ...mockJob, created_at: date.toISOString() };
      renderWithToast(<JobCard job={jobHoursAgo} />);
      expect(screen.getByText("5h ago")).toBeInTheDocument();
    });

    it("shows days ago for jobs within a week", () => {
      const date = new Date();
      date.setDate(date.getDate() - 3);
      const jobDaysAgo = { ...mockJob, created_at: date.toISOString() };
      renderWithToast(<JobCard job={jobDaysAgo} />);
      expect(screen.getByText("3d ago")).toBeInTheDocument();
    });

    it("shows date for jobs older than a week", () => {
      const date = new Date();
      date.setDate(date.getDate() - 10);
      const jobOld = { ...mockJob, created_at: date.toISOString() };
      renderWithToast(<JobCard job={jobOld} />);
      const formatted = date.toLocaleDateString();
      expect(screen.getByText(formatted)).toBeInTheDocument();
    });
  });

  describe("bookmark button", () => {
    it("renders bookmark button when onToggleBookmark is provided", () => {
      const onToggleBookmark = vi.fn();
      renderWithToast(
        <JobCard job={mockJob} onToggleBookmark={onToggleBookmark} />,
      );
      expect(screen.getByTestId("btn-bookmark")).toBeInTheDocument();
    });

    it("does not render bookmark button when onToggleBookmark is not provided", () => {
      renderWithToast(<JobCard job={mockJob} />);
      expect(screen.queryByTestId("btn-bookmark")).not.toBeInTheDocument();
    });

    it("calls onToggleBookmark when clicked", async () => {
      const user = userEvent.setup();
      const onToggleBookmark = vi.fn();
      renderWithToast(
        <JobCard job={mockJob} onToggleBookmark={onToggleBookmark} />,
      );

      const bookmarkBtn = screen.getByTestId("btn-bookmark");
      await user.click(bookmarkBtn);

      expect(onToggleBookmark).toHaveBeenCalledWith(1);
    });

    it("shows filled bookmark icon when bookmarked", () => {
      const bookmarkedJob = { ...mockJob, bookmarked: true };
      renderWithToast(
        <JobCard job={bookmarkedJob} onToggleBookmark={vi.fn()} />,
      );

      const bookmarkBtn = screen.getByTestId("btn-bookmark");
      expect(bookmarkBtn).toHaveAttribute("data-bookmarked");
      expect(bookmarkBtn).toHaveAttribute("aria-label", "Remove bookmark");
    });

    it("shows empty bookmark icon when not bookmarked", () => {
      renderWithToast(<JobCard job={mockJob} onToggleBookmark={vi.fn()} />);

      const bookmarkBtn = screen.getByTestId("btn-bookmark");
      expect(bookmarkBtn).not.toHaveAttribute("data-bookmarked");
      expect(bookmarkBtn).toHaveAttribute("aria-label", "Bookmark this job");
    });
  });

  describe("notes button", () => {
    it("renders notes button when onEditNotes is provided", () => {
      const onEditNotes = vi.fn();
      renderWithToast(<JobCard job={mockJob} onEditNotes={onEditNotes} />);
      expect(screen.getByTestId("btn-notes")).toBeInTheDocument();
    });

    it("does not render notes button when onEditNotes is not provided", () => {
      renderWithToast(<JobCard job={mockJob} />);
      expect(screen.queryByTestId("btn-notes")).not.toBeInTheDocument();
    });

    it("calls onEditNotes with job id and notes when clicked", async () => {
      const user = userEvent.setup();
      const onEditNotes = vi.fn();
      renderWithToast(<JobCard job={mockJob} onEditNotes={onEditNotes} />);

      const notesBtn = screen.getByTestId("btn-notes");
      await user.click(notesBtn);

      expect(onEditNotes).toHaveBeenCalledWith(1, null);
    });

    it("shows filled notes icon when notes exist", () => {
      const jobWithNotes = { ...mockJob, notes: "Important company" };
      renderWithToast(<JobCard job={jobWithNotes} onEditNotes={vi.fn()} />);

      const notesBtn = screen.getByTestId("btn-notes");
      expect(notesBtn).toHaveAttribute("aria-label", "Edit notes");
    });

    it("shows empty notes icon when no notes", () => {
      renderWithToast(<JobCard job={mockJob} onEditNotes={vi.fn()} />);

      const notesBtn = screen.getByTestId("btn-notes");
      expect(notesBtn).toHaveAttribute("aria-label", "Add notes");
    });
  });

  describe("hide button", () => {
    it("renders hide button when onHideJob is provided", () => {
      const onHideJob = vi.fn();
      renderWithToast(<JobCard job={mockJob} onHideJob={onHideJob} />);
      expect(screen.getByTestId("btn-hide")).toBeInTheDocument();
    });

    it("does not render hide button when onHideJob is not provided", () => {
      renderWithToast(<JobCard job={mockJob} />);
      expect(screen.queryByTestId("btn-hide")).not.toBeInTheDocument();
    });

    it("calls onHideJob with job id when clicked", async () => {
      const user = userEvent.setup();
      const onHideJob = vi.fn();
      renderWithToast(<JobCard job={mockJob} onHideJob={onHideJob} />);

      const hideBtn = screen.getByTestId("btn-hide");
      await user.click(hideBtn);

      expect(onHideJob).toHaveBeenCalledWith(1);
    });

    it("has correct aria-label", () => {
      renderWithToast(<JobCard job={mockJob} onHideJob={vi.fn()} />);
      const hideBtn = screen.getByTestId("btn-hide");
      expect(hideBtn).toHaveAttribute(
        "aria-label",
        "Not interested in this job",
      );
    });
  });

  describe("research company button", () => {
    it("renders research button when onResearchCompany is provided", () => {
      const onResearchCompany = vi.fn();
      renderWithToast(
        <JobCard job={{ ...mockJob, hash: "job-carebridge" }} onResearchCompany={onResearchCompany} />,
      );
      expect(screen.getByTestId("btn-research")).toBeInTheDocument();
    });

    it("does not render research button when onResearchCompany is not provided", () => {
      renderWithToast(<JobCard job={mockJob} />);
      expect(screen.queryByTestId("btn-research")).not.toBeInTheDocument();
    });

    it("calls onResearchCompany with company name when clicked", async () => {
      const user = userEvent.setup();
      const onResearchCompany = vi.fn();
      renderWithToast(
        <JobCard
          job={{ ...mockJob, hash: "job-carebridge" }}
          onResearchCompany={onResearchCompany}
        />,
      );

      const researchBtn = screen.getByTestId("btn-research");
      await user.click(researchBtn);

      expect(onResearchCompany).toHaveBeenCalledWith(
        "CareBridge Services",
        "job-carebridge",
      );
    });
  });

});
