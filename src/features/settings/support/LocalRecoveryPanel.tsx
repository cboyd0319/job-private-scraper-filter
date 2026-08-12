import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";

type PrivacyState =
  | "looks_good"
  | "needs_attention"
  | "paused_for_safety"
  | "optional_improvement";
type PrivacyAction =
  | "create_portable_backup"
  | "review_backup"
  | "unlock_credential_vault"
  | "review_recovery"
  | "review_external_ai"
  | "refresh_browser_import_code"
  | "review_source_safety";
type StorageArea = "application_data" | "configuration" | "cache";

interface LocalStorageRecoveryReport {
  state: "ready" | "restore_from_backup_required" | "unavailable";
  reclaimable_bytes: number;
  wal_bytes: number | null;
  incremental_vacuum_supported: boolean;
  cleanup_available: boolean;
  connectivity_required: false;
}

interface LocalRecoveryReport {
  schema_version: number;
  connectivity_required: false;
  queued_local_work: {
    pending_url_imports: number;
    capacity: number;
    available_offline: true;
    connectivity_required: false;
  };
  storage: LocalStorageRecoveryReport;
  privacy_doctor: {
    schema_version: number;
    overall: PrivacyState;
    checks: {
      id: string;
      state: PrivacyState;
      message: string;
      action: PrivacyAction | null;
      connectivity_required: false;
    }[];
    connectivity_required: false;
  };
  platform_health: {
    schema_version: number;
    permissions: {
      area: StorageArea;
      state:
        | "private"
        | "missing"
        | "needs_repair"
        | "manual_review"
        | "unchecked";
      action: null | "repair_locally" | "follow_manual_guidance";
      connectivity_required: false;
    }[];
    package_repair: {
      mode: "guidance_only";
      actions: {
        action:
          | "use_downloaded_verified_installer"
          | "obtain_verified_installer";
        connectivity_required: boolean;
      }[];
    };
  };
}

interface PlatformPermissionRepair {
  schema_version: number;
  area: StorageArea;
  outcome: "repaired" | "manual_guidance_required" | "failed";
  connectivity_required: false;
}

const privacyActionLabels: Record<PrivacyAction, string> = {
  create_portable_backup: "Create an encrypted portable backup.",
  review_backup: "Review your encrypted backup recovery copy.",
  unlock_credential_vault: "Unlock saved details in Settings.",
  review_recovery: "Review encrypted backup recovery before maintenance.",
  review_external_ai: "Review Outside AI settings before sending.",
  refresh_browser_import_code: "Copy a fresh one-use Browser Import button.",
  review_source_safety: "Review restricted source safety settings.",
};

const areaLabels: Record<StorageArea, string> = {
  application_data: "Application data",
  configuration: "Configuration",
  cache: "Cache",
};

const privacyStateLabels: Record<PrivacyState, string> = {
  looks_good: "Looks good",
  needs_attention: "Needs attention",
  paused_for_safety: "Paused for safety",
  optional_improvement: "Optional improvement",
};

export function LocalRecoveryPanel() {
  const [report, setReport] = useState<LocalRecoveryReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [cleaning, setCleaning] = useState(false);
  const [repairing, setRepairing] = useState<StorageArea | null>(null);
  const [operationStatus, setOperationStatus] = useState("");

  const loadReport = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    try {
      const nextReport = await invoke<LocalRecoveryReport>(
        "get_local_recovery_report",
      );
      if (!nextReport || nextReport.schema_version !== 2) {
        setReport(null);
        setLoadFailed(true);
        return;
      }
      setReport(nextReport);
    } catch {
      setReport(null);
      setLoadFailed(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadReport();
  }, [loadReport]);

  const runCleanup = async () => {
    setCleaning(true);
    setOperationStatus("");
    try {
      const storage = await invoke<LocalStorageRecoveryReport>(
        "run_local_storage_cleanup",
      );
      setReport((current) =>
        current ? { ...current, storage } : current,
      );
      setOperationStatus(
        "Cleanup finished locally. Saved records were preserved; only already-free space was reclaimed.",
      );
    } catch {
      setOperationStatus(
        "Local cleanup could not finish. Saved records were not removed.",
      );
    } finally {
      setCleaning(false);
    }
  };

  const repairPermissions = async (area: StorageArea) => {
    setRepairing(area);
    setOperationStatus("");
    let status =
      `${areaLabels[area]} permissions could not be changed automatically. ` +
      "Use your system's file-permission settings or help.";
    try {
      const result = await invoke<PlatformPermissionRepair>(
        "repair_local_permissions",
        { area },
      );
      if (result.outcome === "repaired") {
        status = `${areaLabels[area]} permissions were repaired locally.`;
      } else if (result.outcome === "manual_guidance_required") {
        status =
          `Automatic repair is not safe for ${areaLabels[area].toLowerCase()}. ` +
          "Use your system's file-permission settings or help.";
      }
    } catch {
      // The fixed status text above deliberately excludes raw platform errors.
    }
    await loadReport();
    setOperationStatus(status);
    setRepairing(null);
  };

  return (
    <section
      className="mb-6 rounded-lg border border-surface-200 bg-surface-50 p-4 dark:border-surface-700 dark:bg-surface-800"
      aria-labelledby="local-recovery-heading"
    >
      <h3
        id="local-recovery-heading"
        className="font-medium text-surface-800 dark:text-surface-200"
      >
        Local Recovery
      </h3>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
        <span className="font-medium text-surface-700 dark:text-surface-200">
          Status check:
        </span>
        <span className="rounded-full bg-green-100 px-2 py-1 text-green-800 dark:bg-green-950 dark:text-green-200">
          Local only. No internet required.
        </span>
      </div>
      <p className="mt-2 text-xs text-surface-500 dark:text-surface-400">
        Local repair buttons also work offline. Any recovery step that needs
        internet is marked before you take it.
      </p>

      {loading && !report && (
        <p
          className="mt-4 text-sm text-surface-600 dark:text-surface-300"
          role="status"
          aria-label="Local recovery status"
          aria-live="polite"
        >
          Checking local recovery...
        </p>
      )}

      {loadFailed && !report && (
        <div className="mt-4">
          <p
            className="text-sm text-surface-700 dark:text-surface-200"
            role="status"
            aria-label="Local recovery status"
            aria-live="polite"
          >
            Local recovery status is unavailable. Other settings are still
            available.
          </p>
          <Button
            className="mt-3"
            variant="secondary"
            size="sm"
            onClick={() => void loadReport()}
          >
            Retry local recovery check
          </Button>
        </div>
      )}

      {report && (
        <div className="mt-4 space-y-5">
          <section aria-labelledby="queued-local-work-heading">
            <h4
              id="queued-local-work-heading"
              className="text-sm font-medium text-surface-800 dark:text-surface-100"
            >
              Queued local work
            </h4>
            <p className="mt-1 text-sm text-surface-700 dark:text-surface-200">
              {report.queued_local_work.pending_url_imports === 0
                ? "No local URL imports are waiting in this app session."
                : `${report.queued_local_work.pending_url_imports} local URL ${
                    report.queued_local_work.pending_url_imports === 1
                      ? "import is"
                      : "imports are"
                  } ready in this app session.`}
            </p>
            <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
              Up to {report.queued_local_work.capacity} imports are available
              offline and expire locally after 30 minutes. This status never
              includes job contents.
            </p>
          </section>

          <StorageSection
            cleaning={cleaning}
            loading={loading}
            onCleanup={() => void runCleanup()}
            storage={report.storage}
          />

          <section aria-labelledby="privacy-doctor-heading">
            <h4
              id="privacy-doctor-heading"
              className="text-sm font-medium text-surface-800 dark:text-surface-100"
            >
              Privacy Doctor
            </h4>
            <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
              {privacyStateLabels[report.privacy_doctor.overall]}. These are
              fixed local checks and do not read saved secrets.
            </p>
            <ul className="mt-2 space-y-2">
              {report.privacy_doctor.checks.map((check) => (
                <li
                  key={check.id}
                  className="rounded-md border border-surface-200 p-2 text-sm dark:border-surface-700"
                >
                  <p className="text-surface-700 dark:text-surface-200">
                    {check.message}
                  </p>
                  {check.action && (
                    <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
                      {privacyActionLabels[check.action]}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          </section>

          <section aria-labelledby="local-permissions-heading">
            <h4
              id="local-permissions-heading"
              className="text-sm font-medium text-surface-800 dark:text-surface-100"
            >
              Local permissions
            </h4>
            <ul className="mt-2 space-y-2">
              {report.platform_health.permissions.map((permission) => (
                <li
                  key={permission.area}
                  className="rounded-md border border-surface-200 p-2 text-sm dark:border-surface-700"
                >
                  <p className="font-medium text-surface-700 dark:text-surface-200">
                    {areaLabels[permission.area]}:{" "}
                    {permission.state === "private"
                      ? "Private"
                      : permission.state === "missing"
                        ? "Folder not created yet"
                        : "Needs review"}
                  </p>
                  {permission.action === "repair_locally" && (
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      <Button
                        variant="secondary"
                        size="sm"
                        loading={repairing === permission.area}
                        loadingText="Repairing locally..."
                        disabled={loading || repairing !== null}
                        onClick={() =>
                          void repairPermissions(permission.area)
                        }
                      >
                        Repair {areaLabels[permission.area].toLowerCase()}{" "}
                        permissions
                      </Button>
                      <span className="text-xs text-surface-500 dark:text-surface-400">
                        No internet required.
                      </span>
                    </div>
                  )}
                  {permission.action === "follow_manual_guidance" && (
                    <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
                      Automatic repair is not available for{" "}
                      {areaLabels[permission.area].toLowerCase()}. Use your
                      system&apos;s file-permission settings or help to keep
                      this area private.
                    </p>
                  )}
                </li>
              ))}
            </ul>
          </section>

          <section aria-labelledby="package-recovery-heading">
            <h4
              id="package-recovery-heading"
              className="text-sm font-medium text-surface-800 dark:text-surface-100"
            >
              App package recovery
            </h4>
            <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
              Guidance only. JobSentinel does not download or run an installer
              for you.
            </p>
            <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-surface-700 dark:text-surface-200">
              {report.platform_health.package_repair.actions.map((action) => (
                <li key={action.action}>
                  {action.action === "use_downloaded_verified_installer"
                    ? "Use an already-downloaded, verified JobSentinel installer"
                    : "Obtain a verified JobSentinel installer"}
                  :{" "}
                  {action.connectivity_required
                    ? "Internet required."
                    : "Works offline."}
                </li>
              ))}
            </ul>
          </section>
        </div>
      )}

      {operationStatus && (
        <p
          className="mt-4 text-sm text-surface-700 dark:text-surface-200"
          role="status"
          aria-label="Local recovery status"
          aria-live="polite"
        >
          {operationStatus}
        </p>
      )}
    </section>
  );
}

function StorageSection({
  cleaning,
  loading,
  onCleanup,
  storage,
}: {
  cleaning: boolean;
  loading: boolean;
  onCleanup: () => void;
  storage: LocalStorageRecoveryReport;
}) {
  const ready = storage.state === "ready";
  return (
    <section aria-labelledby="local-storage-heading">
      <h4
        id="local-storage-heading"
        className="text-sm font-medium text-surface-800 dark:text-surface-100"
      >
        Local storage
      </h4>
      <p className="mt-1 text-sm text-surface-700 dark:text-surface-200">
        {ready
          ? "Local storage is ready."
          : storage.state === "restore_from_backup_required"
            ? "Local storage needs recovery."
            : "Local storage could not be checked."}
      </p>
      {ready ? (
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          {storage.reclaimable_bytes > 0
            ? `${storage.reclaimable_bytes.toLocaleString()} bytes of already-free local storage can be reclaimed.`
            : "No already-free storage is ready to reclaim."}
        </p>
      ) : (
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          Cleanup is paused for safety. Use encrypted backup recovery to restore
          from an encrypted backup before cleanup.
        </p>
      )}
      <p className="mt-2 text-xs text-surface-500 dark:text-surface-400">
        Cleanup works offline, preserves every saved record, and only reclaims
        space already marked free.
      </p>
      <Button
        className="mt-2"
        variant="secondary"
        size="sm"
        loading={cleaning}
        loadingText="Cleaning up locally..."
        disabled={
          loading ||
          cleaning ||
          !ready ||
          !storage.cleanup_available
        }
        onClick={onCleanup}
      >
        Clean up local storage
      </Button>
    </section>
  );
}
