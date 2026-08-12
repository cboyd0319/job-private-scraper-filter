export type RestrictedJobSourceDomainCategory =
  | "authenticated-marketplace"
  | "local-credential-source"
  | "policy-blocked-automation"
  | "restricted-job-board"
  | "review-required-source";

export interface RestrictedJobSourceDomainRecord {
  readonly domain: string;
  readonly category: RestrictedJobSourceDomainCategory;
  readonly sourceRefs: readonly string[];
  readonly reason: string;
}

export const RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS = [
  {
    domain: "bayt.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:bayt"],
    reason:
      "Bayt is a MENA job board marked restricted-user-gated in the source taxonomy; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "bdjobs.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:bdjobs"],
    reason:
      "BDJobs is a Bangladesh job board marked restricted-user-gated in the source taxonomy; warn before direct collection until source-specific access terms, rate limits, and account behavior are reviewed.",
  },
  {
    domain: "builtin.com",
    category: "policy-blocked-automation",
    sourceRefs: ["job-source-discovery:builtin", "source-manifest:builtin-v2"],
    reason:
      "Built In's reviewed provider policy prohibits automated retrieval, so JobSentinel blocks scheduled and pasted-URL fetches while retaining warnings for user-opened links and visible Browser Import.",
  },
  {
    domain: "builtincolorado.com",
    category: "policy-blocked-automation",
    sourceRefs: ["job-source-discovery:builtin", "source-manifest:builtin-v2"],
    reason:
      "Built In Colorado belongs to the reviewed Built In network, so JobSentinel blocks scheduled and pasted-URL fetches while retaining warnings for user-opened links and visible Browser Import.",
  },
  {
    domain: "careerbuilder.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:careerbuilder"],
    reason:
      "CareerBuilder is a broad job board marked restricted-user-gated; direct collection needs explicit user agreement because account, terms, and platform controls may apply.",
  },
  {
    domain: "clearancejobs.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:clearancejobs"],
    reason:
      "ClearanceJobs covers clearance-sensitive roles and is marked restricted-user-gated; warn before direct collection because account state and sensitive job-search context may apply.",
  },
  {
    domain: "cv-library.co.uk",
    category: "review-required-source",
    sourceRefs: ["job-source-discovery:cv-library"],
    reason:
      "CV-Library is a legacy UK broad-board warning gate without a reviewed native adapter; keep it restricted until source-specific terms, rate limits, and account behavior are reviewed.",
  },
  {
    domain: "dice.com",
    category: "policy-blocked-automation",
    sourceRefs: ["job-source-discovery:dice", "source-manifest:dice-v2"],
    reason:
      "Dice's reviewed provider policy does not authorize the legacy HTML adapter or pasted-URL fetches; user-opened links and visible Browser Import remain warning-gated local paths.",
  },
  {
    domain: "flexjobs.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:flexjobs"],
    reason:
      "FlexJobs is marked restricted-user-gated and may involve member/account access; warn before direct collection and do not silently schedule it without agreement.",
  },
  {
    domain: "foundit.in",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:foundit"],
    reason:
      "Foundit India is marked restricted-user-gated for India/APAC job search; warn before direct collection until access model, terms, and account behavior are reviewed.",
  },
  {
    domain: "foundit.id",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:foundit"],
    reason:
      "Foundit Indonesia is covered by the restricted Foundit taxonomy entry; keep warning coverage until source-specific terms, rate limits, and account behavior are reviewed.",
  },
  {
    domain: "foundit.sg",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:foundit"],
    reason:
      "Foundit Singapore is marked restricted-user-gated for APAC job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "founditgulf.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:foundit"],
    reason:
      "Foundit Gulf is covered by the restricted Foundit taxonomy entry; keep warning coverage until source-specific terms, rate limits, and account behavior are reviewed.",
  },
  {
    domain: "freelancer.com",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:freelancer"],
    reason:
      "Freelancer is a marketplace marked restricted-user-gated; warn because account state, bidding workflows, marketplace terms, and user privacy are central to access.",
  },
  {
    domain: "glassdoor.com",
    category: "policy-blocked-automation",
    sourceRefs: ["job-source-discovery:glassdoor", "source-manifest:glassdoor-v2"],
    reason:
      "Glassdoor's reviewed provider policy prohibits unapproved automation, so JobSentinel blocks scheduled and pasted-URL fetches while retaining warnings for user-opened links and visible Browser Import.",
  },
  {
    domain: "governmentjobs.com",
    category: "review-required-source",
    sourceRefs: ["job-source-discovery:governmentjobs"],
    reason:
      "GovernmentJobs is review-required in the source taxonomy and can include application/account surfaces; warn on direct collection until public access boundaries are reviewed.",
  },
  {
    domain: "hiretechladies.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:tech-ladies"],
    reason:
      "Tech Ladies is a community job board marked restricted-user-gated; direct collection should require user agreement because membership, community access, and account behavior may apply.",
  },
  {
    domain: "gulftalent.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:gulftalent"],
    reason:
      "GulfTalent is marked restricted-user-gated for Gulf-region job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "indeed.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:indeed"],
    reason:
      "Indeed is a broad job board marked restricted-user-gated; direct collection requires explicit user agreement because terms, account, and anti-automation controls may apply.",
  },
  {
    domain: "jobstreet.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:jobstreet"],
    reason:
      "JobStreet is marked restricted-user-gated for APAC job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "jora.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:jora"],
    reason:
      "Jora is marked restricted-user-gated for international job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "linkedin.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:linkedin", "interactive-source:linkedin"],
    reason:
      "LinkedIn jobs and tracker surfaces are restricted and account-centric; use only explicit user-gated, fresh-login interactive flows with no auth or session persistence.",
  },
  {
    domain: "monster.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:monster"],
    reason:
      "Monster is a broad job board marked restricted-user-gated; direct collection needs explicit user agreement because account, terms, and platform controls may apply.",
  },
  {
    domain: "malt.com",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:malt"],
    reason:
      "Malt is a European freelance marketplace marked restricted-user-gated; warn because account state, proposal workflows, marketplace terms, and user privacy are central to access.",
  },
  {
    domain: "malt.de",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:malt"],
    reason:
      "Malt Germany is part of the Malt freelance marketplace; warn because account state, proposal workflows, marketplace terms, and user privacy are central to access.",
  },
  {
    domain: "malt.fr",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:malt"],
    reason:
      "Malt France is part of the Malt freelance marketplace; warn because account state, proposal workflows, marketplace terms, and user privacy are central to access.",
  },
  {
    domain: "naukri.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:naukri"],
    reason:
      "Naukri is marked restricted-user-gated for India job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "otta.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:otta"],
    reason:
      "Otta is a curated startup job board marked restricted-user-gated; direct collection should require user agreement because account, personalization, and platform controls may apply.",
  },
  {
    domain: "reed.co.uk",
    category: "local-credential-source",
    sourceRefs: ["job-source-discovery:reed"],
    reason:
      "Reed is modeled as a local-credential/API candidate; warn for direct website collection so JobSentinel favors reviewed API or credentialed paths when available.",
  },
  {
    domain: "seek.co.nz",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:seek"],
    reason:
      "SEEK New Zealand is marked restricted-user-gated; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "seek.com.au",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:seek"],
    reason:
      "SEEK Australia is marked restricted-user-gated; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "shine.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:shine"],
    reason:
      "Shine is marked restricted-user-gated for India job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "simplyhired.com",
    category: "policy-blocked-automation",
    sourceRefs: ["job-source-discovery:simplyhired", "source-manifest:simplyhired-v2"],
    reason:
      "SimplyHired is covered by reviewed Indeed provider terms that prohibit unapproved automation, so JobSentinel blocks scheduled and pasted-URL fetches while retaining user-opened warnings.",
  },
  {
    domain: "snagajob.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:snagajob"],
    reason:
      "Snagajob is marked restricted-user-gated for hourly retail and hospitality roles; warn before direct collection until access model and account behavior are reviewed.",
  },
  {
    domain: "stepstone.co.uk",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:stepstone"],
    reason:
      "StepStone UK is part of the StepStone job-board network marked restricted-user-gated; direct collection needs country-specific terms and platform-control review.",
  },
  {
    domain: "stepstone.de",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:stepstone"],
    reason:
      "StepStone Germany is part of the StepStone job-board network marked restricted-user-gated; direct collection needs country-specific terms and platform-control review.",
  },
  {
    domain: "stepstone.fr",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:stepstone"],
    reason:
      "StepStone France is part of the StepStone job-board network marked restricted-user-gated; direct collection needs country-specific terms and platform-control review.",
  },
  {
    domain: "timesjobs.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:timesjobs"],
    reason:
      "TimesJobs is marked restricted-user-gated for India job search; warn before direct collection until source-specific access terms and account behavior are reviewed.",
  },
  {
    domain: "totaljobs.com",
    category: "review-required-source",
    sourceRefs: ["job-source-discovery:totaljobs"],
    reason:
      "Totaljobs is a legacy UK broad-board warning gate without a reviewed native adapter; keep it restricted until source-specific terms, rate limits, and account behavior are reviewed.",
  },
  {
    domain: "toptal.com",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:toptal"],
    reason:
      "Toptal is a talent marketplace marked restricted-user-gated; warn because account state, marketplace terms, screening context, and user privacy are central to access.",
  },
  {
    domain: "upwork.com",
    category: "authenticated-marketplace",
    sourceRefs: ["job-source-discovery:upwork"],
    reason:
      "Upwork is a marketplace marked restricted-user-gated; warn because account state, proposal workflows, marketplace terms, and user privacy are central to access.",
  },
  {
    domain: "vivian.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:vivian-health"],
    reason:
      "Vivian Health is marked restricted-user-gated for healthcare and travel nursing roles; warn before direct collection until access model and account behavior are reviewed.",
  },
  {
    domain: "wellfound.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:wellfound"],
    reason:
      "Wellfound is marked restricted-user-gated for startup hiring; warn before direct collection until current source access model and account behavior are reviewed.",
  },
  {
    domain: "xing.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:xing-jobs"],
    reason:
      "XING Jobs is account-adjacent professional-network job search for German-speaking markets; warn before collection until account behavior and terms are reviewed.",
  },
  {
    domain: "ziprecruiter.com",
    category: "restricted-job-board",
    sourceRefs: ["job-source-discovery:ziprecruiter"],
    reason:
      "ZipRecruiter is a broad job board marked restricted-user-gated; direct collection needs explicit user agreement because terms, account, and platform controls may apply.",
  },
] as const satisfies readonly RestrictedJobSourceDomainRecord[];

export const RESTRICTED_JOB_SOURCE_DOMAINS: readonly string[] =
  RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS.map((record) => record.domain);
export const POLICY_BLOCKED_SCHEDULED_SOURCE_IDS = [
  "builtin",
  "dice",
  "simplyhired",
  "glassdoor",
] as const;
const POLICY_BLOCKED_AUTOMATION_DOMAINS =
  RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS.filter(
    (record) => record.category === "policy-blocked-automation",
  ).map((record) => record.domain);

export const RESTRICTED_JOB_SOURCE_WARNING =
  "Some job sites have rules about automated tools. JobSentinel keeps you in control: it will not sign in for you, click for you, bypass checks, or save your login. Continue only for sites you choose to use, and review that site's rules if you are unsure.";

export const RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES = 60;

export const RESTRICTED_AUTHENTICATED_SOURCE_WARNING =
  "JobSentinel can open a sign-in window for a restricted job site when you ask. You sign in and use the site yourself. JobSentinel does not save your sign-in, read the page for you, click buttons, or run in the background. Use JobSentinel's local buttons and notes to record what you did.";

export type RestrictedSourceAcknowledgements = Record<
  "builtin" | "dice" | "simplyhired" | "glassdoor",
  boolean
>;

export const DEFAULT_RESTRICTED_SOURCE_ACKNOWLEDGEMENTS: RestrictedSourceAcknowledgements =
  {
    builtin: false,
    dice: false,
    simplyhired: false,
    glassdoor: false,
  };

export function normalizeRestrictedSourceAcknowledgements(
  acknowledgements:
    | Partial<RestrictedSourceAcknowledgements>
    | null
    | undefined,
): RestrictedSourceAcknowledgements {
  return {
    ...DEFAULT_RESTRICTED_SOURCE_ACKNOWLEDGEMENTS,
    ...(acknowledgements ?? {}),
  };
}

export function isRestrictedJobSourceHost(hostname: string): boolean {
  const normalized = hostname.trim().toLowerCase();
  return RESTRICTED_JOB_SOURCE_DOMAINS.some(
    (domain) => normalized === domain || normalized.endsWith(`.${domain}`),
  );
}

export function isRestrictedJobSourceUrl(value: string): boolean {
  try {
    return isRestrictedJobSourceHost(new URL(value).hostname);
  } catch {
    return false;
  }
}

export function isPolicyBlockedAutomationUrl(value: string): boolean {
  try {
    const hostname = new URL(value).hostname.toLowerCase();
    return POLICY_BLOCKED_AUTOMATION_DOMAINS.some(
      (domain) => hostname === domain || hostname.endsWith(`.${domain}`),
    );
  } catch {
    return false;
  }
}

export function isPolicyBlockedScheduledSourceId(value: string): boolean {
  return POLICY_BLOCKED_SCHEDULED_SOURCE_IDS.some((sourceId) => sourceId === value);
}
