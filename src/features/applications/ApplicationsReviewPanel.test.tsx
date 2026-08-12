import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApplicationsReviewPanel } from "./ApplicationsReviewPanel";
import type { ApplicationReviewSummary } from "./applicationsModel";

const summary: ApplicationReviewSummary = {
  title: "Do these first",
  description: "Start with the highest priority work.",
  actions: [
    {
      id: "reminder-1",
      kind: "reminders",
      priority: "high",
      title: "Finish reminders",
      description: "2 follow-ups, prep items, or deadlines are waiting.",
      reminderId: 1,
    },
    {
      id: "interview-3",
      kind: "interviews",
      priority: "medium",
      title: "Prepare for interviews",
      description: "1 conversation needs prep or follow-up.",
      applicationId: 3,
    },
    {
      id: "to-apply-4",
      kind: "to_apply",
      priority: "low",
      title: "Review this tracked role",
      description: "Review its saved status and notes.",
      applicationId: 4,
      handoff: {
        label: "Application tracking",
        description: "Confirm status and notes, then decide whether to apply, skip, or close the role.",
      },
    },
    {
      id: "weekly-review",
      kind: "weekly_review",
      priority: "low",
      title: "Replan this week",
      description: "Compare sources and quiet roles before changing pace.",
      handoff: {
        label: "Job-search plan",
        description: "Replan lanes, sources, pacing, and stop rules from tracker evidence.",
      },
    },
  ],
};

describe("ApplicationsReviewPanel", () => {
  it("renders concrete mission actions and returns the selected target", () => {
    const onSelectAction = vi.fn();
    const handlers = {
      onSelectAction,
      onOpenSummary: vi.fn(),
      onGoToJobs: vi.fn(),
      onImportJob: vi.fn(),
    };

    render(<ApplicationsReviewPanel summary={summary} {...handlers} />);

    expect(screen.getByText("Daily mission")).toBeInTheDocument();
    expect(screen.getByText("Finish reminders")).toBeInTheDocument();
    expect(screen.getByText("Prepare for interviews")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Review this tracked role" }),
    ).toBeInTheDocument();
    expect(screen.getByText("After this: Application tracking")).toBeInTheDocument();
    expect(screen.getByText("After this: Job-search plan")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Review reminders" }));
    fireEvent.click(screen.getByRole("button", { name: "Open interviews" }));
    fireEvent.click(screen.getByRole("button", { name: "Review this tracked role" }));
    fireEvent.click(screen.getByRole("button", { name: "Review weekly plan" }));
    fireEvent.click(screen.getByRole("button", { name: "Open Summary" }));

    expect(onSelectAction.mock.calls.map(([action]) => action.id)).toEqual([
      "reminder-1",
      "interview-3",
      "to-apply-4",
      "weekly-review",
    ]);
    expect(handlers.onOpenSummary).toHaveBeenCalledTimes(1);
  });

  it("returns the empty-state import action without starting it", () => {
    const onSelectAction = vi.fn();

    render(
      <ApplicationsReviewPanel
        summary={{
          ...summary,
          actions: [
            {
              id: "add-job",
              kind: "to_apply",
              priority: "low",
              title: "Add a job to track",
              description: "Start with one saved, pasted, or imported job.",
            },
          ],
        }}
        onSelectAction={onSelectAction}
        onOpenSummary={vi.fn()}
        onGoToJobs={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Add or import job" }));

    expect(onSelectAction).toHaveBeenCalledWith(
      expect.objectContaining({ id: "add-job", kind: "to_apply" }),
    );
  });
});
