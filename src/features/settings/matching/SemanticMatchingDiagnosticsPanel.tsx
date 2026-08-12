import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "../../../platform/tauri/events";
import { SettingsSymbol } from "../shared/SettingsIcons";
import { SemanticMatchingModelControls } from "./SemanticMatchingModelControls";
import {
  cancelSemanticMatchingModelDownload,
  downloadSemanticMatchingModels,
  formatSemanticMatchingBytes,
  getSemanticMatchingDiagnostics,
  removeSemanticMatchingModels,
  repairSemanticMatchingModelCache,
  type SemanticMatchingDiagnostics,
  type ModelCacheHealth,
  type SemanticMatchingModelDiagnostic,
  type SemanticMatchingRuntimeStatus,
} from "./semanticMatchingDiagnostics";

const STATUS_COPY: Record<
  SemanticMatchingRuntimeStatus,
  { label: string; tone: string }
> = {
  ready: {
    label: "Advanced local matching is ready",
    tone: "bg-success/10 text-success",
  },
  needs_model_download: {
    label: "Advanced local matching needs model files",
    tone: "bg-warning/15 text-surface-800 dark:text-warning",
  },
  disabled_in_this_build: {
    label: "Using built-in local matching rules",
    tone: "bg-surface-100 text-surface-700 dark:bg-surface-800 dark:text-surface-200",
  },
  misconfigured: {
    label: "Local matching needs attention",
    tone: "bg-danger/10 text-danger",
  },
};

const HEALTH_LABELS: Record<ModelCacheHealth, string> = {
  missing: "Not downloaded",
  incomplete: "Incomplete",
  integrity_mismatch: "Integrity check failed",
  ready: "Ready",
};

export function SemanticMatchingDiagnosticsPanel() {
  const [diagnostics, setDiagnostics] =
    useState<SemanticMatchingDiagnostics | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [repairingModelId, setRepairingModelId] = useState<string | null>(null);
  const [modelAction, setModelAction] = useState<
    "downloading" | "removing" | null
  >(null);
  const [downloadProgress, setDownloadProgress] = useState<{
    completed_bytes: number;
    total_bytes: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      setDiagnostics(await getSemanticMatchingDiagnostics());
    } catch (refreshError) {
      setError(
        refreshError instanceof Error
          ? refreshError.message
          : "Local matching diagnostics could not be loaded.",
      );
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    const unlisten = listen<{
      completed_bytes: number;
      total_bytes: number;
    }>("local-model-download-progress", ({ payload }) => {
      setDownloadProgress(payload);
    });
    return () => {
      void unlisten.then((stopListening) => stopListening());
    };
  }, []);
  const repair = useCallback(
    async (modelId: string) => {
      setRepairingModelId(modelId);
      setError(null);
      try {
        if (await repairSemanticMatchingModelCache(modelId)) {
          await refresh();
        }
      } catch (repairError) {
        setError(
          repairError instanceof Error
            ? repairError.message
            : "Local model repair could not be completed.",
        );
      } finally {
        setRepairingModelId(null);
      }
    },
    [refresh],
  );
  const downloadModels = useCallback(async () => {
    setModelAction("downloading");
    setDownloadProgress(null);
    setError(null);
    try {
      await downloadSemanticMatchingModels();
      await refresh();
    } catch (downloadError) {
      const canceledOrphan = await cancelSemanticMatchingModelDownload().catch(
        () => false,
      );
      setError(
        canceledOrphan
          ? "A previous model download was still running and has been canceled. Try again."
          : downloadError instanceof Error
            ? downloadError.message
            : "Local model download could not be completed.",
      );
    } finally {
      setModelAction(null);
    }
  }, [refresh]);
  const removeModels = useCallback(async () => {
    setModelAction("removing");
    setError(null);
    try {
      if (await removeSemanticMatchingModels()) {
        await refresh();
      }
    } catch (removeError) {
      setError(
        removeError instanceof Error
          ? removeError.message
          : "Local models could not be removed.",
      );
    } finally {
      setModelAction(null);
    }
  }, [refresh]);
  const cancelDownload = useCallback(async () => {
    try {
      await cancelSemanticMatchingModelDownload();
    } catch (cancelError) {
      setError(
        cancelError instanceof Error
          ? cancelError.message
          : "Local model download could not be canceled.",
      );
    }
  }, []);
  const modelActionRef = useRef(modelAction);
  useEffect(() => {
    modelActionRef.current = modelAction;
  }, [modelAction]);
  useEffect(
    () => () => {
      if (modelActionRef.current === "downloading") {
        void cancelSemanticMatchingModelDownload().catch(() => {});
      }
    },
    [],
  );
  return (
    <section className="mb-6">
      <div className="mb-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <h3 className="flex items-center gap-2 font-medium text-surface-800 dark:text-surface-200">
          Local Match Check
        </h3>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={isLoading}
          className="inline-flex items-center justify-center gap-2 rounded-md border border-surface-300 px-3 py-1.5 text-sm font-medium text-surface-700 transition-colors hover:bg-surface-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-surface-600 dark:text-surface-200 dark:hover:bg-surface-800"
        >
          <SettingsSymbol icon="search" className="h-4 w-4" />
          {isLoading ? "Checking..." : "Refresh"}
        </button>
      </div>

      <div className="border border-surface-200 dark:border-surface-700 rounded-lg p-4 bg-white dark:bg-surface-900/20">
        {error ? (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        ) : null}

        {!diagnostics && !error ? (
          <p className="text-sm text-surface-600 dark:text-surface-400">
            Checking local matching...
          </p>
        ) : null}

        {diagnostics ? (
          <DiagnosticsSummary
            diagnostics={diagnostics}
            repairingModelId={repairingModelId}
            modelLifecycleBusy={modelAction !== null}
            onRepair={repair}
          />
        ) : null}
        {diagnostics ? (
          <SemanticMatchingModelControls
            diagnostics={diagnostics}
            modelAction={modelAction}
            downloadProgress={downloadProgress}
            onDownload={downloadModels}
            onCancel={cancelDownload}
            onRemove={removeModels}
          />
        ) : null}
      </div>
    </section>
  );
}

function DiagnosticsSummary({
  diagnostics,
  repairingModelId,
  modelLifecycleBusy,
  onRepair,
}: {
  diagnostics: SemanticMatchingDiagnostics;
  repairingModelId: string | null;
  modelLifecycleBusy: boolean;
  onRepair: (modelId: string) => Promise<void>;
}) {
  const status = STATUS_COPY[diagnostics.runtime_status];
  const highlightedModels = diagnostics.models.filter(
    (model) =>
      model.required_for_qwen3_runtime || model.role.includes("Legacy"),
  );

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <span
              className={`inline-flex rounded-full px-2.5 py-1 text-xs font-semibold ${status.tone}`}
            >
              {status.label}
            </span>
            <span className="text-xs text-surface-500 dark:text-surface-400">
              {diagnostics.build_enabled
                ? "Model build enabled"
                : "Model build not enabled"}
            </span>
          </div>
          <p className="text-sm text-surface-700 dark:text-surface-300">
            {diagnostics.active_profile}
          </p>
          <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
            {diagnostics.privacy_mode}
          </p>
        </div>
        {diagnostics.manifest_hash ? (
          <div className="text-left text-xs text-surface-500 dark:text-surface-400 sm:text-right">
            <div className="font-medium text-surface-700 dark:text-surface-200">
              Model lock
            </div>
            <div>{diagnostics.manifest_hash.slice(0, 12)}</div>
          </div>
        ) : null}
      </div>

      {diagnostics.user_action ? (
        <p className="rounded-md bg-surface-50 px-3 py-2 text-sm text-surface-700 dark:bg-surface-800 dark:text-surface-200">
          {diagnostics.user_action}
        </p>
      ) : null}

      <ModelList
        models={highlightedModels}
        repairingModelId={repairingModelId}
        modelLifecycleBusy={modelLifecycleBusy}
        onRepair={onRepair}
      />
      <SignalList diagnostics={diagnostics} />
    </div>
  );
}

function ModelList({
  models,
  repairingModelId,
  modelLifecycleBusy,
  onRepair,
}: {
  models: SemanticMatchingModelDiagnostic[];
  repairingModelId: string | null;
  modelLifecycleBusy: boolean;
  onRepair: (modelId: string) => Promise<void>;
}) {
  if (models.length === 0) {
    return (
      <p className="text-sm text-surface-600 dark:text-surface-400">
        Advanced local model details appear when this app is built with local
        semantic matching.
      </p>
    );
  }

  return (
    <div>
      <h4 className="mb-2 text-sm font-semibold text-surface-900 dark:text-white">
        Local Models
      </h4>
      <div className="grid gap-2">
        {models.map((model) => (
          <div
            key={model.id}
            className="grid gap-2 rounded-md border border-surface-200 p-3 text-sm dark:border-surface-700 sm:grid-cols-[minmax(0,1fr)_auto]"
          >
            <div className="min-w-0">
              <div className="font-medium text-surface-900 dark:text-white">
                {model.role}
              </div>
              <div className="break-words text-surface-600 dark:text-surface-400">
                {model.repo}
              </div>
              <div className="mt-1 text-xs text-surface-500 dark:text-surface-400">
                {model.backend} - {formatRevision(model.revision)}
                {model.dimension ? ` - ${model.dimension} dims` : ""}
                {model.locked_size_bytes
                  ? ` - ${formatSemanticMatchingBytes(model.locked_size_bytes)}`
                  : ""}
              </div>
            </div>
            <div className="text-left sm:text-right">
              <div className="text-sm font-semibold text-surface-900 dark:text-white">
                {HEALTH_LABELS[model.health]}
              </div>
              <div className="text-xs text-surface-500 dark:text-surface-400">
                {model.required_files_present}/{model.required_files} files
                present
              </div>
              {model.health === "integrity_mismatch" &&
              model.required_for_qwen3_runtime ? (
                <button
                  type="button"
                  onClick={() => void onRepair(model.id)}
                  disabled={modelLifecycleBusy || repairingModelId !== null}
                  className="mt-2 rounded-md border border-danger/40 px-2 py-1 text-xs font-medium text-danger disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {repairingModelId === model.id
                    ? "Reviewing repair..."
                    : "Review local model repair"}
                </button>
              ) : null}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function SignalList({
  diagnostics,
}: {
  diagnostics: SemanticMatchingDiagnostics;
}) {
  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <div>
        <h4 className="mb-2 text-sm font-semibold text-surface-900 dark:text-white">
          What Matching Uses
        </h4>
        <div className="space-y-2">
          {diagnostics.scoring_signals.map((signal) => (
            <div key={signal.id} className="text-sm">
              <div className="flex items-center justify-between gap-3">
                <span className="font-medium text-surface-800 dark:text-surface-100">
                  {signal.label}
                </span>
                <span className="text-xs text-surface-500 dark:text-surface-400">
                  {signal.state}
                </span>
              </div>
              <p className="mt-0.5 text-xs text-surface-500 dark:text-surface-400">
                {signal.explanation}
              </p>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h4 className="mb-2 text-sm font-semibold text-surface-900 dark:text-white">
          Quality Checks
        </h4>
        <ul className="space-y-1.5 text-xs text-surface-600 dark:text-surface-400">
          {diagnostics.eval_contract.map((item) => (
            <li key={item} className="flex gap-2">
              <SettingsSymbol
                icon="check"
                className="mt-0.5 h-3.5 w-3.5 flex-shrink-0"
              />
              <span>{item}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function formatRevision(revision: string): string {
  return revision.length > 12 ? revision.slice(0, 12) : revision;
}
