import { Button } from "../../../../ui/Button";
import {
  type PendingBookmarkletImport,
  pendingActionKey,
} from "./browserImportModel";

interface BrowserImportReviewListProps {
  action: string | null;
  error: string | null;
  imports: PendingBookmarkletImport[];
  loading: boolean;
  message: string | null;
  onRefresh: () => void;
  onSave: (ids: string[]) => void;
  onSkip: (ids: string[]) => void;
}

export function BrowserImportReviewList({
  action,
  error,
  imports,
  loading,
  message,
  onRefresh,
  onSave,
  onSkip,
}: BrowserImportReviewListProps) {
  if (!message && !error && imports.length === 0) {
    return null;
  }

  const ids = imports.map((item) => item.id);

  return (
    <div className="rounded-lg border border-sentinel-500/30 bg-sentinel-500/10 p-4">
      <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold text-sentinel-100">
            Jobs waiting for review
          </h4>
          <p className="mt-1 text-sm text-sentinel-50/90">
            These jobs are not saved yet. Check the details, then save or skip
            them.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={onRefresh}
            size="sm"
            variant="secondary"
            disabled={loading || action !== null}
          >
            {loading ? "Refreshing..." : "Refresh Review List"}
          </Button>
          {imports.length > 1 && (
            <>
              <Button
                onClick={() => onSave(ids)}
                size="sm"
                variant="primary"
                disabled={action !== null}
              >
                Save All
              </Button>
              <Button
                onClick={() => onSkip(ids)}
                size="sm"
                variant="ghost"
                disabled={action !== null}
              >
                Skip All
              </Button>
            </>
          )}
        </div>
      </div>

      {message && (
        <p role="status" className="mb-3 text-sm text-green-200">
          {message}
        </p>
      )}
      {error && (
        <p role="alert" className="mb-3 text-sm text-red-200">
          {error}
        </p>
      )}

      {imports.length > 0 && (
        <div className="space-y-3">
          {imports.map((item) => {
            const saveKey = pendingActionKey("save", [item.id]);
            const skipKey = pendingActionKey("skip", [item.id]);
            const appliedDraft = item.operation === "applied_logging";
            const missingDetails = item.missing_fields.map((field) =>
              field === "title" ? "Job title" : "Company",
            );
            return (
              <div
                key={item.id}
                className="rounded-lg border border-white/10 bg-gray-900/60 p-3"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    {appliedDraft && (
                      <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-blue-300">
                        Applied draft
                      </p>
                    )}
                    <p className="font-medium text-white">{item.title}</p>
                    <p className="text-sm text-gray-300">
                      {item.company}
                      {item.location ? ` · ${item.location}` : ""}
                      {item.remote ? " · Remote" : ""}
                    </p>
                    {item.description_preview && (
                      <p className="mt-2 text-xs leading-5 text-gray-400">
                        {item.description_preview}
                      </p>
                    )}
                    {missingDetails.length > 0 && (
                      <p className="mt-2 text-xs font-medium text-amber-200">
                        Missing details: {missingDetails.join(", ")}
                      </p>
                    )}
                    <p className="mt-2 break-all text-xs text-gray-500">
                      {item.url}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      onClick={() => onSave([item.id])}
                      size="sm"
                      variant="primary"
                      disabled={action !== null}
                      aria-label={
                        appliedDraft
                          ? "Save applied draft"
                          : `Save ${item.title}`
                      }
                    >
                      {action === saveKey
                        ? "Saving..."
                        : appliedDraft
                          ? "Save Applied Draft"
                          : "Save Job"}
                    </Button>
                    <Button
                      onClick={() => onSkip([item.id])}
                      size="sm"
                      variant="ghost"
                      disabled={action !== null}
                      aria-label={`Skip ${item.title}`}
                    >
                      {action === skipKey ? "Skipping..." : "Skip"}
                    </Button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
