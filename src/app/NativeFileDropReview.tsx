/** Routes one native dropped file through explicit local review without exposing its source path. */

import { useEffect, useRef, useState } from "react";
import { invalidateCacheByCommand, invoke } from "../platform/tauri";
import { listen } from "../platform/tauri/events";
import { Button } from "../ui/Button";
import { Modal, ModalFooter } from "../ui/Modal";
import {
  formatMissingDetails,
  type JobImportPreview,
  type JobImportResult,
} from "../features/dashboard/components/jobImportModel";

type NativeFileDropPayload = {
  dropId: string | null;
  name: string | null;
  error: string | null;
};
type Drop = { dropId: string; name: string };
type View = "choices" | "job" | "backup";
type DraftField = "title" | "company" | "url" | "location";
type PackStageState =
  "needs_review" | "ready" | "disabled" | "quarantined" | "removed";

function packStageState(value: unknown): PackStageState | null {
  if (!value || typeof value !== "object") return null;
  const result = value as Record<string, unknown>;
  return typeof result.publisherKeyId === "string" &&
    result.publisherKeyId.length > 0 &&
    typeof result.packId === "string" &&
    result.packId.length > 0 &&
    Number.isSafeInteger(result.generation) &&
    Number(result.generation) > 0 &&
    Number.isSafeInteger(result.releaseSequence) &&
    Number(result.releaseSequence) > 0 &&
    ["needs_review", "ready", "disabled", "quarantined", "removed"].includes(
      String(result.state),
    )
    ? (result.state as PackStageState)
    : null;
}

const PACK_STAGE_STATUS: Record<PackStageState, string> = {
  needs_review: "was staged locally for review in Settings > Packs.",
  ready: "is already active. Review its signed facts in Settings > Packs.",
  disabled: "is already installed and disabled. Review it in Settings > Packs.",
  quarantined:
    "is already quarantined. Review its recovery guidance in Settings > Packs.",
  removed:
    "was previously removed. Its signed history remains in Settings > Packs.",
};

function validDrop(payload: unknown): Drop | null {
  if (!payload || typeof payload !== "object") return null;
  const { dropId, name, error } = payload as NativeFileDropPayload;
  if (error || typeof dropId !== "string" || typeof name !== "string")
    return null;
  const safeName = name.trim().split(/[\\/]/).pop()?.trim();
  return dropId.trim() && safeName ? { dropId, name: safeName } : null;
}

function missingFields(draft: JobImportPreview): string[] {
  return [
    !draft.title.trim() && "title",
    !draft.company.trim() && "company",
    !draft.url.trim() && "url",
  ].filter((field): field is string => Boolean(field));
}

export function NativeFileDropReview() {
  const currentDropId = useRef<string | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);
  const [view, setView] = useState<View>("choices");
  const [draft, setDraft] = useState<JobImportPreview | null>(null);
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [alert, setAlert] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const passphraseIsLongEnough = [...passphrase].length >= 16;
  const isSignedPack = drop?.name.toLowerCase().endsWith(".jspack") ?? false;

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listen<NativeFileDropPayload>("native-file-drop", (event) => {
      const nextDrop = validDrop(event.payload);
      if (!nextDrop) {
        currentDropId.current = null;
        setDrop(null);
        setDraft(null);
        setPassphrase("");
        setAlert(
          "Drop one regular file, or use the existing import buttons. Nothing was imported.",
        );
        return;
      }
      currentDropId.current = nextDrop.dropId;
      setDrop(nextDrop);
      setView("choices");
      setDraft(null);
      setPassphrase("");
      setAlert(null);
      setStatus(null);
    })
      .then((stop) => {
        if (active) unlisten = stop;
        else stop();
      })
      .catch(() =>
        setAlert("File drop review is unavailable. Nothing was imported."),
      );
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const finishReview = (dropId: string) => {
    if (currentDropId.current !== dropId) return;
    currentDropId.current = null;
    setDrop(null);
    setDraft(null);
    setPassphrase("");
    setView("choices");
  };

  const dismiss = async () => {
    if (!drop || busy) return;
    const dropId = drop.dropId;
    const name = drop.name;
    setBusy(true);
    try {
      await invoke("discard_native_file_drop", { dropId });
    } catch {
      setAlert(`${name} could not be discarded. Nothing was imported.`);
    } finally {
      setBusy(false);
      finishReview(dropId);
    }
  };

  const importResume = async () => {
    if (!drop) return;
    const dropId = drop.dropId;
    const name = drop.name;
    setBusy(true);
    setAlert(null);
    try {
      await invoke("import_dropped_resume", { dropId });
      finishReview(dropId);
      setStatus(`${name} was imported locally. Review it on the Resumes page.`);
    } catch {
      setAlert(`${name} could not be imported as a resume. Nothing was saved.`);
    } finally {
      setBusy(false);
    }
  };

  const stagePack = async () => {
    if (!drop || !isSignedPack) return;
    const dropId = drop.dropId;
    const name = drop.name;
    setBusy(true);
    setAlert(null);
    try {
      const result = packStageState(
        await invoke("stage_dropped_pack", { dropId }),
      );
      finishReview(dropId);
      if (result) setStatus(`${name} ${PACK_STAGE_STATUS[result]}`);
      else
        setAlert(
          "Signed pack state could not be confirmed. Refresh Settings > Packs before continuing.",
        );
    } catch {
      setAlert(
        `${name} could not be verified and staged. Nothing was activated.`,
      );
    } finally {
      setBusy(false);
    }
  };

  const previewJob = async (edited: boolean) => {
    if (!drop) return;
    const dropId = drop.dropId;
    setBusy(true);
    setAlert(null);
    try {
      const args =
        edited && draft
          ? {
              dropId,
              title: draft.title,
              company: draft.company,
              jobUrl: draft.url,
              location: draft.location,
            }
          : { dropId };
      const preview = await invoke<JobImportPreview>(
        "preview_dropped_job",
        args,
      );
      if (currentDropId.current !== dropId) return;
      setDraft(preview);
      setView("job");
    } catch {
      if (currentDropId.current === dropId) {
        setAlert("The job posting could not be reviewed. Nothing was saved.");
      }
    } finally {
      setBusy(false);
    }
  };

  const updateDraft = (field: DraftField, value: string) => {
    if (!draft) return;
    const next = {
      ...draft,
      [field]: field === "location" ? value || null : value,
      import_id: null,
    };
    next.missing_fields = missingFields(next);
    setDraft(next);
  };

  const saveJob = async () => {
    if (!drop || !draft?.import_id || draft.already_exists) return;
    const dropId = drop.dropId;
    const name = drop.name;
    setBusy(true);
    setAlert(null);
    try {
      await invoke<JobImportResult>("confirm_smart_paste", {
        importId: draft.import_id,
      });
      invalidateCacheByCommand("get_recent_jobs");
      invalidateCacheByCommand("get_statistics");
      try {
        await invoke("discard_native_file_drop", { dropId });
      } catch {
        // A newer drop can make this token stale. The old reference also
        // clears when the process exits.
      }
      finishReview(dropId);
      setStatus(`${name} was saved locally as a job.`);
    } catch {
      setAlert(`${name} could not be saved as a job. Nothing new was saved.`);
    } finally {
      setBusy(false);
    }
  };

  const stageBackup = async () => {
    if (!drop || !passphraseIsLongEnough) return;
    const dropId = drop.dropId;
    const name = drop.name;
    setBusy(true);
    setAlert(null);
    try {
      await invoke("stage_dropped_portable_restore", { dropId, passphrase });
      finishReview(dropId);
      setStatus(
        `${name} is staged locally. Restart is required; the current session is unchanged.`,
      );
    } catch {
      setAlert(`${name} could not be staged as a backup. Nothing was changed.`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      {(alert || status) && (
        <div
          role={alert ? "alert" : "status"}
          aria-label={
            alert ? "Native file drop alert" : "Native file drop status"
          }
          className="fixed bottom-4 right-4 z-[1100] flex max-w-md items-start gap-3 rounded-lg border border-surface-300 bg-white p-4 text-sm shadow-lg dark:border-surface-600 dark:bg-surface-800"
        >
          <p className="min-w-0 flex-1 break-words">{alert ?? status}</p>
          <button
            type="button"
            className="shrink-0 underline"
            onClick={() => {
              setAlert(null);
              setStatus(null);
            }}
          >
            Dismiss notification
          </button>
        </div>
      )}
      <Modal
        isOpen={Boolean(drop)}
        onClose={() => void dismiss()}
        title="Review dropped file"
        description="The dropped file stays local. Nothing is saved until you explicitly review it."
        size="lg"
        closeOnOverlayClick={!busy}
        showCloseButton={!busy}
        closeButtonLabel="Close file drop review"
      >
        <div className="space-y-4">
          <p className="break-words text-sm text-surface-600 dark:text-surface-300">
            <span className="font-semibold">{drop?.name}</span> stays local.
            Nothing is saved until you explicitly review it.
          </p>
          {view === "choices" && (
            <>
              <div
                data-testid="native-file-drop-actions"
                className="flex flex-col gap-3 sm:flex-row sm:flex-wrap"
              >
                {isSignedPack ? (
                  <Button onClick={() => void stagePack()} disabled={busy}>
                    Verify and stage signed pack
                  </Button>
                ) : (
                  <>
                    <Button onClick={importResume} disabled={busy}>
                      Add resume
                    </Button>
                    <Button
                      onClick={() => void previewJob(false)}
                      variant="secondary"
                      disabled={busy}
                    >
                      Review job posting
                    </Button>
                    <Button
                      onClick={() => setView("backup")}
                      variant="secondary"
                      disabled={busy}
                    >
                      Backup/Recovery
                    </Button>
                  </>
                )}
              </div>
              <p className="text-sm text-surface-600 dark:text-surface-300">
                {isSignedPack
                  ? "Verification stages the pack for a separate signed-permission review. It does not activate it."
                  : "You can instead use Add resume, Import Job, or Backup/Recovery from the keyboard."}
              </p>
            </>
          )}
          {view === "job" && draft && (
            <JobReview
              draft={draft}
              busy={busy}
              onChange={updateDraft}
              onReview={() => void previewJob(true)}
              onSave={() => void saveJob()}
            />
          )}
          {view === "backup" && (
            <div className="space-y-3">
              <label
                htmlFor="dropped-backup-passphrase"
                className="block text-sm font-medium"
              >
                Backup passphrase
              </label>
              <input
                id="dropped-backup-passphrase"
                type="password"
                value={passphrase}
                onChange={(event) => setPassphrase(event.target.value)}
                className="w-full rounded-lg border border-surface-300 px-3 py-2 dark:border-surface-600 dark:bg-surface-800"
              />
              <p className="text-sm text-surface-600 dark:text-surface-300">
                Use at least 16 characters. Staging requires a restart and does
                not change this session.
              </p>
              <Button
                onClick={() => void stageBackup()}
                disabled={busy || !passphraseIsLongEnough}
              >
                Stage backup
              </Button>
            </div>
          )}
        </div>
        <ModalFooter className="flex flex-col gap-2 sm:flex-row sm:justify-between">
          {view !== "choices" && (
            <Button
              onClick={() => {
                setDraft(null);
                setPassphrase("");
                setView("choices");
              }}
              variant="secondary"
              disabled={busy}
            >
              Choose another option
            </Button>
          )}
          <Button
            onClick={() => void dismiss()}
            variant="secondary"
            disabled={busy}
          >
            Cancel
          </Button>
        </ModalFooter>
      </Modal>
    </>
  );
}

function JobReview({
  draft,
  busy,
  onChange,
  onReview,
  onSave,
}: {
  draft: JobImportPreview;
  busy: boolean;
  onChange: (field: DraftField, value: string) => void;
  onReview: () => void;
  onSave: () => void;
}) {
  const fields: Array<[DraftField, string, "text" | "url"]> = [
    ["title", "Job title", "text"],
    ["company", "Company", "text"],
    ["url", "Job link", "url"],
    ["location", "Location", "text"],
  ];
  return (
    <div className="space-y-3 rounded-lg border border-surface-200 p-4 dark:border-surface-700">
      <h3 className="text-lg font-semibold">Review Draft</h3>
      {fields.map(([field, label, type]) => (
        <label key={field} className="block text-sm font-medium">
          {label}
          <input
            type={type}
            value={draft[field] ?? ""}
            onChange={(event) => onChange(field, event.target.value)}
            className="mt-1 w-full rounded-lg border border-surface-300 px-3 py-2 dark:border-surface-600 dark:bg-surface-800"
          />
        </label>
      ))}
      {draft.description_preview && (
        <p className="text-sm text-surface-600 dark:text-surface-300">
          Description preview: {draft.description_preview}
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
      {!draft.import_id && !draft.already_exists && (
        <Button
          onClick={onReview}
          disabled={busy || draft.missing_fields.length > 0}
        >
          Review Draft
        </Button>
      )}
      {draft.import_id && !draft.already_exists && (
        <Button onClick={onSave} disabled={busy}>
          Save Job
        </Button>
      )}
    </div>
  );
}
