import { RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES } from "../../shared/restrictedSourceTaxonomy";

export const LINKEDIN_WORKBENCH_PRIVACY_REMINDER_MINUTES =
  RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES;

export type LinkedInWorkbenchEventType =
  | "applied"
  | "saved"
  | "tracking"
  | "rejected"
  | "interview"
  | "follow_up"
  | "reminder"
  | "note"
  | "not_interested";

export interface LinkedInWorkbenchPrefill {
  title: string;
  company: string;
  url: string;
  notes: string;
}

const STRIPPED_WORKBENCH_QUERY_KEYS = new Set([
  "fbclid",
  "gclid",
  "igshid",
  "mc_cid",
  "mc_eid",
  "msclkid",
  "origin",
  "origintolandingjobpostings",
  "ref",
  "referrer",
  "referralsearchid",
  "source",
]);
const SENSITIVE_WORKBENCH_QUERY_MARKERS = [
  "token",
  "session",
  "auth",
  "credential",
  "password",
  "email",
  "candidate",
];
const URL_PATTERN = /https?:\/\/[^\s"'<>\\)]+/gi;
const LINKEDIN_COOKIE_PATTERN = /li_at=[^\s;]+/gi;
const SENSITIVE_NOTE_FIELD_PATTERN =
  /\b(access_token|refresh_token|api[_-]?key|token|secret|password|session|auth|credential)=([^\s&"'<>\\)]+)/gi;

export function defaultLinkedInWorkbenchPrefill(): LinkedInWorkbenchPrefill {
  return {
    title: "",
    company: "",
    url: "",
    notes: "",
  };
}

export function parseUserProvidedLinkedInText(text: string): LinkedInWorkbenchPrefill {
  const cleaned = text.trim();
  if (!cleaned) {
    return defaultLinkedInWorkbenchPrefill();
  }

  const lines = cleaned
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const rawUrl = extractFirstHttpsUrl(cleaned);
  const url = rawUrl ? sanitizeLinkedInWorkbenchUrl(rawUrl) : "";
  const nonUrlLines = lines.filter((line) => line !== rawUrl);
  const titleAndCompany = splitTitleAndCompany(nonUrlLines[0] ?? "");

  return {
    title: titleAndCompany.title,
    company: titleAndCompany.company || nonUrlLines[1] || "",
    url,
    notes: sanitizeLinkedInWorkbenchTextForStorage(cleaned),
  };
}

export function sanitizeLinkedInWorkbenchTextForStorage(text: string): string {
  const withoutCookies = text.replace(LINKEDIN_COOKIE_PATTERN, "li_at=[REDACTED]");
  const withoutSensitiveFields = withoutCookies.replace(
    SENSITIVE_NOTE_FIELD_PATTERN,
    (_match, key: string) => `${key}=[removed]`,
  );

  return withoutSensitiveFields.replace(URL_PATTERN, (url) =>
    sanitizeLinkedInWorkbenchUrl(url),
  );
}

export function sanitizeLinkedInWorkbenchUrl(value: string): string {
  const trimmed = value.trim().replace(/[),.;]+$/, "");
  if (!trimmed) {
    return "";
  }

  try {
    const parsed = new URL(trimmed);
    parsed.username = "";
    parsed.password = "";
    parsed.hash = "";

    for (const key of Array.from(parsed.searchParams.keys())) {
      const normalized = key.toLowerCase();
      const values = parsed.searchParams.getAll(key);
      if (
        normalized.startsWith("utm_") ||
        STRIPPED_WORKBENCH_QUERY_KEYS.has(normalized) ||
        SENSITIVE_WORKBENCH_QUERY_MARKERS.some((marker) =>
          normalized.includes(marker),
        ) ||
        values.some(isSensitiveWorkbenchQueryValue)
      ) {
        parsed.searchParams.delete(key);
      }
    }

    if (parsed.hostname === "linkedin.com" || parsed.hostname.endsWith(".linkedin.com")) {
      parsed.search = "";
    }

    return parsed.toString();
  } catch {
    return trimmed;
  }
}

export function shouldShowLinkedInWorkbenchPrivacyReminder(
  sessionStartedAt: number | null,
  now: number = Date.now(),
): boolean {
  if (sessionStartedAt === null) {
    return false;
  }

  return now - sessionStartedAt >= LINKEDIN_WORKBENCH_PRIVACY_REMINDER_MINUTES * 60_000;
}

function extractFirstHttpsUrl(text: string): string {
  const match = text.match(/https:\/\/[^\s<>"']+/i);
  return match?.[0]?.replace(/[),.;]+$/, "") ?? "";
}

function isSensitiveWorkbenchQueryValue(value: string): boolean {
  try {
    new URL(value);
    return true;
  } catch {
    const normalized = value.toLowerCase();
    return SENSITIVE_WORKBENCH_QUERY_MARKERS.some(
      (marker) =>
        normalized.includes(`${marker}=`) ||
        normalized.includes(`${marker}%3d`),
    );
  }
}

function splitTitleAndCompany(line: string): Pick<LinkedInWorkbenchPrefill, "title" | "company"> {
  const separator = /\s+(?:at|@|-|–|—)\s+/i;
  const parts = line.split(separator).map((part) => part.trim()).filter(Boolean);

  if (parts.length >= 2) {
    return {
      title: parts[0] ?? "",
      company: parts.slice(1).join(" "),
    };
  }

  return {
    title: line,
    company: "",
  };
}
