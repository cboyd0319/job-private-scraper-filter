/** Implements browser-development application and interview commands. */

import {
  mockApplicationStats,
  mockStatistics,
} from "../../mocks/data";
import {
  APPLICATION_STATUS_KEYS,
  getArg,
  getNumericArg,
} from "../../mocks/handlers/commandHelpers";
import type {
  MockApplication,
  MockApplications,
  MockApplicationStatus,
  MockInterview,
  MockJob,
  MockPendingReminder,
} from "../../mocks/handlers/types";

export interface MockApplicationsCommandState {
  jobs: MockJob[];
  applications: MockApplications;
  pendingReminders: MockPendingReminder[];
  interviews: MockInterview[];
}

export interface MockApplicationsCommandResult {
  handled: boolean;
  shouldSave: boolean;
  state: MockApplicationsCommandState;
  value: unknown;
}

export function handleMockApplicationsCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockApplicationsCommandState,
): MockApplicationsCommandResult {
  switch (command) {
    case "get_applications_kanban":
      return withoutSave(state, state.applications);

    case "create_application":
      return createApplication(args, state);

    case "update_application_status":
      return updateApplicationStatus(args, state);

    case "add_application_notes":
      return addApplicationNotes(args, state);

    case "get_pending_reminders":
      return withoutSave(state, state.pendingReminders);

    case "complete_reminder":
      return {
        handled: true,
        shouldSave: true,
        state: {
          ...state,
          pendingReminders: state.pendingReminders.filter(
            (reminder) => reminder.id !== getNumericArg(args, "reminderId"),
          ),
        },
        value: undefined,
      };

    case "detect_ghosted_applications":
      return withoutSave(state, 0);

    case "get_application_stats":
      return withoutSave(state, mockApplicationStats);

    case "get_jobs_by_source":
      return withoutSave(
        state,
        Object.entries(mockStatistics.by_source).map(([source, count]) => ({
          source,
          count,
        })),
      );

    case "get_salary_distribution":
      return withoutSave(
        state,
        [
          {
            range: "$40k-$60k",
            count: state.jobs.filter((job) => job.salary_min < 60000).length,
          },
          {
            range: "$60k-$80k",
            count: state.jobs.filter(
              (job) => job.salary_min >= 60000 && job.salary_min < 80000,
            ).length,
          },
          {
            range: "$80k-$100k",
            count: state.jobs.filter(
              (job) => job.salary_min >= 80000 && job.salary_min < 100000,
            ).length,
          },
          {
            range: "$100k+",
            count: state.jobs.filter((job) => job.salary_min >= 100000).length,
          },
        ].filter((bucket) => bucket.count > 0),
      );

    case "get_upcoming_interviews": {
      const now = Date.now();
      const inThirtyDays = now + 30 * 24 * 60 * 60 * 1000;
      return withoutSave(
        state,
        state.interviews
          .filter(
            (interview) =>
              !interview.completed && Date.parse(interview.scheduled_at) <= inThirtyDays,
          )
          .sort(
            (left, right) =>
              Math.abs(Date.parse(left.scheduled_at) - now) -
              Math.abs(Date.parse(right.scheduled_at) - now),
          ),
      );
    }

    case "get_past_interviews":
      return withoutSave(
        state,
        state.interviews
          .filter((interview) => interview.completed)
          .sort(
            (left, right) =>
              Date.parse(right.scheduled_at) - Date.parse(left.scheduled_at),
          ),
      );

    case "schedule_interview":
      return scheduleInterview(args, state);

    case "complete_interview":
      return {
        handled: true,
        shouldSave: true,
        state: {
          ...state,
          interviews: state.interviews.map((interview): MockInterview =>
            interview.id === getArg(args, "interviewId")
              ? {
                  ...interview,
                  completed: true,
                  outcome: getArg(args, "outcome") as string,
                  post_interview_notes: (getArg(args, "notes") as string | null) ?? null,
                }
              : interview,
          ),
        },
        value: undefined,
      };

    case "delete_interview":
      return {
        handled: true,
        shouldSave: true,
        state: {
          ...state,
          interviews: state.interviews.filter(
            (interview) => interview.id !== getArg(args, "interviewId"),
          ),
        },
        value: undefined,
      };

    case "find_duplicates":
      return withoutSave(state, []);

    case "merge_duplicates":
      return withoutSave(state, undefined);

    default:
      return { handled: false, shouldSave: false, state, value: undefined };
  }
}

function withoutSave(
  state: MockApplicationsCommandState,
  value: unknown,
): MockApplicationsCommandResult {
  return { handled: true, shouldSave: false, state, value };
}

function createApplication(
  args: Record<string, unknown> | undefined,
  state: MockApplicationsCommandState,
): MockApplicationsCommandResult {
  const jobHash = getArg(args, "jobHash") ?? getArg(args, "job_hash");
  const job = state.jobs.find((candidate) => candidate.hash === jobHash);
  const nextId =
    Math.max(
      0,
      ...APPLICATION_STATUS_KEYS.flatMap((status) =>
        state.applications[status].map((application) => application.id),
      ),
    ) + 1;
  const applications = {
    ...state.applications,
    to_apply: [
      ...state.applications.to_apply,
      {
        id: nextId,
        job_hash: typeof jobHash === "string" ? jobHash : `mock-${nextId}`,
        job_title: job?.title ?? "Tracked Job",
        company: job?.company ?? "Mock Company",
        status: "to_apply",
        applied_at: null,
        notes: null,
        last_contact: null,
      },
    ],
  };

  return {
    handled: true,
    shouldSave: true,
    state: { ...state, applications },
    value: nextId,
  };
}

function updateApplicationStatus(
  args: Record<string, unknown> | undefined,
  state: MockApplicationsCommandState,
): MockApplicationsCommandResult {
  const applicationId = getNumericArg(args, "applicationId");
  const status = getArg(args, "status");
  if (applicationId === undefined || typeof status !== "string") {
    return withoutSave(state, undefined);
  }

  return {
    handled: true,
    shouldSave: true,
    state: {
      ...state,
      applications: moveApplicationStatus(
        state.applications,
        applicationId,
        status,
      ),
    },
    value: undefined,
  };
}

function addApplicationNotes(
  args: Record<string, unknown> | undefined,
  state: MockApplicationsCommandState,
): MockApplicationsCommandResult {
  const applicationId = getNumericArg(args, "applicationId");
  const notes = getArg(args, "notes");
  if (applicationId === undefined) {
    return withoutSave(state, undefined);
  }

  return {
    handled: true,
    shouldSave: true,
    state: {
      ...state,
      applications: updateApplication(
        state.applications,
        applicationId,
        (application) => ({
          ...application,
          notes: typeof notes === "string" ? notes : null,
        }),
      ),
    },
    value: undefined,
  };
}

function scheduleInterview(
  args: Record<string, unknown> | undefined,
  state: MockApplicationsCommandState,
): MockApplicationsCommandResult {
  const id = Math.max(...state.interviews.map((interview) => interview.id), 0) + 1;
  const applicationId = getArg(args, "applicationId") as number;
  const application = Object.values(state.applications)
    .flat()
    .find((candidate) => candidate.id === applicationId);
  const interview: MockInterview = {
    id,
    application_id: applicationId,
    job_hash: application?.job_hash ?? `mock-${applicationId}`,
    interview_type: getArg(args, "interviewType") as string,
    scheduled_at: getArg(args, "scheduledAt") as string,
    duration_minutes: getArg(args, "durationMinutes") as number,
    location: (getArg(args, "location") as string) || null,
    interviewer_name: (getArg(args, "interviewerName") as string) || null,
    interviewer_title: (getArg(args, "interviewerTitle") as string) || null,
    notes: (getArg(args, "notes") as string) || null,
    completed: false,
    outcome: null,
    post_interview_notes: null,
    job_title: application?.job_title ?? "Mock Job",
    company: application?.company ?? "Mock Company",
  };

  return {
    handled: true,
    shouldSave: true,
    state: { ...state, interviews: [...state.interviews, interview] },
    value: id,
  };
}

function findApplication(
  applications: MockApplications,
  applicationId: number,
): { status: MockApplicationStatus; application: MockApplication } | null {
  for (const status of APPLICATION_STATUS_KEYS) {
    const application = applications[status].find((app) => app.id === applicationId);
    if (application) return { status, application };
  }
  return null;
}

function updateApplication(
  applications: MockApplications,
  applicationId: number,
  updater: (application: MockApplication) => MockApplication,
): MockApplications {
  return APPLICATION_STATUS_KEYS.reduce((acc, status) => {
    acc[status] = applications[status].map((application) =>
      application.id === applicationId ? updater(application) : application,
    );
    return acc;
  }, {} as MockApplications);
}

function moveApplicationStatus(
  applications: MockApplications,
  applicationId: number,
  status: string,
): MockApplications {
  if (!APPLICATION_STATUS_KEYS.includes(status as MockApplicationStatus)) {
    return applications;
  }

  const current = findApplication(applications, applicationId);
  if (!current) return applications;

  const nextStatus = status as MockApplicationStatus;
  const updated = APPLICATION_STATUS_KEYS.reduce((acc, key) => {
    acc[key] = applications[key].filter(
      (application) => application.id !== applicationId,
    );
    return acc;
  }, {} as MockApplications);
  updated[nextStatus] = [
    ...updated[nextStatus],
    { ...current.application, status: nextStatus },
  ];
  return updated;
}
