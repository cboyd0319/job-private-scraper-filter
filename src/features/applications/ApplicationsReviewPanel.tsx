import { Badge } from "../../ui/Badge";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import type {
  ApplicationReviewAction,
  ApplicationReviewSummary,
} from "./applicationsModel";

interface ApplicationsReviewPanelProps {
  summary: ApplicationReviewSummary;
  onSelectAction: (action: ApplicationReviewAction) => void;
  onOpenSummary: () => void;
  onGoToJobs: () => void;
  onImportJob?: () => void;
}

const PRIORITY_LABELS: Record<ApplicationReviewAction["priority"], string> = {
  high: "Do first",
  medium: "Next",
  low: "When ready",
};

const PRIORITY_BADGES: Record<ApplicationReviewAction["priority"], "alert" | "sentinel" | "surface"> = {
  high: "alert",
  medium: "sentinel",
  low: "surface",
};

export function ApplicationsReviewPanel({
  summary,
  onSelectAction,
  onOpenSummary,
  onGoToJobs,
  onImportJob,
}: ApplicationsReviewPanelProps) {
  return (
    <Card
      className="mb-6 dark:bg-surface-800"
      data-testid="application-review-panel"
    >
      <div className="flex min-w-0 flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="min-w-0">
          <p className="text-xs font-semibold uppercase tracking-wide text-sentinel-700 dark:text-sentinel-300">
            Daily mission
          </p>
          <h2 className="mt-1 font-display text-display-sm text-surface-900 dark:text-white">
            {summary.title}
          </h2>
          <p className="mt-1 max-w-3xl text-sm text-surface-600 dark:text-surface-400">
            {summary.description}
          </p>
        </div>
        <div className="flex shrink-0 flex-wrap gap-2">
          <Button variant="secondary" onClick={onOpenSummary}>
            Open Summary
          </Button>
          <Button variant="secondary" onClick={onGoToJobs}>
            Jobs
          </Button>
          {onImportJob && (
            <Button variant="secondary" onClick={onImportJob}>
              Import Job
            </Button>
          )}
        </div>
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {summary.actions.map((action) => (
          <ReviewActionItem
            key={action.id}
            action={action}
            onClick={() => onSelectAction(action)}
          />
        ))}
      </div>
    </Card>
  );
}

function ReviewActionItem({
  action,
  onClick,
}: {
  action: ApplicationReviewAction;
  onClick: () => void;
}) {
  return (
    <div className="flex min-w-0 flex-col justify-between border-t border-surface-200 pt-4 dark:border-surface-700">
      <div className="min-w-0">
        <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2">
          <Badge variant={PRIORITY_BADGES[action.priority]} size="sm">
            {PRIORITY_LABELS[action.priority]}
          </Badge>
        </div>
        <h3 className="break-words text-sm font-semibold text-surface-900 dark:text-white">
          {action.title}
        </h3>
        <p className="mt-1 break-words text-sm text-surface-600 dark:text-surface-400">
          {action.description}
        </p>
        {action.handoff && (
          <div className="mt-3 rounded-md border border-surface-200 bg-surface-50 px-3 py-2 text-xs text-surface-700 dark:border-surface-700 dark:bg-surface-900/40 dark:text-surface-300">
            <p className="font-semibold">
              After this: {action.handoff.label}
            </p>
            <p className="mt-1 leading-5">{action.handoff.description}</p>
          </div>
        )}
      </div>
      <Button
        className="mt-4 w-full whitespace-normal"
        size="sm"
        variant={action.priority === "high" ? "primary" : "secondary"}
        onClick={onClick}
      >
        {getActionButtonLabel(action)}
      </Button>
    </div>
  );
}

function getActionButtonLabel(action: ApplicationReviewAction): string {
  switch (action.kind) {
    case "reminders":
      return "Review reminders";
    case "no_response":
      return "Review quiet roles";
    case "interviews":
      return "Open interviews";
    case "offers":
      return "Review offer and pay";
    case "to_apply":
      return action.applicationId ? "Review this tracked role" : "Add or import job";
    case "weekly_review":
      return "Review weekly plan";
    case "source_review":
      return "Review job sources";
    case "steady":
      return "Open summary";
  }
}
