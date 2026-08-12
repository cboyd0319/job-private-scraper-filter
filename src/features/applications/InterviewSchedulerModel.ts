/** Defines Interview Scheduler renderer contracts and formatting helpers. */

import type { RenderCompanyResearch } from "../../shared/companyResearch";

export interface Interview {
  id: number;
  application_id: number;
  interview_type: string;
  scheduled_at: string;
  duration_minutes: number;
  location: string | null;
  interviewer_name: string | null;
  interviewer_title: string | null;
  notes: string | null;
  completed: boolean;
  outcome: string | null;
  post_interview_notes: string | null;
  job_hash: string;
  job_title: string;
  company: string;
}

export type InterviewTab = "upcoming" | "past";

export interface Application {
  id: number;
  job_title: string;
  company: string;
}

export interface InterviewSchedulerProps {
  onClose: () => void;
  applications?: Application[];
  renderCompanyResearch?: RenderCompanyResearch;
}

export interface FollowUpReminder {
  interviewId: number;
  thankYouSent: boolean;
  sentAt: string | null;
}

export interface PrepChecklistItem {
  itemId: string;
  completed: boolean;
  completedAt: string | null;
}

export interface PrepProgress {
  [itemId: string]: boolean;
}

export function formatFollowUpSentDate(sentAt?: string | null): string {
  return sentAt ? new Date(sentAt).toLocaleDateString() : "";
}

export const INTERVIEW_TYPES: Array<{ value: string; label: string }> = [
  { value: "phone", label: "Phone Screen" },
  { value: "screening", label: "Screening Call" },
  { value: "technical", label: "Skills Interview" },
  { value: "behavioral", label: "Behavioral Interview" },
  { value: "onsite", label: "Onsite Interview" },
  { value: "final", label: "Final Round" },
  { value: "other", label: "Other" },
];

export const TYPE_COLORS: Record<string, string> = {
  phone: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  screening:
    "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  technical:
    "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300",
  behavioral:
    "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  onsite: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  final: "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
  other:
    "bg-surface-100 text-surface-800 dark:bg-surface-700 dark:text-surface-300",
};

export const OUTCOME_COLORS: Record<string, string> = {
  passed: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  pending: "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
  failed:
    "bg-surface-100 text-surface-800 dark:bg-surface-700 dark:text-surface-200",
};

const OUTCOME_LABELS: Record<string, string> = {
  passed: "Went well",
  pending: "Not sure yet",
  failed: "Not a fit",
};

export function formatOutcomeLabel(outcome: string): string {
  return OUTCOME_LABELS[outcome] ?? "Not sure yet";
}

export function formatInterviewTypeLabel(interviewType: string): string {
  return (
    INTERVIEW_TYPES.find((type) => type.value === interviewType)?.label ||
    interviewType
  );
}

export const PREP_CHECKLIST = [
  { id: "research", label: "Research company background", icon: "search" },
  { id: "review_jd", label: "Review job description", icon: "doc" },
  { id: "prepare_questions", label: "Prepare questions to ask", icon: "question" },
  { id: "work_examples", label: "Prepare 2-3 examples from past work", icon: "star" },
  { id: "tech_review", label: "Review role requirements", icon: "code" },
];
