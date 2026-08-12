/** Renders the filtered dashboard job list and job-scoped actions. */

import type { ReactNode, RefObject } from "react";
import { Button } from "../../../ui/Button";
import { Card, CardHeader } from "../../../ui/Card";
import { JobCard } from "./JobCard";
import type { Job } from "../types";
import { BriefcaseIcon, FilterIcon } from "./DashboardIcons";
import type { NoJobsEmptyStateCopy } from "./noJobsEmptyStateCopy";

interface DashboardJobListProps {
  jobs: Job[];
  filteredJobs: Job[];
  noJobsCopy: NoJobsEmptyStateCopy;
  noSourcesEnabled: boolean;
  searching: boolean;
  jobListRef: RefObject<HTMLDivElement | null>;
  bulkMode: boolean;
  selectedJobIds: Set<number>;
  isKeyboardActive: boolean;
  selectedIndex: number;
  salaryFloorUsd: number | null;
  onSearchNow: () => void;
  onOpenSettings: () => void;
  onOpenImport: () => void;
  onClearFilters: () => void;
  onToggleJobSelection: (id: number) => void;
  onHideJob: (id: number) => void;
  onToggleBookmark: (id: number) => void;
  onEditNotes: (id: number, currentNotes?: string | null) => void;
  onResearchCompany: (company: string, jobHash: string | undefined) => void;
  renderApplicationAssistAction?: (job: Job) => ReactNode;
}

export function DashboardJobList({
  jobs,
  filteredJobs,
  noJobsCopy,
  noSourcesEnabled,
  searching,
  jobListRef,
  bulkMode,
  selectedJobIds,
  isKeyboardActive,
  selectedIndex,
  salaryFloorUsd,
  onSearchNow,
  onOpenSettings,
  onOpenImport,
  onClearFilters,
  onToggleJobSelection,
  onHideJob,
  onToggleBookmark,
  onEditNotes,
  onResearchCompany,
  renderApplicationAssistAction,
}: DashboardJobListProps) {
  if (jobs.length === 0) {
    return (
      <Card
        className="text-center py-12 dark:bg-surface-800"
        data-tour="job-list"
        role="status"
        aria-live="polite"
      >
        <div className="w-12 h-12 bg-sentinel-50 dark:bg-sentinel-900/30 rounded-lg flex items-center justify-center mx-auto mb-4">
          <BriefcaseIcon className="w-6 h-6 text-sentinel-400" />
        </div>
        <CardHeader
          title={noJobsCopy.title}
          subtitle={noJobsCopy.subtitle}
        />
        <div className="mt-4 flex flex-wrap justify-center gap-3">
          <Button
            onClick={noSourcesEnabled ? onOpenSettings : onSearchNow}
            loading={!noSourcesEnabled && searching}
          >
            {noJobsCopy.primaryLabel}
          </Button>
          <Button
            onClick={noSourcesEnabled ? onOpenImport : onOpenSettings}
            variant="secondary"
          >
            {noJobsCopy.secondaryLabel}
          </Button>
        </div>
        <p className="mt-3 text-sm text-surface-500 dark:text-surface-400">
          {noJobsCopy.helperText}
        </p>
        <div className="mt-8 pt-6 border-t border-surface-200 dark:border-surface-700">
          <p className="text-sm text-surface-500 dark:text-surface-400 mb-4">
            How JobSentinel works:
          </p>
          <div className="flex flex-col gap-4 max-w-xs mx-auto text-left">
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-sentinel-100 dark:bg-sentinel-900/30 rounded-full flex items-center justify-center text-sentinel-600 dark:text-sentinel-400 font-semibold text-sm">
                1
              </div>
              <div>
                <p className="text-sm font-medium text-surface-700 dark:text-surface-300">
                  {noJobsCopy.firstStepTitle}
                </p>
                <p className="text-xs text-surface-500 dark:text-surface-400">
                  {noJobsCopy.firstStepDescription}
                </p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-sentinel-100 dark:bg-sentinel-900/30 rounded-full flex items-center justify-center text-sentinel-600 dark:text-sentinel-400 font-semibold text-sm">
                2
              </div>
              <div>
                <p className="text-sm font-medium text-surface-700 dark:text-surface-300">
                  Show useful evidence
                </p>
                <p className="text-xs text-surface-500 dark:text-surface-400">
                  Match, pay, and posting risk stay visible
                </p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-sentinel-100 dark:bg-sentinel-900/30 rounded-full flex items-center justify-center text-sentinel-600 dark:text-sentinel-400 font-semibold text-sm">
                3
              </div>
              <div>
                <p className="text-sm font-medium text-surface-700 dark:text-surface-300">
                  You choose
                </p>
                <p className="text-xs text-surface-500 dark:text-surface-400">
                  Open the source, save notes, or skip
                </p>
              </div>
            </div>
          </div>
        </div>
      </Card>
    );
  }

  if (filteredJobs.length === 0) {
    return (
      <Card
        className="text-center py-8 dark:bg-surface-800"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        <div className="w-12 h-12 bg-surface-100 dark:bg-surface-700 rounded-full flex items-center justify-center mx-auto mb-3">
          <FilterIcon className="w-6 h-6 text-surface-400" />
        </div>
        <h3 className="font-medium text-surface-700 dark:text-surface-300 mb-2">
          No jobs match your filters
        </h3>
        <p className="text-sm text-surface-500 dark:text-surface-400 mb-4">
          Try changing or clearing filters to see more jobs.
        </p>
        <button
          onClick={onClearFilters}
          data-testid="btn-clear-filters-empty-state"
          className="text-sm text-sentinel-600 dark:text-sentinel-400 hover:underline"
          aria-label="Clear all filters to show all jobs"
        >
          Clear all filters
        </button>
      </Card>
    );
  }

  return (
    <>
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {filteredJobs.length} job{filteredJobs.length === 1 ? "" : "s"} found
      </div>
      <div
        ref={jobListRef}
        className="space-y-3 stagger-children"
        data-testid="job-list"
      >
        {filteredJobs.map((job, index) => (
          <div key={job.id} className="flex items-start gap-3">
            {bulkMode && (
              <div className="flex-shrink-0 pt-5">
                <input
                  type="checkbox"
                  checked={selectedJobIds.has(job.id)}
                  onChange={() => onToggleJobSelection(job.id)}
                  className="w-5 h-5 rounded border-surface-300 dark:border-surface-600 text-sentinel-500 focus-visible:ring-sentinel-500"
                  aria-label={`Select ${job.title}`}
                />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <JobCard
                job={job}
                onHideJob={bulkMode ? undefined : onHideJob}
                onToggleBookmark={bulkMode ? undefined : onToggleBookmark}
                onEditNotes={bulkMode ? undefined : onEditNotes}
                onResearchCompany={bulkMode ? undefined : onResearchCompany}
                renderApplicationAssistAction={
                  bulkMode ? undefined : renderApplicationAssistAction
                }
                isSelected={isKeyboardActive && index === selectedIndex}
                salaryFloorUsd={salaryFloorUsd}
              />
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
