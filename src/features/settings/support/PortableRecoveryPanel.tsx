import { useEffect, useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
interface LocalOperationResult {
  outcome: "succeeded" | "cancelled";
  connectivity_required: false;
}
type StagedRestoreStatus =
  | "none"
  | "incomplete"
  | "invalid"
  | "ready"
  | "promoting"
  | "promoted";
type Operation = "backup" | "restore" | "cancel" | "export";
export function PortableRecoveryPanel() {
  const [passphrase, setPassphrase] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [includeProtectedRecords, setIncludeProtectedRecords] = useState(false);
  const [operation, setOperation] = useState<Operation | null>(null);
  const [status, setStatus] = useState("");
  const [stagedRestore, setStagedRestore] = useState<StagedRestoreStatus>("none");
  const busy = operation !== null;
  const incompleteRestore = stagedRestore === "incomplete";
  const invalidRestore = stagedRestore === "invalid";
  const backupReady =
    [...passphrase].length >= 16 && passphrase === confirmation && !busy;
  const restoreReady = [...restorePassphrase].length >= 16 && !busy;
  useEffect(() => {
    void invoke<StagedRestoreStatus>("get_staged_restore_status")
      .then((loaded) => setStagedRestore((current) => current === "none" ? loaded : current))
      .catch(() => {
        setStatus("Staged restore status is unavailable.");
      });
  }, []);
  const createBackup = async () => {
    setOperation("backup");
    setStatus("");
    const submittedPassphrase = passphrase;
    setPassphrase("");
    setConfirmation("");
    try {
      const result = await invoke<LocalOperationResult>(
        "create_portable_backup",
        { passphrase: submittedPassphrase },
      );
      setStatus(
        result.outcome === "succeeded"
          ? "Encrypted backup saved locally."
          : "Encrypted backup was cancelled. Nothing was saved.",
      );
    } catch {
      setStatus("Encrypted backup could not be created. Nothing was saved.");
    } finally {
      setOperation(null);
    }
  };
  const cancelRestore = async (
    cleanup: "staged" | "incomplete" | "invalid" = "staged",
  ) => {
    setOperation("cancel");
    setStatus("");
    try {
      const result = await invoke<{
        outcome: "cancelled" | "not_found";
        connectivity_required: false;
        restart_required: false;
      }>("cancel_staged_restore");
      setStagedRestore("none");
      setStatus(
        result.outcome === "cancelled"
          ? cleanup === "incomplete"
            ? "Incomplete restore files removed locally."
            : cleanup === "invalid"
              ? "Invalid restore files preserved privately and cleared locally."
              : "Staged restore cancelled locally."
          : "No staged restore was found.",
      );
    } catch {
      setStatus("Staged restore could not be cancelled.");
    } finally {
      setOperation(null);
    }
  };
  const stageRestore = async () => {
    setOperation("restore");
    setStatus("");
    const submittedPassphrase = restorePassphrase;
    setRestorePassphrase("");
    try {
      const result = await invoke<{
        outcome: "staged" | "cancelled";
        connectivity_required: false;
        restart_required: boolean;
      }>("stage_portable_restore", { passphrase: submittedPassphrase });
      if (result.outcome === "staged") {
        setStagedRestore("ready");
        setStatus(
          "Restore staged locally. Restart JobSentinel to verify and apply it.",
        );
      } else {
        setStatus("Restore selection was cancelled. Nothing was staged.");
      }
    } catch {
      setStatus("Encrypted backup could not be staged. Nothing was changed.");
    } finally {
      setOperation(null);
    }
  };
  const createReviewedExport = async () => {
    setOperation("export");
    setStatus("");
    try {
      const result = await invoke<LocalOperationResult>(
        "create_reviewed_export",
        { includeProtectedRecords },
      );
      setStatus(
        result.outcome === "succeeded"
          ? "Reviewed plaintext export saved locally."
          : "Reviewed plaintext export was cancelled. Nothing was saved.",
      );
    } catch {
      setStatus("Reviewed plaintext export could not be created.");
    } finally {
      setIncludeProtectedRecords(false);
      setOperation(null);
    }
  };

  return (
    <section
      className="mb-6 rounded-lg border border-surface-200 bg-surface-50 p-4 dark:border-surface-700 dark:bg-surface-800"
      aria-labelledby="portable-recovery-heading"
    >
      <h3
        id="portable-recovery-heading"
        className="font-medium text-surface-800 dark:text-surface-200"
      >
        Encrypted Backup and Recovery
      </h3>
      <p className="mt-2 text-xs font-medium text-green-800 dark:text-green-200">
        Local only. No internet required.
      </p>
      <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
        Credential and secret export remains unavailable.
      </p>

      <div className="mt-4 space-y-3">
        <label className="block text-sm text-surface-700 dark:text-surface-200">
          Backup passphrase
          <input
            type="password"
            autoComplete="new-password"
            value={passphrase}
            onChange={(event) => setPassphrase(event.target.value)}
            disabled={busy}
            className="mt-1 block w-full rounded-md border border-surface-300 bg-white px-3 py-2 dark:border-surface-600 dark:bg-surface-900"
          />
        </label>
        <label className="block text-sm text-surface-700 dark:text-surface-200">
          Confirm backup passphrase
          <input
            type="password"
            autoComplete="new-password"
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            disabled={busy}
            className="mt-1 block w-full rounded-md border border-surface-300 bg-white px-3 py-2 dark:border-surface-600 dark:bg-surface-900"
          />
        </label>
        <p className="text-xs text-surface-500 dark:text-surface-400">
          Use at least 16 characters. JobSentinel cannot recover this
          passphrase.
        </p>
        <Button
          size="sm"
          onClick={() => void createBackup()}
          disabled={!backupReady}
          loading={operation === "backup"}
          loadingText="Creating backup..."
        >
          Create encrypted backup
        </Button>
      </div>

      {stagedRestore === "promoted" && (
        <p className="mt-5 text-sm text-surface-700 dark:text-surface-200">
          The staged restore was verified and applied at startup.
        </p>
      )}

      {stagedRestore === "promoting" && (
        <p className="mt-5 text-sm text-surface-700 dark:text-surface-200">
          The staged restore is being verified and promoted at startup.
        </p>
      )}

      {(stagedRestore === "none" || stagedRestore === "promoted") && (
        <div className="mt-5 space-y-3 border-t border-surface-200 pt-4 dark:border-surface-700">
          <h4 className="text-sm font-medium text-surface-800 dark:text-surface-100">
            Stage a restore
          </h4>
          <p className="text-xs text-surface-500 dark:text-surface-400">
            Choose an encrypted backup locally. It remains cancellable until
            restart and does not change this session.
          </p>
          <label className="block text-sm text-surface-700 dark:text-surface-200">
            Restore passphrase
            <input
              type="password"
              autoComplete="off"
              value={restorePassphrase}
              onChange={(event) => setRestorePassphrase(event.target.value)}
              disabled={busy}
              className="mt-1 block w-full rounded-md border border-surface-300 bg-white px-3 py-2 dark:border-surface-600 dark:bg-surface-900"
            />
          </label>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void stageRestore()}
            disabled={!restoreReady}
            loading={operation === "restore"}
            loadingText="Staging backup..."
          >
            Choose and stage backup
          </Button>
        </div>
      )}

      {(stagedRestore === "ready" || incompleteRestore || invalidRestore) && (
        <div className="mt-5 border-t border-surface-200 pt-4 dark:border-surface-700">
          <p className="text-sm text-surface-700 dark:text-surface-200">
            {incompleteRestore
              ? "An interrupted restore staging attempt was found. Startup will ignore it because it has no authorization marker."
              : invalidRestore
                ? "The restore request is invalid. JobSentinel can preserve verified regular restore files before clearing the blocked request."
                : "A restore is staged. You can cancel it until you restart JobSentinel."}
          </p>
          <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
            {incompleteRestore
              ? "Remove the incomplete files locally before staging another restore. Saved records remain unchanged."
              : invalidRestore
                ? "Linked or unreadable restore files are refused. Saved records remain unchanged."
                : "On restart, JobSentinel verifies the backup before promotion and keeps a private recovery copy if promotion cannot finish."}
          </p>
          <Button
            className="mt-3"
            variant="danger"
            size="sm"
            onClick={() =>
              void cancelRestore(
                incompleteRestore
                  ? "incomplete"
                  : invalidRestore
                    ? "invalid"
                    : "staged",
              )
            }
            disabled={busy}
            loading={operation === "cancel"}
            loadingText={
              incompleteRestore || invalidRestore
                ? "Preserving files..."
                : "Cancelling restore..."
            }
          >
            {incompleteRestore
              ? "Remove incomplete restore files"
              : invalidRestore
                ? "Preserve and remove invalid restore files"
                : "Cancel staged restore"}
          </Button>
        </div>
      )}

      <div className="mt-5 space-y-3 border-t border-surface-200 pt-4 dark:border-surface-700">
        <h4 className="text-sm font-medium text-surface-800 dark:text-surface-100">
          Reviewed plaintext export
        </h4>
        <p className="text-xs text-surface-500 dark:text-surface-400">
          Plaintext can expose private details in user-authored text. The
          native review shows the exact record categories before a save
          location is chosen.
        </p>
        <p className="text-xs text-surface-500 dark:text-surface-400">
          Managed credentials and private application paths are always
          excluded. Existing files are never overwritten.
        </p>
        <label className="flex items-start gap-2 text-xs text-surface-700 dark:text-surface-200">
          <input
            type="checkbox"
            checked={includeProtectedRecords}
            onChange={(event) =>
              setIncludeProtectedRecords(event.target.checked)
            }
            disabled={busy}
            className="mt-0.5"
          />
          Include protected application answers, veteran, disability,
          clearance, and military fields, and structured drafts.
        </label>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => void createReviewedExport()}
          disabled={busy}
          loading={operation === "export"}
          loadingText="Preparing review..."
        >
          Review and create plaintext export
        </Button>
      </div>

      <p
        className="mt-4 text-sm text-surface-700 dark:text-surface-200"
        role="status"
        aria-label="Portable recovery status"
        aria-live="polite"
      >
        {status}
      </p>
    </section>
  );
}
