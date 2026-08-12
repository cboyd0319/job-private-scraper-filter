export interface BookmarkletConfig {
  port: number;
  enabled: boolean;
}

export interface PendingBookmarkletImport {
  id: string;
  title: string;
  company: string;
  url: string;
  location: string | null;
  description_preview: string | null;
  remote: boolean;
  operation: "visible_page_capture" | "applied_logging";
  missing_fields: ("title" | "company")[];
  received_at: string;
}

export interface BookmarkletImportConfirmResult {
  imported: number;
  skipped: number;
}

export interface DiscardBookmarkletImportsResponse {
  discarded: number;
}

export const DEFAULT_BOOKMARKLET_CONFIG: BookmarkletConfig = {
  port: 4321,
  enabled: false,
};

export function isBookmarkletConfig(
  value: unknown,
): value is BookmarkletConfig {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as BookmarkletConfig).port === "number" &&
    typeof (value as BookmarkletConfig).enabled === "boolean"
  );
}

function isPendingBookmarkletImport(
  value: unknown,
): value is PendingBookmarkletImport {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const item = value as PendingBookmarkletImport;
  return (
    typeof item.id === "string" &&
    typeof item.title === "string" &&
    typeof item.company === "string" &&
    typeof item.url === "string" &&
    (item.location === null || typeof item.location === "string") &&
    (item.description_preview === null ||
      typeof item.description_preview === "string") &&
    typeof item.remote === "boolean" &&
    (item.operation === "visible_page_capture" ||
      item.operation === "applied_logging") &&
    Array.isArray(item.missing_fields) &&
    item.missing_fields.every(
      (field) => field === "title" || field === "company",
    ) &&
    typeof item.received_at === "string"
  );
}

export function isPendingBookmarkletImportList(
  value: unknown,
): value is PendingBookmarkletImport[] {
  return Array.isArray(value) && value.every(isPendingBookmarkletImport);
}

export function pendingActionKey(action: "save" | "skip", ids: string[]) {
  return `${action}:${ids.join(",")}`;
}
