import { useEffect, useState } from "react";
import { invoke } from "../platform/tauri";
import { default as ErrorBoundary } from "./errors/ErrorBoundary";
import { SkipToContent } from "../ui/SkipToContent";
import { logError } from "../shared/errorReporting/logger";
import {
  copySanitizedDebugReport,
  saveSanitizedDebugReport,
} from "../shared/errorReporting/supportReport";

type StagedRestoreStatus =
  | "none"
  | "incomplete"
  | "invalid"
  | "ready"
  | "promoting"
  | "promoted";

export function StartupRecovery({ onRetry }: { onRetry: () => void }) {
  const [recovery, setRecovery] = useState<{
    required: boolean;
    platform: boolean;
    configuration: boolean;
    database: boolean;
    connectivity_required: false;
  } | null>(null);
  const [repairing, setRepairing] = useState(false);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [stagedRestore, setStagedRestore] =
    useState<StagedRestoreStatus>("none");
  const [recoveryStatus, setRecoveryStatus] = useState("");
  const [reportStatus, setReportStatus] = useState<
    "idle" | "copying" | "copied" | "saving" | "saved" | "failed"
  >("idle");

  useEffect(() => {
    void (async () => {
      try {
        const status = await invoke<NonNullable<typeof recovery>>(
          "get_startup_recovery_status",
        );
        setRecovery(status ?? null);
        if (status?.database && !status.platform) {
          try {
            const staged = await invoke<StagedRestoreStatus>(
              "get_staged_restore_status",
            );
            setStagedRestore(staged ?? "none");
          } catch {
            setRecoveryStatus(
              "Restore status is unavailable. Saved data was not changed.",
            );
          }
        }
      } catch {
        setRecovery(null);
      }
    })();
  }, []);

  const repairConfig = async () => {
    setRepairing(true);
    setRecoveryStatus("");
    try {
      await invoke("repair_invalid_startup_config");
      setRecoveryStatus(
        "Saved settings were preserved locally. Restart JobSentinel to continue.",
      );
    } catch {
      setRecoveryStatus(
        "Saved settings could not be reset safely. No saved file was removed.",
      );
    } finally {
      setRepairing(false);
    }
  };

  const repairPermissions = async () => {
    setRepairing(true);
    setRecoveryStatus("");
    try {
      const results = await Promise.all(
        ["application_data", "configuration", "cache"].map((area) =>
          invoke<{ outcome: string }>("repair_local_permissions", { area }),
        ),
      );
      setRecoveryStatus(
        results.every((result) => result.outcome === "repaired")
          ? "Local permissions were repaired. Restart JobSentinel to continue."
          : "Some local permissions need manual review in your system settings.",
      );
    } catch {
      setRecoveryStatus(
        "Local permissions could not be repaired automatically. No internet connection was used.",
      );
    } finally {
      setRepairing(false);
    }
  };

  const stageRestore = async () => {
    setRepairing(true);
    setRecoveryStatus("");
    const passphrase = restorePassphrase;
    setRestorePassphrase("");
    try {
      const result = await invoke<{ outcome: "staged" | "cancelled" }>(
        "stage_portable_restore",
        { passphrase },
      );
      if (result.outcome === "staged") {
        setStagedRestore("ready");
      }
      setRecoveryStatus(
        result.outcome === "staged"
          ? "Encrypted restore is staged. Close and reopen JobSentinel to finish."
          : "Encrypted restore was cancelled. Saved data was not changed.",
      );
    } catch {
      setRecoveryStatus(
        "Encrypted restore could not be staged. Saved data was not changed.",
      );
    } finally {
      setRepairing(false);
    }
  };

  const cancelRestore = async () => {
    setRepairing(true);
    setRecoveryStatus("");
    const invalid = stagedRestore === "invalid";
    try {
      const result = await invoke<{ outcome: "cancelled" | "not_found" }>(
        "cancel_staged_restore",
      );
      setStagedRestore("none");
      setRecoveryStatus(
        result.outcome === "cancelled"
          ? invalid
            ? "Invalid restore files were preserved privately and cleared from startup. Saved data was not changed."
            : "Staged restore was cancelled. Saved data was not changed."
          : "No staged restore was found. Saved data was not changed.",
      );
    } catch {
      setRecoveryStatus(
        "Staged restore could not be cancelled. Saved data was not changed.",
      );
    } finally {
      setRepairing(false);
    }
  };

  const copyReport = async () => {
    setReportStatus("copying");
    try {
      await copySanitizedDebugReport();
      setReportStatus("copied");
    } catch (error) {
      logError("Failed to copy startup support report:", error);
      setReportStatus("failed");
    }
  };

  const saveReport = async () => {
    setReportStatus("saving");
    try {
      const saved = await saveSanitizedDebugReport();
      setReportStatus(saved ? "saved" : "idle");
    } catch (error) {
      logError("Failed to save startup support report:", error);
      setReportStatus("failed");
    }
  };

  return (
    <ErrorBoundary>
      <SkipToContent />
      <main
        className="min-h-screen bg-surface-950 text-white flex items-center justify-center px-6"
        id="main-content"
        tabIndex={-1}
      >
        <section className="w-full max-w-lg rounded-card border border-surface-700 bg-surface-900 p-8 shadow-xl">
          <h1 className="font-display text-display-lg mb-3">
            JobSentinel could not open saved setup
          </h1>
          <p className="text-sm text-surface-300 mb-6">
            {recovery?.required
              ? "Your saved jobs and settings stay on this device. Use an available local repair, then close and reopen JobSentinel."
              : "Your saved jobs and settings stay on this device. Try again, or save a safe support report before closing and reopening JobSentinel."}
          </p>
          <div className="space-y-3">
            {recovery?.database && recovery.platform && (
              <p className="rounded-lg border border-surface-700 p-4 text-sm text-surface-300">
                Repair local permissions and restart before restoring a backup.
              </p>
            )}
            {recovery?.database &&
              !recovery.platform &&
              stagedRestore === "none" && (
              <div className="rounded-lg border border-surface-700 p-4">
                <p className="mb-3 text-sm text-surface-300">
                  Restore from a JobSentinel encrypted backup. No internet is
                  required. The restore is staged first, can be cancelled, and
                  keeps a private recovery copy when applied after restart.
                  Credentials and secrets are not restored.
                </p>
                <label
                  className="mb-3 block text-sm text-surface-200"
                  htmlFor="startup-restore-passphrase"
                >
                  Backup passphrase
                  <input
                    id="startup-restore-passphrase"
                    type="password"
                    autoComplete="off"
                    value={restorePassphrase}
                    disabled={repairing}
                    onChange={(event) =>
                      setRestorePassphrase(event.target.value)
                    }
                    className="mt-1 block w-full rounded-lg border border-surface-600 bg-surface-950 px-3 py-2"
                  />
                </label>
                <button
                  className="w-full rounded-lg bg-sentinel-500 px-4 py-3 text-sm font-semibold text-white hover:bg-sentinel-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-sentinel-400 disabled:opacity-50"
                  disabled={
                    repairing || Array.from(restorePassphrase).length < 16
                  }
                  onClick={() => void stageRestore()}
                >
                  Choose backup and stage restore
                </button>
              </div>
            )}
            {recovery?.database &&
              !recovery.platform &&
              (stagedRestore === "ready" ||
                stagedRestore === "incomplete" ||
                stagedRestore === "invalid") && (
                <div className="rounded-lg border border-surface-700 p-4">
                  <p className="mb-3 text-sm text-surface-300">
                    {stagedRestore === "ready"
                      ? "An encrypted restore is staged. Cancel it before closing JobSentinel if you do not want it applied at restart."
                      : stagedRestore === "incomplete"
                        ? "An interrupted restore stage was found. Startup will ignore it until you remove the incomplete local files."
                        : "The restore request is invalid. JobSentinel can preserve verified regular restore files under private local names, then clear the blocked startup request."}
                  </p>
                  <button
                    className="w-full rounded-lg bg-surface-800 px-4 py-3 text-sm font-semibold text-surface-100 hover:bg-surface-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-surface-400 disabled:opacity-50"
                    disabled={repairing}
                    onClick={() => void cancelRestore()}
                  >
                    {stagedRestore === "ready"
                      ? "Cancel staged restore"
                      : stagedRestore === "incomplete"
                        ? "Remove incomplete restore files"
                        : "Preserve and remove invalid restore files"}
                  </button>
                </div>
              )}
            {recovery?.database &&
              !recovery.platform &&
              (stagedRestore === "promoting" ||
                stagedRestore === "promoted") && (
                <p className="rounded-lg border border-surface-700 p-4 text-sm text-surface-300">
                  Restore promotion is already in progress. Close and reopen
                  JobSentinel to finish verification or automatic recovery.
                </p>
              )}
            {recovery?.configuration && (
              <button
                className="w-full rounded-lg bg-sentinel-500 px-4 py-3 text-sm font-semibold text-white hover:bg-sentinel-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-sentinel-400 disabled:opacity-50"
                disabled={repairing}
                onClick={() => void repairConfig()}
              >
                Preserve and reset saved settings
              </button>
            )}
            {recovery?.platform && (
              <button
                className="w-full rounded-lg bg-sentinel-500 px-4 py-3 text-sm font-semibold text-white hover:bg-sentinel-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-sentinel-400 disabled:opacity-50"
                disabled={repairing}
                onClick={() => void repairPermissions()}
              >
                Repair local permissions
              </button>
            )}
            {!recovery?.required && (
              <button
                className="w-full rounded-lg bg-sentinel-500 px-4 py-3 text-sm font-semibold text-white hover:bg-sentinel-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-sentinel-400"
                onClick={onRetry}
              >
                Try Again
              </button>
            )}
            <button
              className="w-full rounded-lg bg-surface-800 px-4 py-3 text-sm font-semibold text-surface-100 hover:bg-surface-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-surface-400 disabled:opacity-50"
              disabled={reportStatus === "copying"}
              onClick={() => void copyReport()}
            >
              {reportStatus === "copying"
                ? "Copying..."
                : "Copy Safe Support Report"}
            </button>
            <button
              className="w-full rounded-lg bg-surface-800 px-4 py-3 text-sm font-semibold text-surface-100 hover:bg-surface-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-surface-400 disabled:opacity-50"
              disabled={reportStatus === "saving"}
              onClick={() => void saveReport()}
            >
              {reportStatus === "saving"
                ? "Saving..."
                : "Save Safe Support Report"}
            </button>
          </div>
          {recoveryStatus && (
            <p
              className="mt-4 text-sm text-surface-200"
              role="status"
              aria-label="Startup recovery status"
              aria-live="polite"
            >
              {recoveryStatus}
            </p>
          )}
          {reportStatus === "copied" && (
            <p className="mt-4 text-sm text-success" role="status">
              Safe support report copied
            </p>
          )}
          {reportStatus === "saved" && (
            <p className="mt-4 text-sm text-success" role="status">
              Safe support report saved for review
            </p>
          )}
          {reportStatus === "failed" && (
            <p className="mt-4 text-sm text-danger" role="status">
              Could not create safe support report
            </p>
          )}
        </section>
      </main>
    </ErrorBoundary>
  );
}
