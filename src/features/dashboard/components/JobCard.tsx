/** Renders one saved job with bounded evidence, guidance, and user actions. */
import { useEffect, useState, memo, type ReactNode } from "react";
import { ScoreDisplay } from "../../../ui/score-display/ScoreDisplay";
import { GhostIndicatorCompact } from "./GhostIndicator";
import { ExternalAiJobSummary } from "./ExternalAiJobSummary";
import {
  JobFitFeedbackControls,
} from "./JobFitFeedback";
import { logError } from "../../../shared/errorReporting/logger";
import { formatRelativeDate } from "../../../shared/dateFormatting";
import {
  formatSalaryRange,
  truncateJobDescription,
} from "../jobDisplayFormatting";
import {
  GOOD_JOB_MATCH_THRESHOLD,
  STRONG_JOB_MATCH_THRESHOLD,
} from "../../../shared/jobMatchScore";
import { useToast } from "../../../shared/toast/useToast";
import { isValidJobUrl } from "../jobUrlValidation";
import { openDeepLink } from "../../../shared/search-links";
import { getJobSourceGuidance } from "../../../shared/jobSourceGuidance";
import {
  getLowDetailPostingGuidance,
  getPayFloorGuidance,
  getPostingRiskGuidance,
  getSalaryRangeQualityGuidance,
  getScamRiskGuidance,
} from "./jobCardGuidance";
import {
  recordBrowserAssistLearningSignalIfEnabled,
} from "../../../shared/browserAssistLearning";
import {
  ArrowIcon,
  BookmarkIcon,
  ClockIcon,
  DuplicateIcon,
  HideIcon,
  LocationIcon,
  NotesIcon,
  ResearchIcon,
  SalaryIcon,
  SourceIcon,
} from "./JobCardIcons";
import {
  applyJobFeedbackScoreAdjustment,
  clearJobFeedbackSignal,
  getJobFeedbackKey,
  readJobFeedbackSignal,
  writeJobFeedbackSignal,
  type JobFeedbackSignal,
  type JobFeedbackVerdict,
} from "../../../shared/jobFeedbackScoring";
import { getPayTransparencyGuidance } from "../../../shared/payTransparencyRules";
import type { Job } from "../types";
import { JobCardGuidancePanels } from "./JobCardGuidancePanels";
import { JobScoreModal } from "./JobScoreModal";

interface JobCardProps {
  job: Job;
  onViewJob?: (url: string) => void;
  onHideJob?: (id: number) => void;
  onToggleBookmark?: (id: number) => void;
  onEditNotes?: (id: number, currentNotes?: string | null) => void;
  onResearchCompany?: (company: string, jobHash: string | undefined) => void;
  renderApplicationAssistAction?: (job: Job) => ReactNode;
  isSelected?: boolean;
  salaryFloorUsd?: number | null;
}

export const JobCard = memo(function JobCard({
  job,
  onViewJob,
  onHideJob,
  onToggleBookmark,
  onEditNotes,
  onResearchCompany,
  renderApplicationAssistAction,
  isSelected = false,
  salaryFloorUsd,
}: JobCardProps) {
  const [isScoreModalOpen, setIsScoreModalOpen] = useState(false);
  const jobFeedbackKey = getJobFeedbackKey({
    id: job.id,
    hash: job.hash,
    url: job.url,
  });
  const [jobFeedback, setJobFeedback] = useState<JobFeedbackSignal | null>(
    () => readJobFeedbackSignal(jobFeedbackKey),
  );
  const toast = useToast();

  useEffect(() => {
    setJobFeedback(readJobFeedbackSignal(jobFeedbackKey));
  }, [jobFeedbackKey]);

  const handleKeyDown = (e: React.KeyboardEvent, callback: () => void) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      callback();
    }
  };

  const rejectUnsafeJobLink = (url: string) => {
    if (!isValidJobUrl(url)) {
      toast.error(
        "Check job link",
        "This saved link does not look safe to open.",
      );
      return true;
    }

    return false;
  };

  const handleOpenUrl = async (url: string) => {
    if (rejectUnsafeJobLink(url)) {
      return;
    }

    try {
      await openDeepLink(url);
    } catch (err: unknown) {
      logError("Failed to open job link via Tauri command:", err);
      toast.error("Could not open job link", "Copy the link and open it in your browser.");
    }
  };

  const openJobPosting = () => {
    if (rejectUnsafeJobLink(job.url)) {
      return;
    }

    if (onViewJob) {
      onViewJob(job.url);
      return;
    }

    void handleOpenUrl(job.url);
  };

  const rawScore = job.score;
  const hasValidScore =
    typeof rawScore === "number" &&
    Number.isFinite(rawScore) &&
    rawScore >= 0 &&
    rawScore <= 1;
  const safeScore = hasValidScore ? rawScore : 0;
  const feedbackScoreAdjustment = hasValidScore
    ? applyJobFeedbackScoreAdjustment(safeScore, jobFeedback)
    : null;
  const displayedScore = feedbackScoreAdjustment?.score ?? safeScore;
  const displayedScoreValue = hasValidScore ? displayedScore : job.score;
  const isHighMatch = displayedScore >= STRONG_JOB_MATCH_THRESHOLD;
  const isGoodMatch = displayedScore >= GOOD_JOB_MATCH_THRESHOLD;
  const salaryText = formatSalaryRange(job.salary_min, job.salary_max);
  const descSnippet = truncateJobDescription(job.description);
  const rawPostingRiskScore = job.ghost_score;
  const postingRiskScore =
    typeof rawPostingRiskScore === "number" &&
    Number.isFinite(rawPostingRiskScore) &&
    rawPostingRiskScore >= 0 &&
    rawPostingRiskScore <= 1
      ? rawPostingRiskScore
      : null;
  const postingRiskGuidance = getPostingRiskGuidance(
    postingRiskScore,
    job.ghost_reasons,
  ) ?? getLowDetailPostingGuidance(job.title, job.description);
  const scamRiskGuidance = getScamRiskGuidance(job.description);
  const payFloorGuidance = getPayFloorGuidance(
    job.salary_min,
    job.salary_max,
    salaryFloorUsd,
  );
  const payTransparencyGuidance = getPayTransparencyGuidance({
    location: job.location,
    salaryMin: job.salary_min,
    salaryMax: job.salary_max,
  });
  const salaryRangeQualityGuidance =
    payFloorGuidance?.title === "Open-ended listed pay"
      ? null
      : getSalaryRangeQualityGuidance(
          job.salary_min,
          job.salary_max,
        );
  const sourceGuidance = getJobSourceGuidance(job.source);
  const sourceReviewGuidance = sourceGuidance.review;
  const hasSafeJobUrl = isValidJobUrl(job.url);
  const cardAriaLabel = `${job.title} at ${job.company}${
    safeScore >= STRONG_JOB_MATCH_THRESHOLD
      ? ", high match"
      : safeScore >= GOOD_JOB_MATCH_THRESHOLD
        ? ", good match"
        : ""
  }${salaryText || payFloorGuidance ? "" : ", pay not listed"}${
    scamRiskGuidance ? `, ${scamRiskGuidance.ariaLabel}` : ""
  }${
    postingRiskGuidance ? `, ${postingRiskGuidance.ariaLabel}` : ""
  }${sourceReviewGuidance ? `, ${sourceReviewGuidance.ariaLabel}` : ""}${
    hasSafeJobUrl ? "" : ", job link to check"
  }${
    payFloorGuidance ? `, ${payFloorGuidance.ariaLabel}` : ""
  }${
    payTransparencyGuidance ? `, ${payTransparencyGuidance.ariaLabel}` : ""
  }${
    salaryRangeQualityGuidance ? `, ${salaryRangeQualityGuidance.ariaLabel}` : ""
  }`;

  const updateJobFeedback = (
    verdict: JobFeedbackVerdict,
    mode: "set" | "toggle" = "toggle",
    recordLearning = true,
  ) => {
    if (mode === "toggle" && jobFeedback?.verdict === verdict) {
      clearJobFeedbackSignal(jobFeedbackKey);
      setJobFeedback(null);
      return;
    }

    setJobFeedback(
      writeJobFeedbackSignal({
        jobKey: jobFeedbackKey,
        verdict,
        title: job.title,
        company: job.company,
        recordedAt: new Date().toISOString(),
      }),
    );

    if (recordLearning) {
      recordBrowserAssistLearningSignalIfEnabled({
        source: "job-feedback",
        action: verdict === "useful" ? "useful" : "not_for_me",
        title: job.title,
        company: job.company,
        recordedAt: new Date().toISOString(),
      });
    }
  };

  const hideJob = () => {
    updateJobFeedback("not_useful", "set", false);
    recordBrowserAssistLearningSignalIfEnabled({
      source: "job-card",
      action: "dismissed",
      title: job.title,
      company: job.company,
      recordedAt: new Date().toISOString(),
    });
    onHideJob?.(job.id);
  };

  return (
    <>
      <JobScoreModal
        isOpen={isScoreModalOpen && hasValidScore}
        onClose={() => setIsScoreModalOpen(false)}
        score={displayedScore}
        scoreReasons={job.score_reasons}
        jobTitle={job.title}
      />

      <div
        className={`
          group relative bg-white dark:bg-surface-800 rounded-card border motion-safe:transition-all motion-safe:duration-200 motion-safe:ease-out
          motion-safe:hover:shadow-card-hover motion-safe:hover:-translate-y-0.5
          focus-within:ring-2 focus-within:ring-sentinel-500 dark:focus-within:ring-sentinel-400
          ${
            isSelected
              ? "ring-2 ring-sentinel-500 dark:ring-sentinel-400 border-sentinel-300 dark:border-sentinel-600"
              : isHighMatch
                ? "border-alert-200 dark:border-alert-700 shadow-soft hover:border-alert-300 dark:hover:border-alert-600"
                : "border-surface-100 dark:border-surface-700 shadow-soft dark:shadow-none hover:border-surface-200 dark:hover:border-surface-600"
          }
        `}
        data-testid="job-card"
        data-job-id={job.id}
        data-selected={isSelected || undefined}
        role="article"
        aria-label={cardAriaLabel}
      >
        {/* High match indicator */}
        {isHighMatch && (
          <div className="absolute -top-px -left-px -right-px h-1 bg-gradient-to-r from-alert-400 via-alert-500 to-alert-400 rounded-t-card" />
        )}

        <div className="p-4 sm:p-5">
          <div className="flex flex-col gap-4 sm:flex-row">
            {/* Score */}
            <div className="self-start sm:flex-shrink-0">
              <ScoreDisplay
                score={displayedScoreValue}
                size="md"
                showLabel={false}
                scoreReasons={job.score_reasons}
                onClick={hasValidScore ? () => setIsScoreModalOpen(true) : undefined}
                jobTitle={job.title}
              />
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0">
              {/* Title and company */}
              <h3
                data-testid="job-title"
                className="break-words [overflow-wrap:anywhere] font-display text-display-md text-surface-900 dark:text-white mb-1 group-hover:text-sentinel-600 dark:group-hover:text-sentinel-400 transition-colors"
              >
                {job.title}
              </h3>
              <p
                data-testid="job-company"
                className="break-words [overflow-wrap:anywhere] text-surface-600 dark:text-surface-400 font-medium mb-2"
              >
                {job.company}
              </p>

              {/* Description snippet */}
              {descSnippet && (
                <p className="text-sm text-surface-500 dark:text-surface-400 mb-2 line-clamp-2">
                  {descSnippet}
                </p>
              )}

              <JobCardGuidancePanels
                feedbackScoreAdjustment={feedbackScoreAdjustment}
                hasSafeJobUrl={hasSafeJobUrl}
                onOpenJob={openJobPosting}
                payFloorGuidance={payFloorGuidance}
                payTransparencyGuidance={payTransparencyGuidance}
                postingRiskGuidance={postingRiskGuidance}
                salaryRangeQualityGuidance={salaryRangeQualityGuidance}
                scamRiskGuidance={scamRiskGuidance}
                sourceReviewGuidance={sourceReviewGuidance}
              />

              {/* Meta info */}
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-surface-500 dark:text-surface-400">
                {/* Ghost indicator */}
                {postingRiskScore !== null && postingRiskScore >= 0.5 && (
                  <GhostIndicatorCompact
                    ghostScore={postingRiskScore}
                    ghostReasons={job.ghost_reasons ?? null}
                    jobId={job.id}
                  />
                )}

                {/* Location */}
                <span className="inline-flex items-center gap-1">
                  <LocationIcon />
                  {job.remote ? "Remote" : job.location || "Location TBD"}
                </span>

                {/* Salary */}
                {salaryText ? (
                  <span className="inline-flex items-center gap-1 text-success font-medium">
                    <SalaryIcon />
                    {salaryText}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-amber-600 dark:text-amber-400 font-medium">
                    <SalaryIcon />
                    Pay not listed
                  </span>
                )}

                {/* Source */}
                <span
                  className="inline-flex items-center gap-1"
                  aria-label={`Source: ${sourceGuidance.label}`}
                  aria-description={sourceGuidance.description}
                  title={sourceGuidance.description}
                >
                  <SourceIcon />
                  {sourceGuidance.label}
                </span>

                {/* Times seen (if > 1) */}
                {job.times_seen && job.times_seen > 1 && (
                  <span
                    className="inline-flex items-center gap-1 text-xs text-surface-400 dark:text-surface-500"
                    title={`This job has been seen ${job.times_seen} times. Check source details before treating repeats as separate places.`}
                  >
                    <DuplicateIcon />
                    Seen {job.times_seen} times
                  </span>
                )}

                {/* Time */}
                <span className="inline-flex items-center gap-1">
                  <ClockIcon />
                  {formatRelativeDate(job.created_at)}
                </span>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-1 self-start sm:flex-shrink-0 sm:self-center">
              {hasValidScore && (
                <JobFitFeedbackControls
                  verdict={jobFeedback?.verdict ?? null}
                  onChange={updateJobFeedback}
                />
              )}

              {onResearchCompany && (
                <button
                  onClick={() => onResearchCompany(job.company, job.hash)}
                  onKeyDown={(e) =>
                    handleKeyDown(e, () => onResearchCompany(job.company, job.hash))
                  }
                  className="p-2 text-surface-400 hover:text-purple-500 dark:hover:text-purple-400 opacity-40 group-hover:opacity-100 focus-visible:opacity-100 transition-colors"
                  aria-label="Research company"
                  title="Research company"
                  data-testid="btn-research"
                >
                  <ResearchIcon />
                </button>
              )}

              {onEditNotes && (
                <button
                  onClick={() => onEditNotes(job.id, job.notes)}
                  onKeyDown={(e) =>
                    handleKeyDown(e, () => onEditNotes(job.id, job.notes))
                  }
                  className={`p-2 transition-colors ${
                    job.notes
                      ? "text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
                      : "text-surface-400 hover:text-blue-500 dark:hover:text-blue-400 opacity-40 group-hover:opacity-100 focus-visible:opacity-100"
                  }`}
                  aria-label={job.notes ? "Edit notes" : "Add notes"}
                  data-testid="btn-notes"
                >
                  <NotesIcon filled={!!job.notes} />
                </button>
              )}

              {onToggleBookmark && (
                <button
                  onClick={() => onToggleBookmark(job.id)}
                  onKeyDown={(e) =>
                    handleKeyDown(e, () => onToggleBookmark(job.id))
                  }
                  className={`p-2 transition-colors ${
                    job.bookmarked
                      ? "text-yellow-500 hover:text-yellow-600 dark:text-yellow-400 dark:hover:text-yellow-300"
                      : "text-surface-400 hover:text-yellow-500 dark:hover:text-yellow-400 opacity-40 group-hover:opacity-100 focus-visible:opacity-100"
                  }`}
                  aria-label={
                    job.bookmarked ? "Remove bookmark" : "Bookmark this job"
                  }
                  data-testid="btn-bookmark"
                  data-bookmarked={job.bookmarked || undefined}
                >
                  <BookmarkIcon filled={job.bookmarked} />
                </button>
              )}

              {renderApplicationAssistAction?.({ ...job, score: displayedScore })}

              <ExternalAiJobSummary job={job} />

              <button
                onClick={openJobPosting}
                onKeyDown={(e) => handleKeyDown(e, openJobPosting)}
                className={`
                inline-flex items-center gap-1 px-4 py-2 rounded-lg font-medium text-sm
                transition-all duration-150 cursor-pointer
                ${
                  isGoodMatch
                    ? "bg-sentinel-50 dark:bg-sentinel-900/30 text-sentinel-600 dark:text-sentinel-400 hover:bg-sentinel-100 dark:hover:bg-sentinel-900/50"
                    : "bg-surface-50 dark:bg-surface-700 text-surface-600 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-600"
                }
              `}
                aria-label={`View ${job.title} at ${job.company} job posting`}
                data-testid="btn-view"
              >
                View
                <ArrowIcon />
              </button>

              {onHideJob && (
                <button
                  onClick={hideJob}
                  onKeyDown={(e) => handleKeyDown(e, hideJob)}
                  className="p-2 text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 transition-colors opacity-40 group-hover:opacity-100 focus-visible:opacity-100"
                  aria-label="Not interested in this job"
                  data-testid="btn-hide"
                >
                  <HideIcon />
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
});
