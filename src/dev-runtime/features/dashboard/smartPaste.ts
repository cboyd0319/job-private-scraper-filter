import { isSafeExternalHttpsUrl } from "../../mocks/externalUrlSafety";

const STRIPPED_JOB_IMPORT_QUERY_KEYS = new Set([
  "aff_id",
  "affiliate",
  "campaign",
  "cid",
  "click_id",
  "clickid",
  "fbclid",
  "from",
  "gclid",
  "igshid",
  "lever-origin",
  "lever-source",
  "lever-source[]",
  "mc_cid",
  "mc_eid",
  "medium",
  "msclkid",
  "partner",
  "ref",
  "referrer",
  "refid",
  "sid",
  "source",
  "tk",
  "trackingid",
  "twclid",
]);

const STRIPPED_JOB_IMPORT_QUERY_MARKERS = [
  "token",
  "session",
  "auth",
  "credential",
  "password",
  "email",
  "candidate",
];

const MOCK_SMART_PASTE_CREDENTIAL_FIELD =
  "(?:[a-z0-9]+[_-])*(?:token|credential|password)(?:[_-][a-z0-9]+)*|" +
  "(?:client|api|auth|private|oauth|access|refresh)[_-]?(?:secret|key)|" +
  "secret|passphrase|session[_-]?id|authorization|proxy[_-]?authorization|" +
  "cookie|set[_-]?cookie|li_at|jsessionid|bcookie|bscookie|liap|lidc|li_gc";
const MOCK_SMART_PASTE_CREDENTIAL_ASSIGNMENT = new RegExp(
  `\\b(?:${MOCK_SMART_PASTE_CREDENTIAL_FIELD})\\b\\s*=\\s*[^\\s,;]+`,
  "i",
);
const MOCK_SMART_PASTE_CREDENTIAL_JSON = new RegExp(
  `["'](?:${MOCK_SMART_PASTE_CREDENTIAL_FIELD})["']\\s*:\\s*["'][^"'\\r\\n]{4,}["']`,
  "i",
);
const MOCK_SMART_PASTE_COOKIE = /(?:cookie|set-cookie)\s*:\s*[-a-z0-9_]+\s*=/i;
const MOCK_SMART_PASTE_COOKIE_OBJECT =
  /["']name["']\s*:\s*["'](?:li_at|jsessionid|bcookie|bscookie|liap|lidc|li_gc)["']/i;

export const MAX_SMART_PASTE_CHARS = 50_000;

export function parseMockSmartPasteText(text: string) {
  let title = "";
  let company = "";
  let url = "";
  let location = "";
  const description: string[] = [];
  const unlabeled: string[] = [];

  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const separator = trimmed.indexOf(":");
    if (separator >= 0) {
      const label = trimmed.slice(0, separator);
      const value = trimmed.slice(separator + 1);
      const trimmedValue = value.trim();
      switch (label.trim().toLowerCase()) {
        case "title":
        case "job title":
          if (trimmedValue) {
            title = trimmedValue;
            continue;
          }
          break;
        case "company":
          if (trimmedValue) {
            company = trimmedValue;
            continue;
          }
          break;
        case "location":
          if (trimmedValue) {
            location = trimmedValue;
            continue;
          }
          break;
        case "description":
          if (trimmedValue) {
            description.push(trimmedValue);
            continue;
          }
          break;
        case "url":
        case "job link":
          if (trimmedValue) {
            url = canonicalizeMockSmartPasteUrl(trimmedValue);
            continue;
          }
          break;
      }
    }
    if (!url) {
      const candidate = trimmed
        .split(/\s+/)
        .map((word) => word.replace(/^[[<("']+/, "").replace(/[>)\]"',]+$/, ""))
        .find((word) => word.startsWith("https://"));
      if (candidate) {
        url = canonicalizeMockSmartPasteUrl(candidate);
        continue;
      }
    }
    unlabeled.push(trimmed);
  }

  if (!title) title = unlabeled.shift() ?? "";
  if (!company) company = unlabeled.shift() ?? "";
  description.push(...unlabeled);

  return {
    title,
    company,
    url,
    location,
    description: description.join("\n"),
  };
}

export function rejectMockSmartPasteCredentialMaterial(value: string): void {
  const safeValue = value
    .replace(/\[REDACTED\]/g, "")
    .replace(/\[removed\]/g, "");
  if (
    MOCK_SMART_PASTE_CREDENTIAL_ASSIGNMENT.test(safeValue) ||
    MOCK_SMART_PASTE_CREDENTIAL_JSON.test(safeValue) ||
    MOCK_SMART_PASTE_COOKIE.test(safeValue) ||
    MOCK_SMART_PASTE_COOKIE_OBJECT.test(safeValue) ||
    /authorization:\s*["']?bearer\s+["']?[^\s"',;()]{12,}["']?/i.test(safeValue) ||
    /authorization:\s*["']?(?:basic|negotiate)\s+["']?[a-z0-9+/=]{8,}["']?/i.test(
      safeValue,
    ) ||
    /\bbearer\s+["']?[^\s"',;()]{24,}["']?/i.test(safeValue)
  ) {
    throw new Error(
      "Pasted job details contain session or credential material",
    );
  }
}

export function checkMockSmartPasteField(
  field: "job title" | "company name" | "location",
  value: string,
  maxBytes: number,
): void {
  if (new TextEncoder().encode(value).length > maxBytes) {
    throw new Error(`Pasted ${field} exceeds the local draft limit`);
  }
}

export function truncateMockSmartPasteDescription(description: string): string {
  const chars = [...description];
  return chars.length > 500
    ? `${chars.slice(0, 500).join("")}...`
    : description;
}

export function canonicalizeMockJobImportUrl(url: string): string {
  const parsed = new URL(url);
  parsed.username = "";
  parsed.password = "";
  parsed.hash = "";

  const keptParams = new URLSearchParams();
  parsed.searchParams.forEach((value, key) => {
    const normalizedKey = key.toLowerCase();
    if (!isStrippedJobImportQueryParam(normalizedKey, value)) {
      keptParams.append(key, value);
    }
  });

  const query = keptParams.toString();
  parsed.search = isLinkedInHost(parsed.hostname) ? "" : query ? `?${query}` : "";
  return parsed.toString();
}

function isLinkedInHost(hostname: string): boolean {
  const host = hostname.toLowerCase().replace(/\.$/, "");
  return host === "linkedin.com" || host.endsWith(".linkedin.com");
}

export function canonicalizeMockSmartPasteUrl(value: string): string {
  if (!isSafeExternalHttpsUrl(value)) {
    throw new Error("Paste the full job link from your browser address bar.");
  }
  return canonicalizeMockJobImportUrl(value);
}

function isStrippedJobImportQueryParam(
  normalizedKey: string,
  value: string,
): boolean {
  return (
    normalizedKey.startsWith("utm_") ||
    STRIPPED_JOB_IMPORT_QUERY_KEYS.has(normalizedKey) ||
    STRIPPED_JOB_IMPORT_QUERY_MARKERS.some((marker) =>
      normalizedKey.includes(marker),
    ) ||
    isSensitiveJobImportQueryValue(value)
  );
}

function isSensitiveJobImportQueryValue(value: string): boolean {
  try {
    new URL(value);
    return true;
  } catch {
    const normalizedValue = value.toLowerCase();
    return STRIPPED_JOB_IMPORT_QUERY_MARKERS.some(
      (marker) =>
        normalizedValue.includes(`${marker}=`) ||
        normalizedValue.includes(`${marker}%3d`),
    );
  }
}
