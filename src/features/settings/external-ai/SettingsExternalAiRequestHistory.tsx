import { useEffect, useState } from "react";
import { safeInvoke } from "../../../platform/tauri";

const ACTIVITY_STATUSES = [
  "pending",
  "started",
  "succeeded",
  "failed",
  "ambiguous",
  "cancelled",
] as const;

type ExternalAiActivityStatus = (typeof ACTIVITY_STATUSES)[number];

interface ExternalAiActivityEntry {
  providerId: string;
  destination: string;
  status: ExternalAiActivityStatus;
  createdAt: string;
  completedAt: string | null;
}

const STATUS_LABELS: Record<ExternalAiActivityStatus, string> = {
  pending: "Reviewed, not sent.",
  started: "Sending.",
  succeeded: "Sent.",
  failed: "Failed with a known outcome.",
  ambiguous: "Outcome unknown. Do not retry.",
  cancelled: "Cancelled before send.",
};

function isTimestamp(value: unknown): value is string {
  return typeof value === "string" && !Number.isNaN(Date.parse(value));
}

function readActivity(value: unknown): ExternalAiActivityEntry[] {
  if (!Array.isArray(value)) {
    throw new Error("Outside AI activity response is invalid.");
  }
  return value.flatMap((entry) => {
    if (!entry || typeof entry !== "object") return [];
    const record = entry as Record<string, unknown>;
    if (
      typeof record.providerId !== "string" ||
      !record.providerId ||
      typeof record.destination !== "string" ||
      !record.destination.startsWith("https://") ||
      !ACTIVITY_STATUSES.includes(record.status as ExternalAiActivityStatus) ||
      !isTimestamp(record.createdAt) ||
      (record.completedAt !== null && !isTimestamp(record.completedAt))
    ) {
      return [];
    }
    return [
      {
        providerId: record.providerId,
        destination: record.destination,
        status: record.status as ExternalAiActivityStatus,
        createdAt: record.createdAt,
        completedAt: record.completedAt,
      },
    ];
  });
}

function formatTimestamp(timestamp: string): string {
  return new Date(timestamp).toLocaleString();
}

export function SettingsExternalAiRequestHistory() {
  const [entries, setEntries] = useState<ExternalAiActivityEntry[]>([]);
  const [unavailable, setUnavailable] = useState(false);

  useEffect(() => {
    let active = true;
    safeInvoke<unknown>(
      "list_external_ai_activity",
      {},
      { logContext: "Load outside AI activity", silent: true },
    )
      .then(readActivity)
      .then((activity) => {
        if (active) setEntries(activity);
      })
      .catch(() => {
        if (active) setUnavailable(true);
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <section
      className="rounded-lg border border-surface-200 bg-white p-3 dark:border-surface-700 dark:bg-surface-900"
      aria-labelledby="external-ai-history-heading"
    >
      <h4
        id="external-ai-history-heading"
        className="text-sm font-medium text-surface-900 dark:text-surface-100"
      >
        Durable outside AI activity
      </h4>
      <p className="mt-1 text-xs text-surface-600 dark:text-surface-300">
        This local backend record stores only provider, destination, status, and
        time. It never stores the details sent, response, credential, or raw
        error.
      </p>

      {unavailable ? (
        <p className="mt-3 text-sm text-red-700 dark:text-red-300">
          Outside AI activity is unavailable. Do not retry an uncertain request.
        </p>
      ) : entries.length === 0 ? (
        <p className="mt-3 text-sm text-surface-500 dark:text-surface-400">
          No durable outside AI activity on this device.
        </p>
      ) : (
        <ul className="mt-3 divide-y divide-surface-100 dark:divide-surface-800">
          {entries.map((entry) => (
            <li
              key={`${entry.createdAt}-${entry.providerId}-${entry.destination}`}
              className="py-2 text-sm text-surface-700 dark:text-surface-200"
            >
              <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                <span className="font-medium text-surface-900 dark:text-surface-100">
                  {entry.providerId}
                </span>
                <span className="text-xs text-surface-500 dark:text-surface-400">
                  {formatTimestamp(entry.completedAt ?? entry.createdAt)}
                </span>
              </div>
              <p className="mt-1 break-all text-xs text-surface-600 dark:text-surface-300">
                {entry.destination}
              </p>
              <p className="mt-1 text-xs font-medium text-surface-700 dark:text-surface-200">
                {STATUS_LABELS[entry.status]}
              </p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
