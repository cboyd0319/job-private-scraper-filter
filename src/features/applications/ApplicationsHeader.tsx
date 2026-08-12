import { Button } from "../../ui/Button";
import {
  AnalyticsIcon,
  AppliedIcon,
  BackIcon,
  CalendarIcon,
  OfferIcon,
  PhoneStatIcon,
  ProgressIcon,
  TemplateIcon,
} from "./ApplicationsIcons";
import { QuickStat } from "./ApplicationsBoard";
import type { ApplicationStats } from "./applicationsModel";

interface ApplicationsHeaderProps {
  onBack: () => void;
  onOpenInterviews: () => void;
  onOpenSummary: () => void;
  onOpenTemplates: () => void;
  stats: ApplicationStats | null;
}

export function ApplicationsHeader({
  onBack,
  onOpenInterviews,
  onOpenSummary,
  onOpenTemplates,
  stats,
}: ApplicationsHeaderProps) {
  return (
    <>
      {/* Header */}
      <header className="bg-white dark:bg-surface-800 border-b border-surface-100 dark:border-surface-700 sticky top-0 z-10">
        <div className="max-w-full mx-auto px-6 py-4">
          <div className="flex min-w-0 flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div className="flex min-w-0 items-start gap-3 md:items-center">
              <button
                onClick={onBack}
                className="shrink-0 p-2 text-surface-500 hover:text-surface-700 dark:text-surface-400 dark:hover:text-surface-200 transition-colors"
                aria-label="Go back"
              >
                <BackIcon />
              </button>
              <div className="min-w-0">
                <h1 className="break-words font-display text-display-md text-surface-900 dark:text-white">
                  Application Tracker
                </h1>
                <p className="break-words text-sm text-surface-500 dark:text-surface-400">
                  Track each application from saved job to final decision.
                </p>
              </div>
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2 md:justify-end">
              <Button onClick={() => onOpenTemplates()} variant="secondary" className="min-w-0">
                <TemplateIcon />
                Templates
              </Button>
              <Button onClick={() => onOpenInterviews()} variant="secondary" className="min-w-0">
                <CalendarIcon />
                Interviews
              </Button>
              <Button onClick={() => onOpenSummary()} variant="secondary" className="min-w-0">
                <AnalyticsIcon />
                Summary
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Quick Stats Bar */}
      {stats && (
        <div className="bg-white dark:bg-surface-800 border-b border-surface-100 dark:border-surface-700 px-6 py-3">
          <div className="flex min-w-0 flex-wrap items-center gap-x-6 gap-y-2 text-sm">
            <QuickStat
              label="Applied"
              value={stats.totalApplied}
              icon={<AppliedIcon />}
            />
            <QuickStat
              label="Interviews"
              value={stats.interviews}
              percent={stats.interviewRate}
              icon={<PhoneStatIcon />}
            />
            <QuickStat
              label="Offers"
              value={stats.offers}
              percent={stats.offerRate}
              icon={<OfferIcon />}
              highlight
            />
            <QuickStat
              label="In Progress"
              value={stats.inProgress}
              icon={<ProgressIcon />}
            />
          </div>
        </div>
      )}
    </>
  );
}
