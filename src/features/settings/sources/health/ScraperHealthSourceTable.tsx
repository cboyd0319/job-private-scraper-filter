import { Badge } from "../../../../ui/Badge";
import { LoadingSpinner } from "../../../../ui/LoadingSpinner";
import { Tooltip } from "../../../../ui/Tooltip";
import {
  formatDuration,
  formatRelativeTime,
  formatRunStatus,
  formatSafeIssue,
  formatSourceNextStep,
  formatSourceType,
  getRecentStatus,
  healthStatusConfig,
  selectorHealthConfig,
  type ScraperHealthMetrics,
  type ScraperRun,
} from "./scraperHealthDashboardModel";

export function StatusIcon({ status }: { status: string }) {
  const icons = {
    check: (
      <svg
        className="w-4 h-4"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M5 13l4 4L19 7"
        />
      </svg>
    ),
    warning: (
      <svg
        className="w-4 h-4"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>
    ),
    x: (
      <svg
        className="w-4 h-4"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M6 18L18 6M6 6l12 12"
        />
      </svg>
    ),
    minus: (
      <svg
        className="w-4 h-4"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M20 12H4"
        />
      </svg>
    ),
    question: (
      <svg
        className="w-4 h-4"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
    ),
  };
  return icons[status as keyof typeof icons] || icons.question;
}

interface ScraperHealthSourceTableProps {
  scrapers: ScraperHealthMetrics[];
  selectedScraper: string | null;
  runs: ScraperRun[];
  runsLoading: boolean;
  testingSingle: string | null;
  onSelectedScraperChange: (scraperName: string | null) => void;
  onRequestSmokeTest: (scraper: ScraperHealthMetrics) => void;
  onToggleScraper: (scraperName: string, enabled: boolean) => void;
}

export function ScraperHealthSourceTable({
  scrapers,
  selectedScraper,
  runs,
  runsLoading,
  testingSingle,
  onSelectedScraperChange,
  onRequestSmokeTest,
  onToggleScraper,
}: ScraperHealthSourceTableProps) {
  return (
    <>
      <div className="overflow-visible">
        <table
          className="app-responsive-table w-full text-sm"
          role="table"
          aria-label="Job source status"
        >
          <thead>
            <tr
              className="border-b border-surface-200 dark:border-surface-700"
              role="row"
            >
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Source
              </th>
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Status
              </th>
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Kind
              </th>
              <th className="text-right py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Recent Status
              </th>
              <th className="text-right py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Time Needed
              </th>
              <th className="text-right py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Jobs Found
              </th>
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Last Checked
              </th>
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Reads Job Details
              </th>
              <th className="text-left py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                What To Do
              </th>
              <th className="text-right py-3 px-4 font-medium text-surface-600 dark:text-surface-400">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            {scrapers.map((scraper) => {
              const statusConfig = healthStatusConfig[scraper.health_status];
              const selectorConfig = selectorHealthConfig[scraper.selector_health];
              const recentStatus = getRecentStatus(scraper);
              const providerReviewPending = scraper.scraper_name === "jobswithgpt";

              return (
                <tr
                  key={scraper.scraper_name}
                  className={`border-b border-surface-100 dark:border-surface-700/50 hover:bg-surface-50 dark:hover:bg-surface-700/30 transition-colors ${
                    selectedScraper === scraper.scraper_name
                      ? "bg-sentinel-50 dark:bg-sentinel-900/20"
                      : ""
                  }`}
                >
                  <td className="py-3 px-4" data-label="Source">
                    <button
                      onClick={() =>
                        onSelectedScraperChange(
                          selectedScraper === scraper.scraper_name
                            ? null
                            : scraper.scraper_name,
                        )
                      }
                      className="text-left hover:text-sentinel-600 dark:hover:text-sentinel-400"
                    >
                      <div className="font-medium text-surface-900 dark:text-white">
                        {scraper.display_name}
                      </div>
                      <div className="text-xs text-surface-500">
                        {scraper.requires_auth && (
                          <span className="ml-1 text-alert-500">
                            Needs setup
                          </span>
                        )}
                      </div>
                    </button>
                  </td>
                  <td className="py-3 px-4" data-label="Status">
                    <Badge variant={statusConfig.variant} size="sm">
                      <StatusIcon status={statusConfig.icon} />
                      <span className="ml-1">{statusConfig.label}</span>
                    </Badge>
                  </td>
                  <td className="py-3 px-4" data-label="Kind">
                    <Badge variant="surface" size="sm">
                      {formatSourceType(scraper.scraper_type)}
                    </Badge>
                  </td>
                  <td className="py-3 px-4 text-right" data-label="Recent Status">
                    <span className={recentStatus.className}>
                      {recentStatus.label}
                    </span>
                  </td>
                  <td className="py-3 px-4 text-right text-surface-600 dark:text-surface-400" data-label="Time Needed">
                    {formatDuration(scraper.avg_duration_ms)}
                  </td>
                  <td className="py-3 px-4 text-right" data-label="Jobs Found">
                    {scraper.jobs_found_24h === 0 &&
                    scraper.total_runs_24h > 0 &&
                    scraper.health_status === "healthy" ? (
                      <Tooltip content="This job site checked successfully but found 0 jobs. Check your search terms or this source may be empty.">
                        <span className="font-medium text-amber-600 dark:text-amber-400 cursor-help">
                          <span className="inline-flex items-center justify-end gap-1">
                            0
                            <StatusIcon status="warning" />
                          </span>
                        </span>
                      </Tooltip>
                    ) : (
                      <span className="font-medium text-surface-900 dark:text-white">
                        {scraper.jobs_found_24h}
                      </span>
                    )}
                    <span className="text-surface-500 ml-1">
                      from {scraper.total_runs_24h} checks
                    </span>
                  </td>
                  <td className="py-3 px-4 text-surface-600 dark:text-surface-400" data-label="Last Checked">
                    {formatRelativeTime(scraper.last_success)}
                  </td>
                  <td className="py-3 px-4" data-label="Reads Job Details">
                    {scraper.scraper_type === "html" ||
                    scraper.scraper_type === "hybrid" ? (
                      <Badge variant={selectorConfig.variant} size="sm">
                        {selectorConfig.label}
                      </Badge>
                    ) : (
                      <span className="text-surface-400">
                        Uses official source
                      </span>
                    )}
                  </td>
                  <td className="max-w-xs px-4 py-3 text-surface-700 dark:text-surface-300" data-label="What To Do">
                    {formatSourceNextStep(scraper)}
                  </td>
                  <td className="py-3 px-4 text-right" data-label="Actions">
                    <div className="flex items-center justify-end gap-2">
                      <button
                        aria-label={
                          providerReviewPending
                            ? "JobsWithGPT provider review pending"
                            : scraper.is_enabled
                            ? `Turn ${scraper.display_name} off`
                            : `Turn ${scraper.display_name} on`
                        }
                        onClick={() =>
                          onToggleScraper(
                            scraper.scraper_name,
                            !scraper.is_enabled,
                          )
                        }
                        disabled={providerReviewPending}
                        className={`min-w-20 rounded px-2.5 py-1.5 text-xs font-medium transition-colors ${
                          scraper.is_enabled
                            ? "bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-900/30 dark:text-green-300"
                            : "bg-surface-100 text-surface-600 hover:bg-surface-200 dark:bg-surface-700 dark:text-surface-300"
                        }`}
                      >
                        {providerReviewPending
                          ? "Review Pending"
                          : scraper.is_enabled
                            ? "Turn Off"
                            : "Turn On"}
                      </button>
                      <button
                        aria-label={`Check ${scraper.display_name} now`}
                        onClick={() => onRequestSmokeTest(scraper)}
                        disabled={
                          testingSingle === scraper.scraper_name ||
                          !scraper.is_enabled
                        }
                        className="min-w-20 rounded bg-surface-100 px-2.5 py-1.5 text-xs font-medium text-surface-600 transition-colors hover:bg-surface-200 disabled:opacity-50 dark:bg-surface-700 dark:text-surface-300"
                      >
                        {testingSingle === scraper.scraper_name
                          ? "Checking..."
                          : "Check Now"}
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {selectedScraper && (
        <ScraperRunHistory
          runs={runs}
          runsLoading={runsLoading}
          selectedScraper={selectedScraper}
          scrapers={scrapers}
        />
      )}
    </>
  );
}

interface ScraperRunHistoryProps {
  runs: ScraperRun[];
  runsLoading: boolean;
  selectedScraper: string;
  scrapers: ScraperHealthMetrics[];
}

function ScraperRunHistory({
  runs,
  runsLoading,
  selectedScraper,
  scrapers,
}: ScraperRunHistoryProps) {
  return (
    <div
      className="mt-6 p-4 bg-surface-50 dark:bg-surface-700/30 rounded-lg"
      role="region"
      aria-labelledby="run-history-title"
      aria-live="polite"
    >
      <h3
        id="run-history-title"
        className="font-medium text-surface-900 dark:text-white mb-4"
      >
        Recent Checks:{" "}
        {scrapers.find((scraper) => scraper.scraper_name === selectedScraper)
          ?.display_name}
      </h3>
      {runsLoading ? (
        <LoadingSpinner message="Loading check history..." />
      ) : runs.length === 0 ? (
        <p
          className="text-surface-500 dark:text-surface-400"
          role="status"
        >
          No recent checks found.
        </p>
      ) : (
        <div className="overflow-visible">
          <table
            className="app-responsive-table w-full text-sm"
            role="table"
            aria-label="Source check history"
          >
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-600">
                <th className="text-left py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  Checked
                </th>
                <th className="text-left py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  Status
                </th>
                <th className="text-right py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  Time
                </th>
                <th className="text-right py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  Jobs found
                </th>
                <th className="text-right py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  New jobs
                </th>
                <th className="text-left py-2 px-3 font-medium text-surface-600 dark:text-surface-400">
                  Problem
                </th>
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => (
                <tr
                  key={run.id}
                  className="border-b border-surface-100 dark:border-surface-600/50"
                >
                  <td className="px-3 py-2 text-surface-600 dark:text-surface-400" data-label="Checked">
                    {formatRelativeTime(run.started_at)}
                  </td>
                  <td className="px-3 py-2" data-label="Status">
                    <Badge
                      variant={
                        run.status === "success"
                          ? "success"
                          : run.status === "running"
                            ? "sentinel"
                            : run.status === "rate_limited"
                              ? "alert"
                              : "danger"
                      }
                      size="sm"
                    >
                      {formatRunStatus(run.status, run.retry_attempt)}
                    </Badge>
                  </td>
                  <td className="px-3 py-2 text-right text-surface-600 dark:text-surface-400" data-label="Time">
                    {formatDuration(run.duration_ms)}
                  </td>
                  <td className="px-3 py-2 text-right text-surface-900 dark:text-white" data-label="Jobs found">
                    {run.jobs_found}
                  </td>
                  <td className="px-3 py-2 text-right text-green-600 dark:text-green-400" data-label="New jobs">
                    +{run.jobs_new}
                  </td>
                  <td className="max-w-xs break-words px-3 py-2 text-surface-500 [overflow-wrap:anywhere] dark:text-surface-400" data-label="Problem">
                    {formatSafeIssue(run.error_message)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
