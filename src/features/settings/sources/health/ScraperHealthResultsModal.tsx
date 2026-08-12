import { Badge } from "../../../../ui/Badge";
import { Button } from "../../../../ui/Button";
import { Modal } from "../../../../ui/Modal";
import {
  formatDuration,
  formatSafeIssue,
  type SmokeTestResult,
} from "./scraperHealthDashboardModel";

interface ScraperHealthResultsModalProps {
  isOpen: boolean;
  onClose: () => void;
  results: SmokeTestResult[];
  sourceNameById: ReadonlyMap<string, string>;
}

function getSmokeResultStatus(result: SmokeTestResult) {
  if (result.details?.status === "skipped") {
    return {
      badge: "Skipped",
      variant: "surface" as const,
      className:
        "bg-surface-50 border-surface-200 dark:bg-surface-800/60 dark:border-surface-700",
    };
  }

  return result.passed
    ? {
        badge: "Worked",
        variant: "success" as const,
        className:
          "bg-green-50 border-green-200 dark:bg-green-900/20 dark:border-green-800",
      }
    : {
        badge: "Problem found",
        variant: "danger" as const,
        className:
          "bg-red-50 border-red-200 dark:bg-red-900/20 dark:border-red-800",
      };
}

export function ScraperHealthResultsModal({
  isOpen,
  onClose,
  results,
  sourceNameById,
}: ScraperHealthResultsModalProps) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Check Results"
      size="lg"
    >
      <div className="space-y-3">
        {results.map((result) => {
          const status = getSmokeResultStatus(result);
          return (
            <div
              key={result.scraper_name}
              className={`p-3 rounded-lg border ${status.className}`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium text-surface-900 dark:text-white">
                  {sourceNameById.get(result.scraper_name) ??
                    result.scraper_name}
                </span>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-surface-500 dark:text-surface-400">
                    {formatDuration(result.duration_ms)}
                  </span>
                  <Badge variant={status.variant} size="sm">
                    {status.badge}
                  </Badge>
                </div>
              </div>
              {result.error && (
                <p className="text-sm text-red-600 dark:text-red-400 mt-1">
                  {formatSafeIssue(result.error)}
                </p>
              )}
              {result.details?.status === "skipped" &&
                typeof result.details.reason === "string" && (
                  <p className="mt-1 text-sm text-surface-600 dark:text-surface-300">
                    {result.details.reason}
                  </p>
                )}
            </div>
          );
        })}
      </div>
      <div className="mt-4 flex justify-end">
        <Button variant="secondary" onClick={onClose}>
          Close
        </Button>
      </div>
    </Modal>
  );
}
