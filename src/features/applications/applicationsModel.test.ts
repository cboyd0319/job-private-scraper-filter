/** Verifies application projections, summaries, and next-action guidance. */

import { describe, expect, it } from "vitest";
import {
  findApplicationById,
  findColumnForApplication,
  formatReminderType,
  getApplicationReviewSummary,
  getApplicationStats,
  getInterviewSchedulerApplications,
  hasAnyApplications,
  type Application,
  type ApplicationsByStatus,
  type PendingReminder,
} from "./applicationsModel";

const baseApplication: Application = {
  id: 1,
  job_hash: "job-1",
  job_title: "Care Coordinator",
  company: "Community Clinic",
  status: "applied",
  applied_at: "2026-06-01T12:00:00Z",
  notes: null,
  last_contact: null,
};

const routineReviewActions = [
  {
    id: "weekly-review",
    handoff: {
      description: "Replan lanes, sources, pacing, and stop rules from tracker evidence.",
      label: "Job-search plan",
    },
    kind: "weekly_review",
    priority: "low",
    title: "Review this week's plan",
    description: "Compare sources, quiet roles, saved roles, responses, and interviews before changing pace.",
  },
  {
    id: "source-review",
    handoff: {
      description: "Review enabled sources and start any connectivity-required check yourself.",
      label: "Job sources",
    },
    kind: "source_review",
    priority: "low",
    title: "Review job sources",
    description: "Review enabled sources; no source is contacted until you choose a source action.",
  },
] as const;

function emptyApplications(): ApplicationsByStatus {
  return {
    to_apply: [],
    applied: [],
    screening_call: [],
    phone_interview: [],
    technical_interview: [],
    onsite_interview: [],
    offer_received: [],
    offer_accepted: [],
    offer_rejected: [],
    rejected: [],
    withdrawn: [],
    ghosted: [],
  };
}

describe("applicationsModel", () => {
  it("formats known reminder types with plain labels", () => {
    expect(formatReminderType("follow_up")).toBe("Follow up");
    expect(formatReminderType("interview_prep")).toBe("Interview prep");
    expect(formatReminderType("unknown")).toBe("Reminder");
  });

  it("finds application columns and records across status groups", () => {
    const applications = emptyApplications();
    applications.phone_interview.push({
      ...baseApplication,
      id: 42,
      status: "phone_interview",
    });

    expect(findColumnForApplication(applications, 42)).toBe("phone_interview");
    expect(findColumnForApplication(applications, 7)).toBeNull();
    expect(findColumnForApplication(null, 42)).toBeNull();
    expect(findApplicationById(applications, 42)?.company).toBe("Community Clinic");
    expect(findApplicationById(applications, null)).toBeNull();
  });

  it("reports whether any application is tracked", () => {
    expect(hasAnyApplications(null)).toBe(false);
    expect(hasAnyApplications(emptyApplications())).toBe(false);

    const applications = emptyApplications();
    applications.to_apply.push(baseApplication);
    expect(hasAnyApplications(applications)).toBe(true);
  });

  it("calculates tracker stats without counting saved-only jobs as applied", () => {
    const applications = emptyApplications();
    applications.to_apply.push({ ...baseApplication, id: 1, status: "to_apply" });
    applications.applied.push({ ...baseApplication, id: 2, status: "applied" });
    applications.screening_call.push({
      ...baseApplication,
      id: 3,
      status: "screening_call",
    });
    applications.offer_received.push({
      ...baseApplication,
      id: 4,
      status: "offer_received",
    });
    applications.offer_rejected.push({
      ...baseApplication,
      id: 5,
      status: "offer_rejected",
    });
    applications.rejected.push({ ...baseApplication, id: 6, status: "rejected" });

    expect(getApplicationStats(applications)).toEqual({
      totalApplied: 5,
      interviews: 1,
      offers: 2,
      rejected: 2,
      inProgress: 3,
      interviewRate: 60,
      offerRate: 40,
    });
    expect(getApplicationStats(null)).toBeNull();
  });

  it("builds scheduler application options without status details", () => {
    const applications = emptyApplications();
    applications.applied.push(baseApplication);

    expect(getInterviewSchedulerApplications(applications)).toEqual([
      {
        id: 1,
        job_title: "Care Coordinator",
        company: "Community Clinic",
      },
    ]);
  });

  it("builds a plain next-action review from reminders and active statuses", () => {
    const applications = emptyApplications();
    applications.to_apply.push({
      ...baseApplication,
      id: 2,
      status: "to_apply",
      applied_at: null,
    });
    applications.phone_interview.push({
      ...baseApplication,
      id: 3,
      status: "phone_interview",
      last_contact: "2026-06-18T12:00:00Z",
    });
    applications.offer_received.push({
      ...baseApplication,
      id: 4,
      status: "offer_received",
    });
    const reminders: PendingReminder[] = [
      {
        id: 1,
        application_id: 3,
        job_title: "Care Coordinator",
        company: "Community Clinic",
        reminder_type: "interview_prep",
        reminder_time: "2026-06-20T12:00:00Z",
      },
    ];

    expect(
      getApplicationReviewSummary(applications, reminders, new Date("2026-06-19T12:00:00Z")).actions,
    ).toEqual([
      expect.objectContaining({ id: "reminder-1", kind: "reminders", priority: "high" }),
      expect.objectContaining({ id: "interview-3", kind: "interviews", priority: "medium" }),
      expect.objectContaining({ id: "offer-4", kind: "offers", priority: "medium" }),
      expect.objectContaining({ id: "to-apply-2", kind: "to_apply", priority: "low" }),
      expect.objectContaining({ id: "weekly-review", kind: "weekly_review", priority: "low" }),
    ]);
  });

  it("builds a bounded mission from concrete opportunity actions", () => {
    const applications = emptyApplications();
    applications.applied.push({
      ...baseApplication,
      id: 30,
      job_title: "Office Assistant",
      company: "Example Services",
      applied_at: "2026-05-01T12:00:00Z",
    });
    applications.phone_interview.push(
      {
        ...baseApplication,
        id: 40,
        job_title: "Support Specialist",
        company: "Northwind",
        status: "phone_interview",
      },
      {
        ...baseApplication,
        id: 41,
        job_title: "Operations Coordinator",
        company: "Contoso",
        status: "phone_interview",
      },
    );
    applications.offer_received.push({
      ...baseApplication,
      id: 50,
      job_title: "Program Assistant",
      company: "Fabrikam",
      status: "offer_received",
    });
    applications.to_apply.push(
      {
        ...baseApplication,
        id: 60,
        job_title: "Office Manager",
        company: "Adventure Works",
        status: "to_apply",
      },
      {
        ...baseApplication,
        id: 61,
        job_title: "Customer Advocate",
        company: "Tailspin",
        status: "to_apply",
      },
    );
    const reminders: PendingReminder[] = [
      {
        id: 21,
        application_id: 41,
        job_title: "Operations Coordinator",
        company: "Contoso",
        reminder_type: "follow_up",
        reminder_time: "2026-06-20T12:00:00Z",
      },
      {
        id: 20,
        application_id: 40,
        job_title: "Support Specialist",
        company: "Northwind",
        reminder_type: "interview_prep",
        reminder_time: "2026-06-19T12:00:00Z",
      },
    ];

    const mission = getApplicationReviewSummary(
      applications,
      reminders,
      new Date("2026-06-19T12:00:00Z"),
    );

    expect(mission.actions).toHaveLength(7);
    expect(new Set(mission.actions.map((action) => action.id)).size).toBe(7);
    expect(mission.actions[0]).toEqual(
      expect.objectContaining({
        id: "reminder-20",
        kind: "reminders",
        reminderId: 20,
        applicationId: 40,
        description: expect.stringContaining("Support Specialist at Northwind"),
      }),
    );
    expect(mission.actions).toContainEqual(
      expect.objectContaining({
        id: "no-response-30",
        kind: "no_response",
        applicationId: 30,
        description: expect.stringContaining("Office Assistant at Example Services"),
      }),
    );
  });

  it("adds review handoffs so next actions explain what to do after the click", () => {
    const applications = emptyApplications();
    applications.to_apply.push({
      ...baseApplication,
      id: 2,
      status: "to_apply",
      applied_at: null,
    });
    applications.offer_received.push({
      ...baseApplication,
      id: 3,
      status: "offer_received",
    });

    const actions = getApplicationReviewSummary(applications, []).actions;

    expect(actions).toContainEqual(
      expect.objectContaining({
        kind: "to_apply",
        handoff: expect.objectContaining({
          label: "Application tracking",
          description: expect.stringContaining("status and notes"),
        }),
      }),
    );
    expect(actions).toContainEqual(
      expect.objectContaining({
        kind: "offers",
        handoff: expect.objectContaining({
          label: "Offer and pay review",
          description: expect.stringContaining("written offer"),
        }),
      }),
    );
    expect(actions).toContainEqual(
      expect.objectContaining({
        kind: "weekly_review",
        title: "Review this week's plan",
        handoff: expect.objectContaining({
          label: "Job-search plan",
          description: expect.stringContaining("stop rules"),
        }),
      }),
    );
  });

  it("flags quiet roles only after the no-response review window", () => {
    const applications = emptyApplications();
    applications.applied.push({
      ...baseApplication,
      id: 2,
      applied_at: "2026-06-05T12:00:00Z",
      last_contact: null,
    });
    applications.technical_interview.push({
      ...baseApplication,
      id: 3,
      status: "technical_interview",
      applied_at: "2026-05-01T12:00:00Z",
      last_contact: "2026-06-18T12:00:00Z",
    });

    expect(
      getApplicationReviewSummary(applications, [], new Date("2026-06-19T12:00:00Z")).actions,
    ).toContainEqual(
      expect.objectContaining({
        kind: "no_response",
        priority: "high",
        applicationId: 2,
      }),
    );
  });

  it("shows a steady state when nothing needs attention", () => {
    const applications = emptyApplications();
    applications.applied.push({
      ...baseApplication,
      applied_at: "2026-06-18T12:00:00Z",
    });

    expect(
      getApplicationReviewSummary(applications, [], new Date("2026-06-19T12:00:00Z")),
    ).toEqual({
      title: "What to focus on next",
      description: "Choose from these local actions; source checks stay behind an explicit click.",
      actions: [
        {
          id: "review-outcomes",
          handoff: {
            description: "Use weekly review when changing sources, lanes, pacing, or stop rules.",
            label: "Job-search plan",
          },
          kind: "steady",
          priority: "low",
          title: "Review completed outcomes",
          description: "Review closed roles and outcomes before changing your search plan.",
        },
        ...routineReviewActions,
      ],
    });
  });

  it("guides empty trackers toward importing or saving one job", () => {
    expect(getApplicationReviewSummary(emptyApplications(), [])).toEqual({
      title: "What to focus on next",
      description: "Choose from these local actions; source checks stay behind an explicit click.",
      actions: [
        {
          id: "add-job",
          handoff: {
            description: "Confirm status and notes, then decide whether to apply, skip, or close the role.",
            label: "Application tracking",
          },
          kind: "to_apply",
          priority: "low",
          title: "Add a job to track",
          description: "Save, paste, or import one job to start an opportunity case.",
        },
        ...routineReviewActions,
      ],
    });
  });
});
