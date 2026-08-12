import {
  RESTRICTED_AUTHENTICATED_SOURCE_WARNING,
  RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES,
} from "./restrictedSourceTaxonomy";

export type JobSourceAccessModel =
  | "native-public"
  | "native-public-with-local-credential"
  | "native-public-feed"
  | "public-community"
  | "employer-career-system"
  | "restricted-user-gated"
  | "review-required";

export type JobSourceImplementationStatus =
  | "supported"
  | "candidate"
  | "restricted"
  | "manual-only"
  | "research";

export type JobSourceTechnicalAccess =
  | "public-unauthenticated"
  | "public-local-credential"
  | "authenticated-user-session"
  | "unknown-review-required";

export type JobSourceDiscoveryCategory =
  | "ats-platform"
  | "broad-job-board"
  | "employer-careers"
  | "freelance-contract"
  | "government-public"
  | "healthcare"
  | "higher-education"
  | "international"
  | "nonprofit"
  | "public-community"
  | "public-sector"
  | "regional-local"
  | "remote-job-board"
  | "search-engine"
  | "sector-specific"
  | "startup-job-board";

export type JobSourceCoverage = "all" | readonly string[];

export interface RestrictedInteractiveSessionPolicy {
  readonly requiresUserInitiatedAction: boolean;
  readonly requiresFreshLogin: boolean;
  readonly preLoginWarningRequired: boolean;
  readonly storesAuthTokens: boolean;
  readonly storesSessionCookies: boolean;
  readonly storesBrowserStorage: boolean;
  readonly storesAuthorizationHeaders: boolean;
  readonly backgroundAutomationAllowed: boolean;
  readonly offlineUseAllowed: boolean;
  readonly privacyReminderMinutes: number;
  readonly hardSessionExpiryRequired: boolean;
  readonly warning: string;
}

export interface JobSourceDiscoveryEntry {
  readonly id: string;
  readonly label: string;
  readonly category: JobSourceDiscoveryCategory;
  readonly accessModel: JobSourceAccessModel;
  readonly technicalAccess?: JobSourceTechnicalAccess;
  readonly status: JobSourceImplementationStatus;
  readonly regions: readonly string[];
  readonly careerProfileIds: JobSourceCoverage;
  readonly hostPatterns: readonly string[];
  readonly locationSearchPatterns?: readonly string[];
  readonly searchParameterPatterns?: readonly string[];
  readonly navigationSurfacePatterns?: readonly string[];
  readonly restrictedInteractiveSessionPolicy?: RestrictedInteractiveSessionPolicy;
  readonly examples: readonly string[];
  readonly implementationPath: string;
  readonly notes: string;
  readonly requiresUserAgreement?: boolean;
}

export const TECH_PROFILE_IDS = [
  "software-engineering",
  "cybersecurity",
  "data-science",
] as const;

export const BUSINESS_PROFILE_IDS = [
  "office-administration",
  "product-management",
  "seo-digital-marketing",
  "sales-business-dev",
  "hr-recruiting",
  "finance-accounting",
  "project-operations",
  "customer-success",
] as const;

export const CREATIVE_PROFILE_IDS = [
  "content-copywriting",
  "creative-media",
] as const;

export const EDUCATION_PROFILE_IDS = ["education"] as const;
export const HEALTHCARE_PROFILE_IDS = ["healthcare"] as const;
export const LEGAL_PROFILE_IDS = ["legal"] as const;
export const TRADES_PROFILE_IDS = ["trades-field-service"] as const;
export const GOVERNMENT_FRIENDLY_PROFILE_IDS = [
  "office-administration",
  "trades-field-service",
  "software-engineering",
  "cybersecurity",
  "data-science",
  "product-management",
  "finance-accounting",
  "project-operations",
  "healthcare",
  "legal",
  "education",
  "customer-success",
] as const;

export const EMPLOYER_CAREER_SYSTEM_NOTES =
  "Use platform-specific public endpoints when documented. Otherwise keep this to user-opened discovery, Browser Import, pasted job links, or manual entry until terms and stability are reviewed.";

export const RESTRICTED_BOARD_NOTES =
  "Do not run silent scheduled discovery. Require explicit user agreement before automated access and prefer user-opened import paths. If the source is public and unauthenticated, do not apply authenticated-session rules. For account-backed interactive sessions, do not persist sign-in material or run background collection; use a privacy reminder instead of a hard time cap when JobSentinel is not inspecting or automating the restricted site.";

export const POLICY_BLOCKED_AUTOMATION_NOTES =
  "Current provider policy does not authorize JobSentinel automation. Local user agreement cannot enable scheduled access or pasted-URL fetching. Keep user-opened search links, visible Browser Import, and manual entry available.";

export function technicalAccessForJobSource(
  entry: Pick<JobSourceDiscoveryEntry, "accessModel" | "technicalAccess">,
): JobSourceTechnicalAccess {
  if (entry.technicalAccess !== undefined) {
    return entry.technicalAccess;
  }

  switch (entry.accessModel) {
    case "native-public":
    case "native-public-feed":
    case "public-community":
      return "public-unauthenticated";
    case "native-public-with-local-credential":
      return "public-local-credential";
    case "restricted-user-gated":
      return "public-unauthenticated";
    case "employer-career-system":
    case "review-required":
      return "unknown-review-required";
  }
}

export const RESTRICTED_AUTHENTICATED_INTERACTIVE_POLICY: RestrictedInteractiveSessionPolicy =
  {
    requiresUserInitiatedAction: true,
    requiresFreshLogin: true,
    preLoginWarningRequired: true,
    storesAuthTokens: false,
    storesSessionCookies: false,
    storesBrowserStorage: false,
    storesAuthorizationHeaders: false,
    backgroundAutomationAllowed: false,
    offlineUseAllowed: false,
    privacyReminderMinutes: RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES,
    hardSessionExpiryRequired: false,
    warning: RESTRICTED_AUTHENTICATED_SOURCE_WARNING,
  };
