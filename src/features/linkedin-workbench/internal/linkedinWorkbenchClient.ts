import { invoke } from "../../../platform/tauri";
import type { LinkedInWorkbenchEventType } from "../linkedinWorkbenchPolicy";

export interface LinkedInWorkbenchEventInput {
  eventType: LinkedInWorkbenchEventType;
  title?: string;
  company?: string;
  url?: string;
  notes?: string;
}

export interface LinkedInWorkbenchEventResult {
  jobId: number;
  jobHash: string;
  applicationId: number | null;
  status: string;
  needsDetails: boolean;
  savedAsBookmark: boolean;
  hidden: boolean;
}

export type LinkedInWorkbenchReviewStatus =
  | "reviewed"
  | "review_required"
  | "policy_refresh_required";

export function getLinkedInWorkbenchReviewStatus(): Promise<LinkedInWorkbenchReviewStatus> {
  return invoke<LinkedInWorkbenchReviewStatus>("get_linkedin_workbench_review_status");
}

export function reviewLinkedInWorkbench(): Promise<boolean> {
  return invoke<boolean>("review_linkedin_workbench");
}

export function revokeLinkedInWorkbenchReview(): Promise<boolean> {
  return invoke<boolean>("revoke_linkedin_workbench_review");
}

export function recordLinkedInWorkbenchEvent(
  input: LinkedInWorkbenchEventInput,
): Promise<LinkedInWorkbenchEventResult> {
  return invoke<LinkedInWorkbenchEventResult>("record_linkedin_workbench_event", {
    input,
  });
}
