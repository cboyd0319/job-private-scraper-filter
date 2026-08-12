import { getUserFriendlyError } from "../../../shared/errorReporting/messages";

export interface JobImportPreview {
  import_id: string | null;
  title: string;
  company: string;
  url: string;
  location: string | null;
  description_preview: string | null;
  salary: string | null;
  date_posted: string | null;
  valid_through: string | null;
  employment_types: string[];
  remote: boolean;
  missing_fields: string[];
  already_exists: boolean;
}

export interface JobImportResult {
  jobId: number;
}

export const policyBlockedPastedLinkMessage =
  "JobSentinel cannot fetch this pasted link. Open it in your browser and use visible Browser Import or manual entry.";
const policyBlockedCaptureMessage =
  "JobSentinel cannot fetch or capture this source. Open it in your browser or use manual entry.";

const missingDetailLabels = new Map<string, string>([
  ["title", "job title"],
  ["job_title", "job title"],
  ["company", "company name"],
  ["company_name", "company name"],
  ["salary", "pay range"],
  ["salary_min", "pay range"],
  ["salary_max", "pay range"],
  ["pay", "pay range"],
  ["pay_range", "pay range"],
  ["date_posted", "posting date"],
  ["posted_date", "posting date"],
  ["posting_date", "posting date"],
  ["valid_through", "closing date"],
  ["location", "location"],
  ["remote", "remote option"],
  ["employment_type", "work type"],
  ["employment_types", "work type"],
  ["description", "job description"],
  ["description_preview", "job description"],
  ["url", "job link"],
  ["job_url", "job link"],
  ["job_link", "job link"],
]);

export const fullJobLinkMessage =
  "Paste the full job link from your browser address bar.";
export const publicJobLinkMessage =
  "Paste a public job posting link from your browser address bar.";
export const secureJobLinkMessage =
  "Paste an https job posting link from your browser address bar.";

function formatMissingDetail(field: string) {
  const normalized = field
    .trim()
    .toLowerCase()
    .replace(/[\s-]+/g, "_");
  return missingDetailLabels.get(normalized) ?? normalized.replace(/_/g, " ");
}

export function formatMissingDetails(fields: string[]) {
  const labels = fields.map(formatMissingDetail);
  return [...new Set(labels)].join(", ");
}

export function formatImportDate(value: string) {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) {
    return "Date not shown";
  }

  return date.toLocaleDateString("en-US", { timeZone: "UTC" });
}

function extractImportErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return "";
}

function getSafeImportSpecificMessage(error: unknown): string | null {
  const message = extractImportErrorMessage(error).trim();
  if (!message) return null;
  if (message === policyBlockedPastedLinkMessage) return message;
  if (message === policyBlockedCaptureMessage) return message;

  const safePatterns = [
    new RegExp(`^${fullJobLinkMessage.replace(/\./g, "\\.")}$`),
    new RegExp(`^${publicJobLinkMessage.replace(/\./g, "\\.")}$`),
    new RegExp(`^${secureJobLinkMessage.replace(/\./g, "\\.")}$`),
    /^Add a job link from your browser address bar\.$/,
    /^Could not read this page as a single job posting\. Open one job posting, copy its browser address, or save the job with the details JobSentinel can find\.$/,
    /^Could not read this as one job posting\. Open one job posting and copy its browser address\.$/,
    /^Found \d+ job postings on this page\. Please use a more specific URL that links to a single job\.$/,
    /^Missing required information: [A-Za-z ]+\. This job posting may be incomplete\.$/,
    /^This took too long\. Check your internet connection and try again\.$/,
    /^Could not connect to the website\. Please check your internet connection\.$/,
    /^Failed to fetch the page\. Please check the URL and try again\.$/,
    /^The website returned an error: [0-9]{3}(?: [A-Za-z ]+)?$/,
    /^The job page response is too large to import safely\. Maximum size is \d+ MiB\.$/,
    /^The job link redirects to another page\. Paste the final public job posting link from your browser address bar\.$/,
    /^This job is already in your saved jobs\.?$/,
    /^This job preview expired\. Check the job link again before saving\.$/,
    /^This Smart Paste draft expired\. Create it again before saving\.$/,
    /^Create and review the Smart Paste draft before saving\.$/,
    /^Smart Paste is paused because the reviewed source policy changed\. Restart JobSentinel and try again\.$/,
    /^Paste no more than 50,000 characters of job details at a time\.$/,
    /^Shorten the pasted [A-Za-z ]+ before reviewing this draft\.$/,
    /^Remove passwords, tokens, cookies, and authorization details before creating this draft\.$/,
    /^JobSentinel cannot fetch this pasted link\. Open it in your browser and use visible Browser Import or manual entry\.$/,
  ];

  return safePatterns.some((pattern) => pattern.test(message)) ? message : null;
}

export function getSafeJobImportError(error: unknown) {
  const importMessage = getSafeImportSpecificMessage(error);
  if (importMessage) {
    return {
      message: importMessage,
      action: importMessage,
    };
  }

  const friendly = getUserFriendlyError(error);
  return {
    message: friendly.message,
    action: friendly.action ?? friendly.message,
  };
}
