import {
  sanitizeLinkedInWorkbenchTextForStorage,
  sanitizeLinkedInWorkbenchUrl,
} from "../../../features/linkedin-workbench/linkedinWorkbenchPolicy";
import { isSafeExternalHttpsUrl } from "../../mocks/externalUrlSafety";
import {
  APPLICATION_STATUS_KEYS,
  cloneApplications,
  getArg,
  getNextId,
} from "../../mocks/handlers/commandHelpers";
import type {
  MockApplications,
  MockApplicationStatus,
  MockJob,
  MockPendingReminder,
} from "../../mocks/handlers/types";

const LINKEDIN_WORKBENCH_DEFAULT_TITLE = "LinkedIn application";
const LINKEDIN_WORKBENCH_DEFAULT_COMPANY = "Company needs details";
const LINKEDIN_WORKBENCH_JOBS_URL = "https://www.linkedin.com/jobs/";
const LINKEDIN_WORKBENCH_APPLIED_URL =
  "https://www.linkedin.com/jobs-tracker/?stage=applied";

type MockLinkedInWorkbenchEventType =
  | "applied"
  | "saved"
  | "tracking"
  | "rejected"
  | "interview"
  | "follow_up"
  | "reminder"
  | "note"
  | "not_interested";

interface MockLinkedInWorkbenchState {
  jobs: MockJob[];
  applications: MockApplications;
  pendingReminders: MockPendingReminder[];
  reviewed: boolean;
}

interface MockLinkedInWorkbenchValue {
  jobId: number;
  jobHash: string;
  applicationId: number | null;
  status: string;
  needsDetails: boolean;
  savedAsBookmark: boolean;
  hidden: boolean;
}

interface MockLinkedInWorkbenchResult {
  state: MockLinkedInWorkbenchState;
  value:
    | MockLinkedInWorkbenchValue
    | boolean
    | "reviewed"
    | "review_required";
}

export function handleMockLinkedInWorkbenchCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockLinkedInWorkbenchState,
): MockLinkedInWorkbenchResult {
  if (command === "get_linkedin_workbench_review_status") {
    return { state, value: state.reviewed ? "reviewed" : "review_required" };
  }
  if (command === "review_linkedin_workbench") {
    return { state: { ...state, reviewed: true }, value: true };
  }
  if (command === "revoke_linkedin_workbench_review") {
    return { state: { ...state, reviewed: false }, value: state.reviewed };
  }
  if (command !== "record_linkedin_workbench_event") {
    throw new Error(`Unsupported LinkedIn Workbench command: ${command}`);
  }
  if (!state.reviewed) {
    throw new Error("Review LinkedIn Workbench before recording work.");
  }
  let { jobs, applications, pendingReminders } = state;
  const input = getRecordArg(args, "input");
  const eventType = getLinkedInWorkbenchEventType(input.eventType);
  const title =
    trimMockWorkbenchText(input.title, 500) ?? LINKEDIN_WORKBENCH_DEFAULT_TITLE;
  const company =
    trimMockWorkbenchText(input.company, 200) ?? LINKEDIN_WORKBENCH_DEFAULT_COMPANY;
  const notes = trimMockWorkbenchText(input.notes, 5_000);
  const sanitizedNotes =
    notes === undefined ? undefined : sanitizeLinkedInWorkbenchTextForStorage(notes);
  const rawUrl = trimMockWorkbenchText(input.url, 2_000);
  const url = rawUrl
    ? canonicalizeMockWorkbenchUrl(rawUrl)
    : eventType === "applied"
      ? LINKEDIN_WORKBENCH_APPLIED_URL
      : LINKEDIN_WORKBENCH_JOBS_URL;
  const needsDetails =
    title === LINKEDIN_WORKBENCH_DEFAULT_TITLE ||
    company === LINKEDIN_WORKBENCH_DEFAULT_COMPANY ||
    !rawUrl;
  const existingJob =
    rawUrl !== undefined ? jobs.find((job) => job.url === url) : undefined;
  const jobId = existingJob?.id ?? getNextId(jobs);
  const jobHash = existingJob?.hash ?? mockLinkedInWorkbenchHash(url, jobId, rawUrl);
  const shouldBookmark = [
    "applied",
    "saved",
    "tracking",
    "interview",
    "follow_up",
    "reminder",
  ].includes(eventType);
  const hidden = eventType === "not_interested";
  const now = new Date().toISOString();
  const job: MockJob = {
    ...(existingJob ?? {
      id: jobId,
      hash: jobHash,
      title,
      company,
      location: "LinkedIn",
      description: "Created from a user-controlled LinkedIn Workbench action.",
      url,
      source: "linkedin",
      salary_min: 0,
      salary_max: 0,
      remote: false,
      score: 0.5,
      hidden: false,
      bookmarked: false,
      notes: null,
      created_at: now,
    }),
    id: jobId,
    hash: jobHash,
    title,
    company,
    url,
    source: "linkedin",
    hidden: existingJob?.hidden === true || hidden,
    bookmarked: existingJob?.bookmarked === true || shouldBookmark,
    notes: sanitizedNotes ?? existingJob?.notes ?? null,
  };

  jobs = existingJob
    ? jobs.map((candidate) => (candidate.id === jobId ? job : candidate))
    : [job, ...jobs];

  const applicationUpdate = [
    "applied",
    "tracking",
    "rejected",
    "interview",
    "follow_up",
    "reminder",
  ].includes(eventType)
    ? upsertMockLinkedInApplication(
        applications,
        jobHash,
        title,
        company,
        mockLinkedInWorkbenchApplicationStatus(eventType),
        now,
        eventType === "follow_up",
      )
    : null;
  const applicationId = applicationUpdate?.id ?? null;
  applications = applicationUpdate?.applications ?? applications;

  if (eventType === "reminder" && applicationId !== null) {
    pendingReminders = [
      ...pendingReminders,
      {
        id: getNextId(pendingReminders),
        application_id: applicationId,
        reminder_type: "follow_up",
        reminder_time: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString(),
        message: "Review this LinkedIn job in JobSentinel",
        job_hash: jobHash,
        job_title: title,
        company,
      },
    ];
  }

  return {
    state: { jobs, applications, pendingReminders, reviewed: state.reviewed },
    value: {
      jobId,
      jobHash,
      applicationId,
      status: mockLinkedInWorkbenchStatus(eventType),
      needsDetails,
      savedAsBookmark: shouldBookmark,
      hidden,
    },
  };
}

function getRecordArg(
  args: Record<string, unknown> | undefined,
  key: string,
): Record<string, unknown> {
  const value = getArg(args, key);
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function getLinkedInWorkbenchEventType(value: unknown): MockLinkedInWorkbenchEventType {
  if (
    typeof value === "string" &&
    [
      "applied",
      "saved",
      "tracking",
      "rejected",
      "interview",
      "follow_up",
      "reminder",
      "note",
      "not_interested",
    ].includes(value)
  ) {
    return value as MockLinkedInWorkbenchEventType;
  }

  throw new Error("Unsupported LinkedIn workbench action");
}

function trimMockWorkbenchText(value: unknown, limit: number): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }

  const trimmed = value.trim();
  return trimmed ? [...trimmed].slice(0, limit).join("") : undefined;
}

function canonicalizeMockWorkbenchUrl(rawUrl: string): string {
  if (!isSafeExternalHttpsUrl(rawUrl)) {
    throw new Error("This LinkedIn job link is not safe to save");
  }

  return sanitizeLinkedInWorkbenchUrl(rawUrl);
}

function mockLinkedInWorkbenchHash(
  url: string,
  jobId: number,
  rawUrl: string | undefined,
): string {
  return rawUrl
    ? `linkedin-workbench:${url}`
    : `linkedin-workbench-draft:${jobId}`;
}

function upsertMockLinkedInApplication(
  applications: MockApplications,
  jobHash: string,
  title: string,
  company: string,
  status: MockApplicationStatus,
  now: string,
  markContact: boolean,
): { applications: MockApplications; id: number } {
  const existing = APPLICATION_STATUS_KEYS
    .flatMap((key) => applications[key])
    .find((application) => application.job_hash === jobHash);
  const id =
    existing?.id ??
    APPLICATION_STATUS_KEYS
      .flatMap((key) => applications[key])
      .reduce((max, application) => Math.max(max, application.id), 0) + 1;

  const nextApplications = APPLICATION_STATUS_KEYS.reduce<MockApplications>(
    (next, key) => {
      next[key] = applications[key].filter(
        (application) => application.job_hash !== jobHash,
      );
      return next;
    },
    cloneApplications(applications),
  );
  nextApplications[status] = [
    ...nextApplications[status],
    {
      id,
      job_hash: jobHash,
      job_title: title,
      company,
      status,
      applied_at: status === "applied" ? existing?.applied_at ?? now : null,
      notes: existing?.notes ?? null,
      last_contact: markContact ? now : existing?.last_contact ?? null,
    },
  ];

  return { applications: nextApplications, id };
}

function mockLinkedInWorkbenchApplicationStatus(
  eventType: MockLinkedInWorkbenchEventType,
): MockApplicationStatus {
  switch (eventType) {
    case "applied":
      return "applied";
    case "rejected":
      return "rejected";
    case "interview":
      return "phone_interview";
    case "tracking":
    case "follow_up":
    case "reminder":
    case "saved":
    case "note":
    case "not_interested":
      return "to_apply";
  }
}

function mockLinkedInWorkbenchStatus(
  eventType: MockLinkedInWorkbenchEventType,
): string {
  switch (eventType) {
    case "applied":
      return "applied";
    case "saved":
      return "saved";
    case "tracking":
      return "tracking";
    case "rejected":
      return "rejected";
    case "interview":
      return "interview";
    case "follow_up":
      return "follow_up";
    case "reminder":
      return "reminder";
    case "note":
      return "noted";
    case "not_interested":
      return "not_interested";
  }
}
