import {
  getNextId,
  getStringArg,
} from "../../mocks/handlers/commandHelpers";
import type { MockJob } from "../../mocks/handlers/types";
import {
  isLocalOrPrivateHost,
  isSafeExternalHttpsUrl,
} from "../../mocks/externalUrlSafety";
import {
  canonicalizeMockJobImportUrl,
  canonicalizeMockSmartPasteUrl,
  checkMockSmartPasteField,
  MAX_SMART_PASTE_CHARS,
  parseMockSmartPasteText,
  rejectMockSmartPasteCredentialMaterial,
  truncateMockSmartPasteDescription,
} from "./smartPaste";

export interface MockJobImportPreview {
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

export interface MockPendingUrlImport {
  id: string;
  preview: MockJobImportPreview;
  kind: "jobImport" | "smartPaste";
  description?: string | null;
  usesSmartPasteDefaults?: boolean;
}

export interface MockJobImportResult {
  jobId: number;
}

interface MockJobImportCommandState {
  jobs: MockJob[];
  pendingUrlImports: MockPendingUrlImport[];
}

export interface MockJobImportCommandResult {
  handled: boolean;
  shouldSave: boolean;
  state: MockJobImportCommandState;
  value: unknown;
}

export function handleMockJobImportCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockJobImportCommandState,
): MockJobImportCommandResult {
  switch (command) {
    case "preview_job_import":
      return previewMockJobImport(args, state);
    case "confirm_job_import":
      return confirmMockPendingImport(
        args,
        state,
        "This job preview expired. Check the job link again before saving.",
        "jobImport",
      );
    case "preview_smart_paste":
      return previewMockSmartPaste(args, state);
    case "confirm_smart_paste":
      return confirmMockPendingImport(
        args,
        state,
        "This Smart Paste draft expired. Create it again before saving.",
        "smartPaste",
      );
    default:
      return { handled: false, shouldSave: false, state, value: undefined };
  }
}

function confirmMockPendingImport(
  args: Record<string, unknown> | undefined,
  state: MockJobImportCommandState,
  expiredMessage: string,
  kind: MockPendingUrlImport["kind"],
): MockJobImportCommandResult {
  const importId = getStringArg(args, "importId")?.trim();
  const pending = state.pendingUrlImports.find(
    (entry) => entry.id === importId && entry.kind === kind,
  );
  if (!pending) {
    throw new Error(expiredMessage);
  }

  if (state.jobs.some((job) => job.url === pending.preview.url)) {
    throw new Error("This job is already in your saved jobs");
  }

  const job = buildMockImportedJob(
    pending.preview,
    getNextId(state.jobs),
    new Date().toISOString(),
    pending.description,
    pending.usesSmartPasteDefaults,
  );
  return {
    handled: true,
    shouldSave: true,
    state: {
      jobs: [job, ...state.jobs],
      pendingUrlImports: state.pendingUrlImports.filter(
        (entry) => entry.id !== importId,
      ),
    },
    value: { jobId: job.id } satisfies MockJobImportResult,
  };
}

function previewMockJobImport(
  args: Record<string, unknown> | undefined,
  state: MockJobImportCommandState,
): MockJobImportCommandResult {
  const url = getJobImportUrl(args);
  const title = getMockImportTitle(url);
  const company = new URL(url).hostname;
  const alreadyExists = state.jobs.some((job) => job.url === url);
  const importId = alreadyExists
    ? null
    : `mock-${hashString(`${url}:${title}`)}`;

  const preview: MockJobImportPreview = {
    import_id: importId,
    title,
    company,
    url,
    location: "Remote",
    description_preview: `${title} role imported from ${company}. Review details before saving.`,
    salary: "$55k-$72k",
    date_posted: new Date().toISOString(),
    valid_through: null,
    employment_types: ["FULL_TIME"],
    remote: true,
    missing_fields: [],
    already_exists: alreadyExists,
  };
  const pendingUrlImports = importId
    ? [
        ...state.pendingUrlImports.filter((entry) => entry.id !== importId),
        { id: importId, preview, kind: "jobImport" as const },
      ]
    : state.pendingUrlImports;

  return withoutSave({ ...state, pendingUrlImports }, preview);
}

function previewMockSmartPaste(
  args: Record<string, unknown> | undefined,
  state: MockJobImportCommandState,
): MockJobImportCommandResult {
  const text = getStringArg(args, "text") ?? "";
  if ([...text].length > MAX_SMART_PASTE_CHARS) {
    throw new Error(
      "Paste no more than 50,000 characters of job details at a time.",
    );
  }
  rejectMockSmartPasteCredentialMaterial(text);

  const parsed = parseMockSmartPasteText(text);
  const editedUrl = getStringArg(args, "jobUrl");
  if (editedUrl !== undefined) {
    rejectMockSmartPasteCredentialMaterial(editedUrl);
  }
  const url =
    editedUrl === undefined
      ? parsed.url
      : editedUrl.trim()
        ? canonicalizeMockSmartPasteUrl(editedUrl.trim())
        : "";
  const editedTitle = getStringArg(args, "title") ?? parsed.title;
  const editedCompany = getStringArg(args, "company") ?? parsed.company;
  const editedLocation =
    (getStringArg(args, "location") ?? parsed.location).trim() || null;
  for (const value of [editedTitle, editedCompany, url, editedLocation ?? ""]) {
    rejectMockSmartPasteCredentialMaterial(value);
  }
  checkMockSmartPasteField("job title", editedTitle, 500);
  checkMockSmartPasteField("company name", editedCompany, 200);
  if (editedLocation) {
    checkMockSmartPasteField("location", editedLocation, 200);
  }
  const title = editedTitle.trim();
  const company = editedCompany.trim();
  const location = editedLocation;

  const missingFields: string[] = [];
  if (!title) missingFields.push("title");
  if (!company) missingFields.push("company");
  if (!url) missingFields.push("url");

  const preview: MockJobImportPreview = {
    import_id: null,
    title,
    company,
    url,
    location,
    description_preview: parsed.description
      ? truncateMockSmartPasteDescription(parsed.description)
      : null,
    salary: null,
    date_posted: null,
    valid_through: null,
    employment_types: [],
    remote: false,
    missing_fields: missingFields,
    already_exists: false,
  };
  if (missingFields.length > 0) {
    return withoutSave(state, preview);
  }

  preview.already_exists = state.jobs.some((job) => job.url === url);
  if (preview.already_exists) {
    return withoutSave(state, preview);
  }

  const importId = `mock-paste-${hashString(`${url}:${title}`)}`;
  preview.import_id = importId;
  const pendingUrlImports = [
    ...state.pendingUrlImports.filter((entry) => entry.id !== importId),
    {
      id: importId,
      preview,
      kind: "smartPaste" as const,
      description: parsed.description || null,
      usesSmartPasteDefaults: true,
    },
  ];
  return withoutSave({ ...state, pendingUrlImports }, preview);
}

function buildMockImportedJob(
  preview: MockJobImportPreview,
  id: number,
  createdAt: string,
  description = preview.description_preview,
  usesSmartPasteDefaults = false,
): MockJob {
  return {
    id,
    hash: `mock-import-${hashString(preview.url)}`,
    title: preview.title,
    company: preview.company,
    location: usesSmartPasteDefaults ? preview.location : preview.location ?? "Remote",
    description: description ?? "",
    url: preview.url,
    source: usesSmartPasteDefaults ? "user-source-actions" : "import",
    salary_min: usesSmartPasteDefaults ? null : 55000,
    salary_max: usesSmartPasteDefaults ? null : 72000,
    remote: preview.remote,
    score: usesSmartPasteDefaults ? null : 1,
    hidden: false,
    bookmarked: false,
    notes: null,
    created_at: createdAt,
  } as MockJob;
}

function getJobImportUrl(args: Record<string, unknown> | undefined): string {
  const url = getStringArg(args, "url")?.trim();
  if (!url) {
    throw new Error("Paste the full job link from your browser address bar.");
  }

  let parsedUrl: URL;
  try {
    parsedUrl = new URL(url);
  } catch {
    throw new Error("Paste the full job link from your browser address bar.");
  }

  if (
    parsedUrl.protocol === "http:" &&
    !isLocalOrPrivateHost(parsedUrl.hostname)
  ) {
    throw new Error(
      "Paste an https job posting link from your browser address bar.",
    );
  }

  if (!isSafeExternalHttpsUrl(url)) {
    throw new Error("Paste the full job link from your browser address bar.");
  }

  return canonicalizeMockJobImportUrl(url);
}

function getMockImportTitle(url: string): string {
  const parts = new URL(url).pathname.split("/").filter(Boolean);
  const slug = parts[parts.length - 1] ?? "imported-job";
  return (
    slug
      .replace(/\.[a-z0-9]+$/i, "")
      .replace(/[-_]+/g, " ")
      .replace(/\b\w/g, (char) => char.toUpperCase())
      .trim() || "Imported Job"
  );
}

function hashString(value: string): string {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = (hash * 31 + value.charCodeAt(index)) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function withoutSave(
  state: MockJobImportCommandState,
  value: unknown,
): MockJobImportCommandResult {
  return { handled: true, shouldSave: false, state, value };
}
