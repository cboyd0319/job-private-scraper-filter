/** Renders review and local history for the disabled JobsWithGPT source. */

import type { Dispatch, SetStateAction } from "react";
import { Badge } from "../../../ui/Badge";
import { Button } from "../../../ui/Button";
import { Input } from "../../../ui/Input";
import {
  formatJobSourceSite,
  type Config,
  type JobsWithGptPayload,
  type SourceRequestOutcome,
  type SourceRequestSummary,
} from "../config/SettingsConfig";
import { SettingsSymbol } from "../shared/SettingsIcons";

interface SettingsConnectedJobSourceProps {
  config: Config;
  jobsWithGptLastRequest: SourceRequestSummary | null;
  jobsWithGptPayload: JobsWithGptPayload | null;
  jobsWithGptPayloadApproved: boolean;
  onApprove: () => void;
  onClearApproval: () => void;
  onConfigChange: (config: Config) => void;
  setShowJobsWithGptEndpoint: Dispatch<SetStateAction<boolean>>;
  showJobsWithGptEndpoint: boolean;
}

function formatSourceRequestTime(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return "Recorded locally";
  }

  return date.toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function formatSourceRequestOutcome(outcome: SourceRequestOutcome): string {
  switch (outcome) {
    case "success":
      return "Completed";
    case "failure":
      return "Needs attention";
    case "timeout":
      return "Took too long";
    case "started":
    default:
      return "Outcome unknown; request may not have been sent.";
  }
}

export function SettingsConnectedJobSource({
  config,
  jobsWithGptLastRequest,
  jobsWithGptPayload,
  jobsWithGptPayloadApproved,
  onApprove,
  onClearApproval,
  onConfigChange,
  setShowJobsWithGptEndpoint,
  showJobsWithGptEndpoint,
}: SettingsConnectedJobSourceProps) {
  const hasJobsWithGptEndpoint = Boolean(config.jobswithgpt_endpoint?.trim());
  const hasJobsWithGptTitles = Boolean(
    config.title_allowlist.some((title) => title.trim().length > 0),
  );

  return (
    <div className="border border-surface-200 dark:border-surface-700 rounded-lg p-3">
      <div className="flex items-center justify-between mb-3 gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <SettingsSymbol
            icon="settings"
            className="h-5 w-5 text-surface-500 dark:text-surface-400"
          />
          <span className="font-medium text-surface-800 dark:text-surface-200">
            Connected job source
          </span>
          <Badge variant="surface" size="sm">
            Provider review pending
          </Badge>
        </div>
      </div>

      <Input
        label="Optional job-source link"
        value={config.jobswithgpt_endpoint ?? ""}
        onChange={(e) => {
          setShowJobsWithGptEndpoint(false);
          onConfigChange({
            ...config,
            jobswithgpt_endpoint: e.target.value,
          });
        }}
        placeholder="Leave blank unless you intentionally use an outside job feed"
        hint="Saved locally; scheduled contact is currently disabled"
      />

      <div className="mt-3 flex items-start gap-1.5 rounded-lg border border-amber-200 bg-amber-50 p-2 text-xs text-amber-700 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-300">
        <SettingsSymbol
          icon="warning"
          className="mt-0.5 h-3.5 w-3.5 flex-shrink-0"
        />
        <span>
          Scheduled contact is disabled while JobSentinel verifies the provider
          endpoint and usage policy.
        </span>
      </div>

      {hasJobsWithGptEndpoint && !hasJobsWithGptTitles && (
        <div className="mt-3 flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-2">
          <SettingsSymbol
            icon="warning"
            className="mt-0.5 h-3.5 w-3.5 flex-shrink-0"
          />
          <span>
            Add at least one job title in Search Preferences before this source
            can be approved.
          </span>
        </div>
      )}

      {jobsWithGptPayload && (
        <div className="mt-3 rounded-lg border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800/50 p-3">
          <p className="text-xs font-semibold text-surface-700 dark:text-surface-200 mb-2">
            Approve these exact details before JobSentinel contacts this source
          </p>
          <dl className="grid grid-cols-1 gap-2 text-xs text-surface-600 dark:text-surface-300 sm:grid-cols-[8rem_1fr]">
            <dt className="font-medium">Job-source site</dt>
            <dd className="space-y-1">
              <div className={showJobsWithGptEndpoint ? "break-all" : ""}>
                {showJobsWithGptEndpoint
                  ? jobsWithGptPayload.endpoint
                  : formatJobSourceSite(jobsWithGptPayload.endpoint)}
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  setShowJobsWithGptEndpoint((visible) => !visible)
                }
              >
                {showJobsWithGptEndpoint ? "Hide full link" : "Show full link"}
              </Button>
            </dd>
            <dt className="font-medium">Job titles</dt>
            <dd>{jobsWithGptPayload.titles.join(", ")}</dd>
            <dt className="font-medium">Location</dt>
            <dd>Not sent</dd>
            <dt className="font-medium">Remote-only filter</dt>
            <dd>{jobsWithGptPayload.remote_only ? "Yes" : "No"}</dd>
            <dt className="font-medium">Jobs to check</dt>
            <dd>{jobsWithGptPayload.limit}</dd>
          </dl>
          {jobsWithGptPayloadApproved && (
            <p className="mt-3 text-xs text-amber-700 dark:text-amber-300">
              These exact details were approved previously. Scheduled contact
              remains disabled until provider review completes.
            </p>
          )}
          {(jobsWithGptLastRequest || jobsWithGptPayloadApproved) && (
            <div className="mt-3 rounded-md border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-900 p-2 text-xs text-surface-600 dark:text-surface-300">
              <p className="font-semibold text-surface-700 dark:text-surface-200">
                Last contacted or attempted:{" "}
                {jobsWithGptLastRequest
                  ? formatSourceRequestTime(jobsWithGptLastRequest.sentAt)
                  : "Not yet"}
              </p>
              {jobsWithGptLastRequest && (
                <dl className="mt-2 grid grid-cols-1 gap-1 sm:grid-cols-[8rem_1fr]">
                  <dt className="font-medium">Website contacted</dt>
                  <dd className="break-all">
                    {jobsWithGptLastRequest.endpointHost ?? "Not recorded"}
                  </dd>
                  <dt className="font-medium">Saved titles</dt>
                  <dd>{jobsWithGptLastRequest.titleCount}</dd>
                  <dt className="font-medium">Location sent</dt>
                  <dd>{jobsWithGptLastRequest.hasLocation ? "Yes" : "No"}</dd>
                  <dt className="font-medium">Remote-only filter</dt>
                  <dd>{jobsWithGptLastRequest.remoteOnly ? "Yes" : "No"}</dd>
                  <dt className="font-medium">Jobs checked</dt>
                  <dd>{jobsWithGptLastRequest.resultLimit}</dd>
                  <dt className="font-medium">Last result</dt>
                  <dd>
                    {formatSourceRequestOutcome(jobsWithGptLastRequest.outcome)}
                  </dd>
                  <dt className="font-medium">Data not sent</dt>
                  <dd>
                    Resume text, salary floor, private notes, application
                    history, full source link
                  </dd>
                </dl>
              )}
            </div>
          )}
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              onClick={onApprove}
              disabled
            >
              Provider review pending
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={onClearApproval}
              disabled={!config.jobswithgpt_approval.enabled}
            >
              Remove approval
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
