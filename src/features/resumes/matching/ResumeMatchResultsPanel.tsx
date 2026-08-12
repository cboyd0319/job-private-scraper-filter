import { Badge } from "../../../ui/Badge";
import { Button } from "../../../ui/Button";
import { Card, CardHeader } from "../../../ui/Card";
import { LoadingSpinner } from "../../../ui/LoadingSpinner";
import { ResumeRequirementEvidence } from "../shared/ResumeRequirementEvidence";
import { getScoreColor, getScoreLabel } from "../shared/resumeScore";
import { JobWordsOverviewCard } from "./ResumeMatchJobWordsOverview";
import { ResumeRoleFamilyCoverageCard } from "./ResumeRoleFamilyCoverageCard";
import {
  buildResumeNextActions,
  formatHardConstraintCategory,
  formatIssueSeverity,
  formatRequirementEvidenceSections,
  formatRequirementState,
  formatSuggestionCategory,
  getResumeFitEvidenceStatus,
  type AtsAnalysisResult,
} from "./resumeMatchModel";
import {
  getImportanceVariant,
  getMissingKeywordDetails,
  getMissingKeywordGroups,
  getRequirementStateVariant,
  getSeverityVariant,
} from "./resumeMatchEvidence";
import {
  HighlightKeywordGroups,
  HighlightKeywords,
  ScoreItem,
} from "./resumeMatchHighlights";

interface ResumeMatchResultsPanelProps {
  analyzing: boolean;
  analysisResult: AtsAnalysisResult | null;
  canShowComparison: boolean;
  showComparison: boolean;
  jobDescription: string;
  comparisonResumeText: string;
  onToggleComparison: () => void;
  onReviewInResumeBuilder?: () => void;
}

export function ResumeMatchResultsPanel({
  analyzing,
  analysisResult,
  canShowComparison,
  showComparison,
  jobDescription,
  comparisonResumeText,
  onToggleComparison,
  onReviewInResumeBuilder,
}: ResumeMatchResultsPanelProps) {
  if (analyzing) {
    return (
      <Card className="flex items-center justify-center py-32">
        <div className="text-center">
          <LoadingSpinner size="lg" />
          <p className="mt-4 text-surface-600 dark:text-surface-400">
            Reviewing resume...
          </p>
        </div>
      </Card>
    );
  }

  if (!analysisResult) {
    return (
      <Card className="text-center py-32">
        <div className="w-16 h-16 bg-sentinel-50 dark:bg-sentinel-900/30 rounded-lg flex items-center justify-center mx-auto mb-4">
          <svg
            className="w-8 h-8 text-sentinel-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        </div>
        <h3 className="font-display text-display-md text-surface-900 dark:text-white mb-2">
          No review yet
        </h3>
        <p className="text-surface-500 dark:text-surface-400">
          Choose or add a resume, paste a job post, then review the match
        </p>
      </Card>
    );
  }

  const fitEvidenceStatus = getResumeFitEvidenceStatus(analysisResult);
  const matchedKeywords = analysisResult.keyword_matches.map((match) => match.keyword);
  const missingKeywordDetails = getMissingKeywordDetails(analysisResult);
  const missingKeywordGroups = getMissingKeywordGroups(missingKeywordDetails);
  const hardConstraintRisks = analysisResult.hard_constraint_risks ?? [];
  const requirementReviews = analysisResult.requirement_reviews ?? [];
  const resumeNextActions = buildResumeNextActions(analysisResult);

  return (
    <>
      <div className="flex gap-3">
        {canShowComparison && (
          <Button
            onClick={onToggleComparison}
            variant={showComparison ? "primary" : "secondary"}
            className="flex-1"
          >
            {showComparison ? "Hide Comparison" : "Show Comparison"}
          </Button>
        )}
        {onReviewInResumeBuilder && (
          <Button
            onClick={onReviewInResumeBuilder}
            variant="success"
            className="flex-1"
          >
            Review in Resume Builder
          </Button>
        )}
      </div>

      {showComparison && canShowComparison && (
        <Card>
          <CardHeader title="Resume-Job Comparison" />
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div className="min-w-0 border-b border-surface-200 pb-4 dark:border-surface-700 md:border-b-0 md:border-r md:pb-0 md:pr-4">
              <h3 className="font-semibold text-surface-800 dark:text-surface-200 mb-3 flex items-center gap-2">
                <svg
                  className="w-5 h-5 text-sentinel-500"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
                Job Requirements
              </h3>
              <div className="max-h-96 overflow-y-auto rounded-lg bg-surface-50 p-4 font-mono text-sm text-surface-700 whitespace-pre-wrap break-words [overflow-wrap:anywhere] dark:bg-surface-800 dark:text-surface-300">
                <HighlightKeywordGroups
                  text={jobDescription}
                  matchedKeywords={matchedKeywords}
                  missingKeywords={missingKeywordDetails.map((gap) => gap.keyword)}
                />
              </div>
            </div>

            <div className="min-w-0 md:pl-4">
              <h3 className="font-semibold text-surface-800 dark:text-surface-200 mb-3 flex items-center gap-2">
                <svg
                  className="w-5 h-5 text-green-500"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                Your Resume
              </h3>
              <div className="max-h-96 overflow-y-auto rounded-lg bg-surface-50 p-4 font-mono text-sm text-surface-700 whitespace-pre-wrap break-words [overflow-wrap:anywhere] dark:bg-surface-800 dark:text-surface-300">
                <HighlightKeywords
                  text={comparisonResumeText}
                  keywords={matchedKeywords}
                  type="match"
                />
              </div>
            </div>
          </div>
          <div className="mt-4 flex items-start gap-2 p-3 bg-sentinel-50 dark:bg-sentinel-900/20 rounded-lg">
            <svg
              className="w-5 h-5 text-sentinel-600 dark:text-sentinel-400 flex-shrink-0 mt-0.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <div className="text-sm">
              <p className="text-sentinel-800 dark:text-sentinel-300">
                <span className="bg-green-200 dark:bg-green-900/50 text-green-900 dark:text-green-100 px-1 rounded mr-1">
                  Green
                </span>
                = Words found in both
                <span className="ml-4 bg-red-200 dark:bg-red-900/50 text-red-900 dark:text-red-100 px-1 rounded mr-1">
                  Red
                </span>
                = Words to review
              </p>
            </div>
          </div>
        </Card>
      )}

      <JobWordsOverviewCard
        keywordMatches={analysisResult.keyword_matches}
        formatEvidenceSections={formatRequirementEvidenceSections}
      />

      <Card>
        <CardHeader title="Resume Fit" />
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div className="text-center">
            <div className={`text-2xl font-bold ${getScoreColor(analysisResult.overall_score)}`}>
              {getScoreLabel(analysisResult.overall_score)}
            </div>
            <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
              Overall fit
            </p>
          </div>
          <div className="space-y-3">
            <ScoreItem label="Job words" score={analysisResult.keyword_score} />
          </div>
        </div>
        <p className="mt-3 text-xs text-surface-500 dark:text-surface-400">
          This score is an evidence-based estimate for this posting, not a hiring decision.
        </p>
        {fitEvidenceStatus && (
          <div className="mt-4 rounded-lg border border-surface-200 bg-surface-50 p-3 text-sm dark:border-surface-700 dark:bg-surface-800/70">
            <div className="mb-1 flex flex-wrap items-center gap-2">
              <span className="font-medium text-surface-800 dark:text-surface-100">
                Evidence status
              </span>
              <Badge variant={fitEvidenceStatus.variant} size="sm">
                {fitEvidenceStatus.label}
              </Badge>
            </div>
            <p className="text-surface-600 dark:text-surface-300">
              {fitEvidenceStatus.detail}
            </p>
          </div>
        )}
      </Card>

      <Card>
        <CardHeader title="Resume Quality" />
        <div className="space-y-3">
          <ScoreItem label="Readable format" score={analysisResult.format_score} />
          <ScoreItem label="Details included" score={analysisResult.completeness_score} />
        </div>
      </Card>

      <ResumeRoleFamilyCoverageCard />

      {resumeNextActions.length > 0 && (
        <Card>
          <CardHeader title="What To Do Next" />
          <div className="space-y-3">
            {resumeNextActions.map((action, idx) => (
              <div
                key={`${action.title}-${idx}`}
                className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
              >
                <div className="flex flex-wrap items-start gap-2 mb-2">
                  <Badge variant={action.variant} size="sm">
                    {action.label}
                  </Badge>
                  <p className="font-medium text-surface-900 dark:text-surface-100 flex-1 min-w-48">
                    {action.title}
                  </p>
                </div>
                <p className="text-sm text-surface-700 dark:text-surface-300">
                  {action.detail}
                </p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {hardConstraintRisks.length > 0 && (
        <Card>
          <CardHeader title={`Hard Requirements To Check (${hardConstraintRisks.length})`} />
          <div className="space-y-3">
            {hardConstraintRisks.map((risk, idx) => (
              <div
                key={`${risk.requirement}-${idx}`}
                className="p-3 bg-danger/5 dark:bg-danger/10 rounded-lg border border-danger/20"
              >
                <div className="flex flex-wrap items-start gap-2 mb-2">
                  <Badge variant="danger" size="sm">Check first</Badge>
                  <Badge variant="surface" size="sm">
                    {formatHardConstraintCategory(risk.category)}
                  </Badge>
                  <p className="font-medium text-surface-900 dark:text-surface-100 flex-1 min-w-48">
                    {risk.requirement}
                  </p>
                </div>
                <p className="text-sm text-surface-700 dark:text-surface-300">
                  {risk.action}
                </p>
                <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">
                  Fit label is limited until this is confirmed.
                </p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {requirementReviews.length > 0 && (
        <Card>
          <CardHeader title={`Requirement Review (${requirementReviews.length})`} />
          <div className="space-y-3">
            {requirementReviews.map((review, idx) => (
              <div
                key={`${review.keyword}-${idx}`}
                className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
              >
                <div className="flex flex-wrap items-start gap-2">
                  <Badge variant={getRequirementStateVariant(review.match_state)} size="sm">
                    {formatRequirementState(review.match_state)}
                  </Badge>
                  <Badge variant={getImportanceVariant(review.importance)} size="sm">
                    {review.importance}
                  </Badge>
                  {review.hard_constraint && (
                    <Badge variant="danger" size="sm">Hard requirement</Badge>
                  )}
                  <p className="font-medium text-surface-900 dark:text-surface-100 flex-1 min-w-48">
                    {review.keyword}
                  </p>
                </div>
                <ResumeRequirementEvidence review={review} />
                <p className="text-sm text-surface-700 dark:text-surface-300 mt-2">
                  {review.recommendation}
                </p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {analysisResult.keyword_matches.length > 0 && (
        <Card>
          <CardHeader title={`Words Found (${analysisResult.keyword_matches.length})`} />
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {analysisResult.keyword_matches.map((match, idx) => (
              <div
                key={idx}
                className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <p className="break-words [overflow-wrap:anywhere] font-medium text-surface-800 dark:text-surface-200">
                      {match.keyword}
                    </p>
                    <p className="mt-0.5 break-words text-xs text-surface-500 [overflow-wrap:anywhere] dark:text-surface-400">
                      Found in: {formatRequirementEvidenceSections(match.found_in)}
                    </p>
                  </div>
                  <Badge variant={getImportanceVariant(match.importance)} size="sm" className="shrink-0">
                    {match.importance}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {missingKeywordDetails.length > 0 && (
        <Card>
          <CardHeader title={`Words To Review (${missingKeywordDetails.length})`} />
          <div className="space-y-4">
            {missingKeywordGroups.required.length > 0 && (
              <div>
                <h4 className="text-sm font-semibold text-danger mb-2">
                  Required to Review
                </h4>
                <div className="flex flex-wrap gap-2">
                  {missingKeywordGroups.required.map((gap, idx) => (
                    <Badge key={idx} variant="danger">
                      {gap.keyword}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
            {missingKeywordGroups.preferred.length > 0 && (
              <div>
                <h4 className="text-sm font-semibold text-alert-700 dark:text-alert-400 mb-2">
                  Preferred to Review
                </h4>
                <div className="flex flex-wrap gap-2">
                  {missingKeywordGroups.preferred.map((gap, idx) => (
                    <Badge key={idx} variant="alert">
                      {gap.keyword}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
            {missingKeywordGroups.other.length > 0 && (
              <div>
                <h4 className="text-sm font-semibold text-surface-700 dark:text-surface-300 mb-2">
                  Nice-to-Have or Other to Review
                </h4>
                <div className="flex flex-wrap gap-2">
                  {missingKeywordGroups.other.map((gap, idx) => (
                    <Badge key={idx} variant="surface">
                      {gap.keyword}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
            <div className="space-y-1 text-xs text-surface-500 dark:text-surface-400">
              <p>Start with required job-post language. Preferred words can help later.</p>
              <p>Only use these words when they honestly fit your experience and improve clarity.</p>
              <p>Do not force words you cannot support with real work, training, or credentials.</p>
            </div>
          </div>
        </Card>
      )}

      {analysisResult.format_issues.length > 0 && (
        <Card>
          <CardHeader title={`Details to Check (${analysisResult.format_issues.length})`} />
          <div className="space-y-3 max-h-80 overflow-y-auto">
            {analysisResult.format_issues.map((issue, idx) => (
              <div
                key={idx}
                className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
              >
                <div className="flex items-start gap-2 mb-2">
                  <Badge variant={getSeverityVariant(issue.severity)} size="sm">
                    {formatIssueSeverity(issue.severity)}
                  </Badge>
                  <p className="font-medium text-surface-800 dark:text-surface-200 flex-1">
                    {issue.issue}
                  </p>
                </div>
                <p className="text-sm text-surface-600 dark:text-surface-400 bg-sentinel-50 dark:bg-sentinel-900/20 px-2 py-1 rounded">
                  Possible edit to review: {issue.fix}
                </p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {analysisResult.suggestions.length > 0 && (
        <Card>
          <CardHeader title={`Suggestions (${analysisResult.suggestions.length})`} />
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {analysisResult.suggestions.map((suggestion, idx) => (
              <details
                key={idx}
                className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
              >
                <summary className="cursor-pointer font-medium text-surface-800 dark:text-surface-200 flex items-center gap-2">
                  <Badge variant="sentinel" size="sm">
                    {formatSuggestionCategory(suggestion.category)}
                  </Badge>
                  <span className="flex-1">{suggestion.suggestion}</span>
                </summary>
                <p className="text-sm text-surface-600 dark:text-surface-400 mt-2 pl-2">
                  Why it helps: {suggestion.impact}
                </p>
              </details>
            ))}
          </div>
        </Card>
      )}
    </>
  );
}
