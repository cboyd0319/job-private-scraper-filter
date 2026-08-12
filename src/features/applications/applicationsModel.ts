export interface Application {
  id: number;
  job_hash: string;
  job_title: string;
  company: string;
  status: string;
  applied_at: string | null;
  notes: string | null;
  last_contact: string | null;
}

export interface ApplicationsByStatus {
  to_apply: Application[];
  applied: Application[];
  screening_call: Application[];
  phone_interview: Application[];
  technical_interview: Application[];
  onsite_interview: Application[];
  offer_received: Application[];
  offer_accepted: Application[];
  offer_rejected: Application[];
  rejected: Application[];
  withdrawn: Application[];
  ghosted: Application[];
}

export interface PendingReminder {
  id: number;
  application_id: number;
  job_title: string;
  company: string;
  reminder_type: string;
  reminder_time: string;
}

export interface ApplicationStats {
  totalApplied: number;
  interviews: number;
  offers: number;
  rejected: number;
  inProgress: number;
  interviewRate: number;
  offerRate: number;
}

export interface InterviewSchedulerApplication {
  id: number;
  job_title: string;
  company: string;
}

export type ApplicationReviewActionKind =
  | "reminders"
  | "no_response"
  | "interviews"
  | "offers"
  | "to_apply"
  | "weekly_review"
  | "source_review"
  | "steady";

export type ApplicationReviewPriority = "high" | "medium" | "low";

export interface ApplicationReviewAction {
  id: string;
  kind: ApplicationReviewActionKind;
  priority: ApplicationReviewPriority;
  title: string;
  description: string;
  applicationId?: number;
  reminderId?: number;
  handoff?: ApplicationReviewHandoff;
}

export interface ApplicationReviewHandoff {
  label: string;
  description: string;
}

export interface ApplicationReviewSummary {
  title: string;
  description: string;
  actions: ApplicationReviewAction[];
}

const DAY_MS = 24 * 60 * 60 * 1000;
const NO_RESPONSE_REVIEW_DAYS = 14;

const REMINDER_TYPE_LABELS: Record<string, string> = {
  follow_up: "Follow up",
  interview_prep: "Interview prep",
  deadline: "Deadline",
  offer_review: "Offer review",
  custom: "Reminder",
};

export const STATUS_COLUMNS = [
  { key: "to_apply", label: "To Apply", color: "bg-surface-500" },
  { key: "applied", label: "Applied", color: "bg-blue-500" },
  { key: "screening_call", label: "Screening Call", color: "bg-purple-500" },
  { key: "phone_interview", label: "Phone Interview", color: "bg-violet-500" },
  { key: "technical_interview", label: "Skills Interview", color: "bg-indigo-500" },
  { key: "onsite_interview", label: "Onsite Interview", color: "bg-cyan-500" },
  { key: "offer_received", label: "Offer Received", color: "bg-success" },
  { key: "offer_accepted", label: "Offer Accepted", color: "bg-emerald-500" },
  { key: "offer_rejected", label: "Offer Declined", color: "bg-orange-500" },
  { key: "rejected", label: "Not Selected", color: "bg-danger" },
  { key: "withdrawn", label: "Withdrawn", color: "bg-amber-500" },
  { key: "ghosted", label: "No Response", color: "bg-surface-400" },
] as const;

export type StatusKey = (typeof STATUS_COLUMNS)[number]["key"];

export function formatReminderType(type: string): string {
  return REMINDER_TYPE_LABELS[type] ?? "Reminder";
}

export function findColumnForApplication(
  applications: ApplicationsByStatus | null,
  appId: number,
): StatusKey | null {
  if (!applications) return null;
  for (const column of STATUS_COLUMNS) {
    if (applications[column.key].some((application) => application.id === appId)) {
      return column.key;
    }
  }
  return null;
}

export function findApplicationById(
  applications: ApplicationsByStatus | null,
  appId: number | null,
): Application | null {
  if (appId === null || !applications) return null;
  for (const column of STATUS_COLUMNS) {
    const application = applications[column.key].find((candidate) => candidate.id === appId);
    if (application) return application;
  }
  return null;
}

export function hasAnyApplications(applications: ApplicationsByStatus | null): boolean {
  if (!applications) return false;
  return STATUS_COLUMNS.some((column) => applications[column.key].length > 0);
}

export function getApplicationStats(
  applications: ApplicationsByStatus | null,
): ApplicationStats | null {
  if (!applications) return null;

  const totalApplied =
    applications.applied.length +
    applications.screening_call.length +
    applications.phone_interview.length +
    applications.technical_interview.length +
    applications.onsite_interview.length +
    applications.offer_received.length +
    applications.offer_accepted.length +
    applications.offer_rejected.length +
    applications.rejected.length +
    applications.withdrawn.length +
    applications.ghosted.length;

  const interviews =
    applications.screening_call.length +
    applications.phone_interview.length +
    applications.technical_interview.length +
    applications.onsite_interview.length;

  const interviewsPlusOffers =
    interviews +
    applications.offer_received.length +
    applications.offer_accepted.length +
    applications.offer_rejected.length;

  const offers =
    applications.offer_received.length +
    applications.offer_accepted.length +
    applications.offer_rejected.length;

  const rejected = applications.rejected.length + applications.offer_rejected.length;

  const inProgress = applications.applied.length + interviews + applications.offer_received.length;

  const interviewRate =
    totalApplied > 0 ? Math.round((interviewsPlusOffers / totalApplied) * 100) : 0;

  const offerRate = totalApplied > 0 ? Math.round((offers / totalApplied) * 100) : 0;

  return {
    totalApplied,
    interviews,
    offers,
    rejected,
    inProgress,
    interviewRate,
    offerRate,
  };
}

export function getInterviewSchedulerApplications(
  applications: ApplicationsByStatus,
): InterviewSchedulerApplication[] {
  return Object.values(applications)
    .flat()
    .map((application) => ({
      id: application.id,
      job_title: application.job_title,
      company: application.company,
    }));
}

export function getApplicationReviewSummary(
  applications: ApplicationsByStatus | null,
  reminders: PendingReminder[],
  now = new Date(),
): ApplicationReviewSummary {
  const actions: ApplicationReviewAction[] = [];
  const add = (action: ApplicationReviewAction) => {
    if (actions.length < 7) actions.push(action);
  };

  if (!applications || !hasAnyApplications(applications)) {
    add({
      id: "add-job",
      kind: "to_apply",
      priority: "low",
      title: "Add a job to track",
      description: "Save, paste, or import one job to start an opportunity case.",
      handoff: ACTION_HANDOFFS.to_apply,
    });
  } else {
    for (const reminder of [...reminders].sort(
      (left, right) => left.reminder_time.localeCompare(right.reminder_time) || left.id - right.id,
    )) {
      add({
        id: `reminder-${reminder.id}`,
        kind: "reminders",
        priority: "high",
        title: `Review ${formatReminderType(reminder.reminder_type).toLowerCase()}`,
        description: `${reminder.job_title} at ${reminder.company} has a saved reminder ready for review.`,
        applicationId: reminder.application_id,
        reminderId: reminder.id,
        handoff: ACTION_HANDOFFS.reminders,
      });
    }

    const activeInterviews = [
      ...applications.screening_call,
      ...applications.phone_interview,
      ...applications.technical_interview,
      ...applications.onsite_interview,
    ];
    const noResponses = [
      ...applications.applied,
      ...applications.phone_interview,
      ...applications.technical_interview,
      ...applications.onsite_interview,
    ]
      .filter((application) => isNoResponseCandidate(application, now))
      .sort(compareApplicationActivity);
    for (const application of noResponses) {
      add({
        id: `no-response-${application.id}`,
        kind: "no_response",
        priority: "high",
        title: "Choose a quiet-role next step",
        description: `${application.job_title} at ${application.company} has been quiet for at least 14 days.`,
        applicationId: application.id,
        handoff: ACTION_HANDOFFS.no_response,
      });
    }
    for (const application of [...activeInterviews].sort(compareApplicationId)) {
      add({
        id: `interview-${application.id}`,
        kind: "interviews",
        priority: "medium",
        title: "Prepare or follow up",
        description: `${application.job_title} at ${application.company} has an interview-stage action.`,
        applicationId: application.id,
        handoff: ACTION_HANDOFFS.interviews,
      });
    }
    for (const application of [...applications.offer_received].sort(compareApplicationId)) {
      add({
        id: `offer-${application.id}`,
        kind: "offers",
        priority: "medium",
        title: "Review this offer",
        description: `Review the written facts, deadline, and options for ${application.job_title} at ${application.company}.`,
        applicationId: application.id,
        handoff: ACTION_HANDOFFS.offers,
      });
    }
    for (const application of [...applications.to_apply].sort(compareApplicationId)) {
      add({
        id: `to-apply-${application.id}`,
        kind: "to_apply",
        priority: "low",
        title: "Review this tracked role",
        description: `Review the status and notes for ${application.job_title} at ${application.company}.`,
        applicationId: application.id,
        handoff: ACTION_HANDOFFS.to_apply,
      });
    }
    if (actions.length === 0) {
      add({
        id: "review-outcomes",
        kind: "steady",
        priority: "low",
        title: "Review completed outcomes",
        description: "Review closed roles and outcomes before changing your search plan.",
        handoff: ACTION_HANDOFFS.steady,
      });
    }
  }

  add({
    id: "weekly-review",
    kind: "weekly_review",
    priority: "low",
    title: "Review this week's plan",
    description: "Compare sources, quiet roles, saved roles, responses, and interviews before changing pace.",
    handoff: ACTION_HANDOFFS.weekly_review,
  });
  if (actions.length < 3) {
    add({
      id: "source-review",
      kind: "source_review",
      priority: "low",
      title: "Review job sources",
      description: "Review enabled sources; no source is contacted until you choose a source action.",
      handoff: ACTION_HANDOFFS.source_review,
    });
  }

  const highPriorityCount = actions.filter((action) => action.priority === "high").length;

  return {
    title: highPriorityCount > 0 ? "Do these first" : "What to focus on next",
    description:
      highPriorityCount > 0
        ? "Start with time-sensitive follow-ups and quiet roles before spending energy elsewhere."
        : "Choose from these local actions; source checks stay behind an explicit click.",
    actions,
  };
}

function isNoResponseCandidate(application: Application, now: Date): boolean {
  const latestActivity = parseApplicationDate(application.last_contact ?? application.applied_at);
  if (!latestActivity) return false;

  return now.getTime() - latestActivity.getTime() >= NO_RESPONSE_REVIEW_DAYS * DAY_MS;
}

function parseApplicationDate(value: string | null): Date | null {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function compareApplicationId(left: Application, right: Application): number {
  return left.id - right.id;
}

function compareApplicationActivity(left: Application, right: Application): number {
  const leftTime = parseApplicationDate(left.last_contact ?? left.applied_at)?.getTime() ?? 0;
  const rightTime = parseApplicationDate(right.last_contact ?? right.applied_at)?.getTime() ?? 0;
  return leftTime - rightTime || compareApplicationId(left, right);
}

const ACTION_HANDOFFS: Record<ApplicationReviewActionKind, ApplicationReviewHandoff> = {
  reminders: {
    label: "Networking outreach",
    description: "Use recruiter replies, referral asks, warm follow ups, or thank-you notes only after review.",
  },
  no_response: {
    label: "Application tracking",
    description: "Decide whether to follow up once, mark No Response, close, skip, or deprioritize.",
  },
  interviews: {
    label: "Interview prep",
    description: "Build story prompts, questions, logistics notes, and follow-up reminders.",
  },
  offers: {
    label: "Offer and pay review",
    description: "Separate written offer facts, deadline, benefits, commute, risks, and counter notes.",
  },
  to_apply: {
    label: "Application tracking",
    description: "Confirm status and notes, then decide whether to apply, skip, or close the role.",
  },
  weekly_review: {
    label: "Job-search plan",
    description: "Replan lanes, sources, pacing, and stop rules from tracker evidence.",
  },
  source_review: {
    label: "Job sources",
    description: "Review enabled sources and start any connectivity-required check yourself.",
  },
  steady: {
    label: "Job-search plan",
    description: "Use weekly review when changing sources, lanes, pacing, or stop rules.",
  },
};
