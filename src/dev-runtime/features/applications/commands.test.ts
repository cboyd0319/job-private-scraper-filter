import { describe, expect, it } from "vitest";
import {
  mockApplications,
  mockJobs,
  mockPendingReminders,
  mockUpcomingInterviews,
} from "../../mocks/data";
import { cloneApplications } from "../../mocks/handlers/commandHelpers";
import { handleMockApplicationsCommand } from "./commands";

function createState() {
  return {
    jobs: mockJobs.map((job) => ({ ...job })),
    applications: cloneApplications(mockApplications),
    pendingReminders: mockPendingReminders.map((reminder) => ({ ...reminder })),
    interviews: mockUpcomingInterviews.map((interview) => ({ ...interview })),
  };
}

describe("Applications mock commands", () => {
  it("returns source totals that match the displayed jobs", () => {
    const state = createState();

    const result = handleMockApplicationsCommand(
      "get_jobs_by_source",
      undefined,
      state,
    );

    expect(result.value).toEqual([
      { source: "linkedin", count: 1 },
      { source: "greenhouse", count: 5 },
      { source: "lever", count: 1 },
      { source: "direct", count: 1 },
    ]);
  });

  it("creates and moves an application through owned state", () => {
    const initial = createState();
    const job = initial.jobs[0];
    const created = handleMockApplicationsCommand(
      "create_application",
      { jobHash: job.hash },
      initial,
    );
    const applicationId = created.value as number;

    expect(created).toMatchObject({ handled: true, shouldSave: true });
    expect(created.state.applications.to_apply).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: applicationId, job_hash: job.hash }),
      ]),
    );

    const moved = handleMockApplicationsCommand(
      "update_application_status",
      { applicationId, status: "applied" },
      created.state,
    );
    expect(moved.state.applications.to_apply).not.toEqual(
      expect.arrayContaining([expect.objectContaining({ id: applicationId })]),
    );
    expect(moved.state.applications.applied).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: applicationId })]),
    );
  });

  it("moves a completed interview and its debrief into past interviews", () => {
    const initial = createState();
    const interviewId = initial.interviews[0].id;
    const completed = handleMockApplicationsCommand(
      "complete_interview",
      {
        interviewId,
        outcome: "passed",
        notes: "Questions asked: How would you prioritize urgent customer requests?",
      },
      initial,
    );

    expect(
      handleMockApplicationsCommand("get_upcoming_interviews", undefined, completed.state).value,
    ).not.toEqual(expect.arrayContaining([expect.objectContaining({ id: interviewId })]));
    expect(
      handleMockApplicationsCommand("get_past_interviews", undefined, completed.state).value,
    ).toEqual([
      expect.objectContaining({
        id: interviewId,
        completed: true,
        outcome: "passed",
        post_interview_notes: "Questions asked: How would you prioritize urgent customer requests?",
      }),
    ]);
  });

  it("matches production interview windows and ordering", () => {
    const state = createState();
    const template = state.interviews[0];
    const day = 24 * 60 * 60 * 1000;
    const now = Date.now();
    state.interviews = [
      { ...template, id: 10, scheduled_at: new Date(now + 31 * day).toISOString() },
      { ...template, id: 11, scheduled_at: new Date(now + 3 * day).toISOString() },
      { ...template, id: 12, scheduled_at: new Date(now - day).toISOString() },
      { ...template, id: 13, completed: true, scheduled_at: new Date(now - 120 * day).toISOString() },
      { ...template, id: 14, completed: true, scheduled_at: new Date(now - 2 * day).toISOString() },
    ];

    expect(
      handleMockApplicationsCommand("get_upcoming_interviews", undefined, state).value,
    ).toEqual([
      expect.objectContaining({ id: 12 }),
      expect.objectContaining({ id: 11 }),
    ]);
    expect(
      handleMockApplicationsCommand("get_past_interviews", undefined, state).value,
    ).toEqual([
      expect.objectContaining({ id: 14 }),
      expect.objectContaining({ id: 13 }),
    ]);
  });

  it("returns an unhandled result for Dashboard commands", () => {
    const state = createState();

    expect(handleMockApplicationsCommand("get_jobs", undefined, state)).toMatchObject(
      { handled: false, shouldSave: false, state },
    );
  });
});
