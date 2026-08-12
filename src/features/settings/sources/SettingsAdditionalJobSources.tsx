import type { Dispatch, SetStateAction } from "react";
import { SettingsSymbol } from "../shared/SettingsIcons";
import { SettingsConnectedJobSource } from "./SettingsConnectedJobSource";
import type {
  Config,
  JobsWithGptPayload,
  SourceRequestSummary,
} from "../config/SettingsConfig";

interface SettingsAdditionalJobSourcesProps {
  config: Config;
  jobsWithGptLastRequest: SourceRequestSummary | null;
  jobsWithGptPayload: JobsWithGptPayload | null;
  jobsWithGptPayloadApproved: boolean;
  onApproveJobsWithGptPayload: () => void;
  onClearJobsWithGptApproval: () => void;
  setConfig: Dispatch<SetStateAction<Config | null>>;
  setShowJobsWithGptEndpoint: Dispatch<SetStateAction<boolean>>;
  showJobsWithGptEndpoint: boolean;
}

export function SettingsAdditionalJobSources({
  config,
  jobsWithGptLastRequest,
  jobsWithGptPayload,
  jobsWithGptPayloadApproved,
  onApproveJobsWithGptPayload,
  onClearJobsWithGptApproval,
  setConfig,
  setShowJobsWithGptEndpoint,
  showJobsWithGptEndpoint,
}: SettingsAdditionalJobSourcesProps) {
  return (
    <details className="border border-surface-200 dark:border-surface-700 rounded-lg">
      <summary className="p-4 cursor-pointer hover:bg-surface-50 dark:hover:bg-surface-800/50 font-medium text-surface-800 dark:text-surface-200 flex items-center gap-2">
        <span>More Job Boards</span>
        <span className="text-xs text-surface-500 dark:text-surface-400 font-normal">
          (optional sources)
        </span>
      </summary>
      <div className="p-4 pt-0 space-y-4">
        {/* Optional connected job source */}
        <SettingsConnectedJobSource
          config={config}
          jobsWithGptLastRequest={jobsWithGptLastRequest}
          jobsWithGptPayload={jobsWithGptPayload}
          jobsWithGptPayloadApproved={jobsWithGptPayloadApproved}
          onApprove={onApproveJobsWithGptPayload}
          onClearApproval={onClearJobsWithGptApproval}
          onConfigChange={setConfig}
          setShowJobsWithGptEndpoint={setShowJobsWithGptEndpoint}
          showJobsWithGptEndpoint={showJobsWithGptEndpoint}
        />

        {/* RemoteOK */}
        <div className="border border-surface-200 dark:border-surface-700 rounded-lg p-3">
          <div className="flex flex-col gap-3 mb-2 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <SettingsSymbol
                icon="globe"
                className="h-5 w-5 text-surface-500 dark:text-surface-400"
              />
              <span className="font-medium text-surface-800 dark:text-surface-200">
                RemoteOK
              </span>
              <span className="text-xs text-surface-500">
                (Remote-only jobs)
              </span>
            </div>
            <label className="relative inline-flex flex-shrink-0 items-center cursor-pointer">
              <input
                type="checkbox"
                aria-label="Turn Remote OK scheduled job checks on or off"
                checked={config.remoteok?.enabled ?? false}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    remoteok: {
                      ...config.remoteok,
                      enabled: e.target.checked,
                      tags: config.remoteok?.tags ?? [],
                      limit: config.remoteok?.limit ?? 50,
                    },
                  })
                }
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-surface-200 peer-focus-visible:ring-2 peer-focus-visible:ring-sentinel-300 rounded-full peer dark:bg-surface-700 peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-sentinel-500"></div>
            </label>
          </div>
        </div>

        {/* WeWorkRemotely */}
        <div className="border border-surface-200 dark:border-surface-700 rounded-lg p-3">
          <div className="flex flex-col gap-3 mb-2 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <SettingsSymbol
                icon="home"
                className="h-5 w-5 text-surface-500 dark:text-surface-400"
              />
              <span className="font-medium text-surface-800 dark:text-surface-200">
                WeWorkRemotely
              </span>
              <span className="text-xs text-surface-500">(Remote jobs)</span>
            </div>
            <label className="relative inline-flex flex-shrink-0 items-center cursor-pointer">
              <input
                type="checkbox"
                aria-label="Turn We Work Remotely scheduled job checks on or off"
                checked={config.weworkremotely?.enabled ?? false}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    weworkremotely: {
                      ...config.weworkremotely,
                      enabled: e.target.checked,
                      limit: config.weworkremotely?.limit ?? 50,
                    },
                  })
                }
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-surface-200 peer-focus-visible:ring-2 peer-focus-visible:ring-sentinel-300 rounded-full peer dark:bg-surface-700 peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-sentinel-500"></div>
            </label>
          </div>
        </div>

        {/* Startup and tech hiring posts */}
        <div className="border border-surface-200 dark:border-surface-700 rounded-lg p-3">
          <div className="flex flex-col gap-3 mb-2 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <SettingsSymbol
                icon="chat"
                className="h-5 w-5 text-surface-500 dark:text-surface-400"
              />
              <span className="font-medium text-surface-800 dark:text-surface-200">
                Startup and tech hiring posts
              </span>
              <span className="text-xs text-surface-500">
                (Monthly hiring posts)
              </span>
            </div>
            <label className="relative inline-flex flex-shrink-0 items-center cursor-pointer">
              <input
                type="checkbox"
                aria-label="Turn startup and tech hiring post checks on or off"
                checked={config.hn_hiring?.enabled ?? false}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    hn_hiring: {
                      ...config.hn_hiring,
                      enabled: e.target.checked,
                      remote_only: config.hn_hiring?.remote_only ?? false,
                      limit: config.hn_hiring?.limit ?? 50,
                    },
                  })
                }
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-surface-200 peer-focus-visible:ring-2 peer-focus-visible:ring-sentinel-300 rounded-full peer dark:bg-surface-700 peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-sentinel-500"></div>
            </label>
          </div>
        </div>

        <div className="space-y-2 rounded-lg border border-surface-200 p-3 text-sm text-surface-600 dark:border-surface-700 dark:text-surface-300">
          <p>
            Scheduled access to Built In, Dice, SimplyHired, and Glassdoor is
            unavailable after provider policy review. Use their user-opened
            search links, Browser Import, or manual entry.
          </p>
          <p>
            Dice&apos;s official MCP option still needs a reviewed privacy,
            schema, and pacing contract before JobSentinel can use it.
          </p>
        </div>

        <p className="flex items-start gap-1.5 text-xs text-surface-500 dark:text-surface-400 pt-2">
          <SettingsSymbol
            icon="lightbulb"
            className="mt-0.5 h-3.5 w-3.5 flex-shrink-0"
          />
          <span>
            When turned on, JobSentinel checks these job boards on your
            schedule. Choose the ones relevant to your job search.
          </span>
        </p>
      </div>
    </details>
  );
}
