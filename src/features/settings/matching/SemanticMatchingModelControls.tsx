import {
  formatSemanticMatchingBytes,
  type SemanticMatchingDiagnostics,
} from "./semanticMatchingDiagnostics";

export function SemanticMatchingModelControls({
  diagnostics,
  modelAction,
  downloadProgress,
  onDownload,
  onCancel,
  onRemove,
}: {
  diagnostics: SemanticMatchingDiagnostics;
  modelAction: "downloading" | "removing" | null;
  downloadProgress: {
    completed_bytes: number;
    total_bytes: number;
  } | null;
  onDownload: () => Promise<void>;
  onCancel: () => Promise<void>;
  onRemove: () => Promise<void>;
}) {
  if (!diagnostics.build_enabled) return null;

  const requiredModels = diagnostics.models.filter(
    (model) => model.required_for_qwen3_runtime,
  );
  const canDownload = diagnostics.runtime_status !== "ready";
  const canRemove = requiredModels.some((model) => model.cache_present);
  const totalBytes = requiredModels.reduce(
    (total, model) => total + (model.locked_size_bytes ?? 0),
    0,
  );
  const licenses = [...new Set(requiredModels.map((model) => model.license))].join(
    ", ",
  );
  return (
    <div className="mt-4 rounded-md border border-surface-200 p-3 dark:border-surface-700">
      <p className="text-sm text-surface-700 dark:text-surface-200">
        Stronger local matching downloads about{" "}
        {formatSemanticMatchingBytes(totalBytes)} of pinned {licenses} model
        files from Hugging Face and its file delivery network. Setup can need
        additional temporary disk space. Resume and job data are never sent.
      </p>
      <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
        Built-in matching remains available during setup, after cancellation,
        and if the download fails. Leaving this page cancels the download.
      </p>
      {modelAction === "downloading" ? (
        <div className="mt-3">
          <progress
            aria-label="Local model download progress"
            className="h-2 w-full"
            max={downloadProgress?.total_bytes || undefined}
            value={downloadProgress?.completed_bytes}
          />
          <div className="mt-2 flex flex-wrap items-center justify-between gap-2">
            <span className="text-xs text-surface-600 dark:text-surface-300">
              {downloadProgress?.total_bytes
                ? `${formatSemanticMatchingBytes(downloadProgress.completed_bytes)} of ${formatSemanticMatchingBytes(downloadProgress.total_bytes)} downloaded; files are verified before activation`
                : "Preparing the verified model download..."}
            </span>
            <button
              type="button"
              onClick={() => void onCancel()}
              className="rounded-md border border-surface-300 px-2 py-1 text-xs font-medium text-surface-700 dark:border-surface-600 dark:text-surface-200"
            >
              Cancel model download
            </button>
          </div>
        </div>
      ) : (
        <div className="mt-3 flex flex-wrap gap-2">
          {canDownload ? (
            <button
              type="button"
              onClick={() => void onDownload()}
              disabled={modelAction !== null}
              className="rounded-md bg-primary-600 px-3 py-1.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
            >
              Set up stronger local matching
            </button>
          ) : null}
          {canRemove ? (
            <button
              type="button"
              onClick={() => void onRemove()}
              disabled={modelAction !== null}
              className="rounded-md border border-danger/40 px-3 py-1.5 text-sm font-medium text-danger disabled:cursor-not-allowed disabled:opacity-60"
            >
              {modelAction === "removing"
                ? "Reviewing removal..."
                : "Remove local models"}
            </button>
          ) : null}
        </div>
      )}
    </div>
  );
}
