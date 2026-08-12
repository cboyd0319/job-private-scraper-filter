import { useState } from "react";
import { Button } from "../../../ui/Button";
import { ModalFooter } from "../../../ui/Modal";
import { invalidateCacheByCommand, invoke } from "../../../platform/tauri";
import { useToast } from "../../../shared/toast/useToast";
import {
  formatMissingDetails,
  getSafeJobImportError,
  type JobImportPreview,
  type JobImportResult,
} from "./jobImportModel";

interface SmartPasteFlowProps {
  onClose: () => void;
  onUseLink: () => void;
  onImportSuccess?: () => void;
}

type DraftField = "title" | "company" | "url" | "location";

export function SmartPasteFlow({
  onClose,
  onUseLink,
  onImportSuccess,
}: SmartPasteFlowProps) {
  const [text, setText] = useState("");
  const [draft, setDraft] = useState<JobImportPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();

  const stage = async (edited: boolean) => {
    if (!text.trim()) {
      setError("Paste copied job details first.");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const args = edited
        ? {
            text,
            title: draft?.title ?? "",
            company: draft?.company ?? "",
            jobUrl: draft?.url ?? "",
            location: draft?.location ?? null,
          }
        : { text };
      const preview = await invoke<JobImportPreview>(
        "preview_smart_paste",
        args,
      );
      setDraft(preview);
    } catch (cause) {
      const safe = getSafeJobImportError(cause);
      setError(safe.message);
      toast.error("Could not create draft", safe.action);
    } finally {
      setLoading(false);
    }
  };

  const updateDraft = (field: DraftField, value: string) => {
    if (!draft) return;
    const next = {
      ...draft,
      [field]: field === "location" ? value || null : value,
      import_id: null,
    };
    next.missing_fields = [];
    if (!next.title.trim()) next.missing_fields.push("title");
    if (!next.company.trim()) next.missing_fields.push("company");
    if (!next.url.trim()) next.missing_fields.push("url");
    setDraft(next);
  };

  const save = async () => {
    if (!draft?.import_id || draft.already_exists) return;
    setSaving(true);
    setError(null);
    try {
      await invoke<JobImportResult>("confirm_smart_paste", {
        importId: draft.import_id,
      });
      invalidateCacheByCommand("get_recent_jobs");
      invalidateCacheByCommand("get_statistics");
      toast.success("Job saved", `Saved "${draft.title}"`);
      onImportSuccess?.();
      onClose();
    } catch (cause) {
      const safe = getSafeJobImportError(cause);
      setError(safe.message);
      toast.error("Could not save job", safe.action);
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="space-y-4">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          Smart Paste reads copied text only and keeps it local. Put the job
          title first and company second. Include or add the public job link,
          then review every field before saving. It does not read screenshots.
        </p>

        {!draft ? (
          <>
            <label
              htmlFor="smart-paste-text"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Pasted job details
            </label>
            <textarea
              id="smart-paste-text"
              value={text}
              onChange={(event) => setText(event.target.value)}
              maxLength={50_000}
              rows={10}
              autoFocus
              disabled={loading}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
              placeholder={
                "Office Manager\nExample Services\nhttps://example.com/jobs/office-manager\nCoordinate office operations."
              }
            />
            <div className="flex justify-end">
              <Button
                onClick={() => stage(false)}
                loading={loading}
                disabled={loading}
                variant="primary"
              >
                {loading ? "Creating draft..." : "Create Draft"}
              </Button>
            </div>
          </>
        ) : (
          <div className="space-y-3 rounded-lg border border-gray-200 bg-gray-50 p-4 dark:border-gray-700 dark:bg-gray-800/50">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              Review Draft
            </h3>
            <DraftInput
              label="Job title"
              value={draft.title}
              onChange={(value) => updateDraft("title", value)}
            />
            <DraftInput
              label="Company"
              value={draft.company}
              onChange={(value) => updateDraft("company", value)}
            />
            <DraftInput
              label="Job link"
              type="url"
              value={draft.url}
              onChange={(value) => updateDraft("url", value)}
            />
            <DraftInput
              label="Location"
              value={draft.location ?? ""}
              onChange={(value) => updateDraft("location", value)}
            />
            {draft.description_preview && (
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <span className="font-medium text-gray-700 dark:text-gray-300">
                  Description preview:
                </span>{" "}
                {draft.description_preview}
              </p>
            )}
            {draft.missing_fields.length > 0 && (
              <p className="text-sm text-amber-800 dark:text-amber-200">
                Add: {formatMissingDetails(draft.missing_fields)}.
              </p>
            )}
            {draft.already_exists && (
              <p className="text-sm text-blue-800 dark:text-blue-200">
                This job is already in your saved jobs.
              </p>
            )}
          </div>
        )}

        {error && (
          <div
            role="alert"
            className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-200"
          >
            {error}
          </div>
        )}
      </div>

      <ModalFooter>
        <Button onClick={onUseLink} variant="secondary" disabled={saving}>
          Use a Job Link
        </Button>
        {draft && (
          <Button
            onClick={() => {
              setDraft(null);
              setError(null);
            }}
            variant="secondary"
            disabled={saving}
          >
            Change Paste
          </Button>
        )}
        <Button onClick={onClose} variant="secondary" disabled={saving}>
          Cancel
        </Button>
        {draft && !draft.import_id && !draft.already_exists && (
          <Button
            onClick={() => stage(true)}
            variant="primary"
            loading={loading}
            disabled={loading || draft.missing_fields.length > 0}
          >
            {loading ? "Reviewing..." : "Review Draft"}
          </Button>
        )}
        {draft?.import_id && !draft.already_exists && (
          <Button
            onClick={save}
            variant="primary"
            loading={saving}
            disabled={saving}
          >
            {saving ? "Saving..." : "Save Job"}
          </Button>
        )}
      </ModalFooter>
    </>
  );
}

function DraftInput({
  label,
  value,
  type = "text",
  onChange,
}: {
  label: string;
  value: string;
  type?: "text" | "url";
  onChange: (value: string) => void;
}) {
  const id = `smart-paste-${label.toLowerCase().replace(/ /g, "-")}`;
  return (
    <div>
      <label
        htmlFor={id}
        className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300"
      >
        {label}
      </label>
      <input
        id={id}
        type={type}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="w-full rounded-lg border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
      />
    </div>
  );
}
