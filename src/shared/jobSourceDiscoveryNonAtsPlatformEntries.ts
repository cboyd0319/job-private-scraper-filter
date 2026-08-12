import * as model from "./jobSourceDiscoveryModel";

export const JOB_SOURCE_NON_ATS_PLATFORM_ENTRIES: readonly model.JobSourceDiscoveryEntry[] = [
  {
    id: "amazon-jobs",
    label: "Amazon Jobs",
    category: "employer-careers",
    accessModel: "native-public",
    status: "candidate",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: ["amazon.jobs", "www.amazon.jobs", "www.amazon.jobs/en/search.json"],
    examples: ["Amazon Jobs"],
    implementationPath:
      "Candidate native adapter using Amazon's employer-owned public JSON search endpoints.",
    notes:
      "Amazon Jobs returns public listing JSON from its own careers domain. Keep request volume bounded and normalize relative job paths.",
  },
  {
    id: "google-careers",
    label: "Google Careers",
    category: "employer-careers",
    accessModel: "employer-career-system",
    technicalAccess: "public-unauthenticated",
    status: "candidate",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: [
      "google.com/about/careers/applications",
      "www.google.com/about/careers/applications",
      "gstatic.com/hiring",
    ],
    examples: ["Google Careers"],
    implementationPath:
      "Detect Google Careers pages and add native support only after a stable public job-search fixture is reviewed.",
    notes:
      "Google Careers is public and employer-owned, but the job-search transport is an internal web app. " +
      model.EMPLOYER_CAREER_SYSTEM_NOTES,
  },
  {
    id: "tesla-careers",
    label: "Tesla Careers",
    category: "employer-careers",
    accessModel: "employer-career-system",
    technicalAccess: "public-unauthenticated",
    status: "research",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: ["tesla.com/careers", "www.tesla.com/careers"],
    examples: ["Tesla Careers"],
    implementationPath:
      "Detect Tesla Careers pages and add native support only after stable public fixtures are captured without bypassing edge controls.",
    notes:
      "Tesla publishes jobs on its own careers domain, but local direct fetches can be blocked by edge controls. Keep browser-open and manual import fallbacks until adapter fixtures are reliable.",
  },
  {
    id: "employer-careers-pages",
    label: "Employer Careers Pages",
    category: "employer-careers",
    accessModel: "employer-career-system",
    status: "candidate",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: ["careers.*", "*/careers", "*/jobs"],
    examples: [
      "Fivetran Careers",
      "SpaceX Careers",
      "Tesla Careers",
      "Google Careers",
      "Amazon Jobs",
      "GitHub Careers",
      "OpenAI Careers",
      "Anthropic Careers",
      "Yahoo Careers",
      "IBM Careers",
      "Microsoft Careers",
    ],
    implementationPath:
      "Discover ATS/API signals first, then fall back to user-opened import, pasted job links, or manual entry.",
    notes:
      "This is the broad discovery layer for companies that do not appear on stock job boards.",
  },
  {
    id: "linkedin",
    label: "LinkedIn",
    category: "broad-job-board",
    accessModel: "restricted-user-gated",
    technicalAccess: "authenticated-user-session",
    status: "restricted",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: ["linkedin.com/jobs", "linkedin.com/company/*/jobs"],
    searchParameterPatterns: [
      "keywords=<role-or-skill>",
      "geoId=<linkedin-location-id>",
      "f_TPR=<posted-within-seconds>",
      "f_AL=true",
      "currentJobId=<selected-job-id>",
      "showHowYouFit=HOW_YOU_FIT",
      "origin=<linkedin-surface>",
      "originToLandingJobPostings=<job-id-list>",
      "referralSearchId=<session-search-id>",
    ],
    navigationSurfacePatterns: [
      "jobs home",
      "preferences",
      "job tracker",
      "my career insights",
    ],
    restrictedInteractiveSessionPolicy:
      model.RESTRICTED_AUTHENTICATED_INTERACTIVE_POLICY,
    examples: ["LinkedIn Jobs", "LinkedIn company job tabs"],
    implementationPath:
      "User-gated restricted discovery only. Prefer the user-opened local Workbench and never run Browser Import or silent scheduled discovery. Persist user-entered query intent and selected filters only; do not persist referral, origin, or session-like identifiers as source configuration.",
    notes:
      "Observed search-results URLs can include role keywords, LinkedIn geo IDs, posted-within filters, Easy Apply filters, selected job IDs, how-you-fit surfaces, origin surfaces, referral search IDs, landing job lists, and jobs-hub text-fragment anchors. Treat LinkedIn-specific IDs and navigation context as volatile user/session context. Keep access user opened, store no sign-in material, and use only details the user enters or selects for the local Workbench.",
    requiresUserAgreement: true,
  },
  {
    id: "linkedin-jobs-tracker",
    label: "LinkedIn Jobs Tracker",
    category: "broad-job-board",
    accessModel: "restricted-user-gated",
    technicalAccess: "authenticated-user-session",
    status: "research",
    regions: ["global"],
    careerProfileIds: "all",
    hostPatterns: ["linkedin.com/jobs-tracker"],
    searchParameterPatterns: [
      "stage=applied",
      "stage=saved",
      "stage=interviewing",
      "stage=archived",
    ],
    restrictedInteractiveSessionPolicy:
      model.RESTRICTED_AUTHENTICATED_INTERACTIVE_POLICY,
    examples: ["LinkedIn applied jobs", "LinkedIn saved jobs"],
    implementationPath:
      "User-gated restricted tracking only. If supported later, limit this to user-reviewed jobs already saved or applied on LinkedIn, never broad background discovery.",
    notes:
      "This is an application-tracking path, not a search source. Require prominent user agreement, do not capture cookies, do not bypass login or human checks, and do not persist session identifiers. " +
      model.RESTRICTED_BOARD_NOTES,
    requiresUserAgreement: true,
  },
] as const;
