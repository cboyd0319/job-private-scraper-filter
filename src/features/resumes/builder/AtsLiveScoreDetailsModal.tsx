import { memo } from "react";
import { Badge } from "../../../ui/Badge";
import { Button } from "../../../ui/Button";
import { Modal, ModalFooter } from "../../../ui/Modal";
import { Tooltip } from "../../../ui/Tooltip";
import { ResumeRequirementEvidence } from "../shared/ResumeRequirementEvidence";
import { getScoreColor, getScoreLabel } from "../shared/resumeScore";
import {
  formatHardConstraintRiskCategory,
  formatIssueSeverity,
  formatResumeEvidenceSections,
  formatSuggestionCategory,
  getMissingKeywordDetails,
  getMissingKeywordGroups,
  type AtsAnalysisResult,
  type HardConstraintRisk,
} from "./AtsLiveScorePanelModel";

interface AtsLiveScoreDetailsModalProps {
  analysis: AtsAnalysisResult | null;
  hardConstraintRisks: HardConstraintRisk[];
  isOpen: boolean;
  onClose: () => void;
}

export function AtsLiveScoreDetailsModal({
  analysis,
  hardConstraintRisks,
  isOpen,
  onClose,
}: AtsLiveScoreDetailsModalProps) {
  const requirementEvidence =
    analysis?.requirement_reviews?.filter(
      (review) => (review.evidence_citations?.length ?? 0) > 0,
    ) ?? [];

  return (
    <Modal
            isOpen={isOpen}
            onClose={onClose}
            title="Resume Readability Review"
            size="lg"
          >
            {analysis && (
              <div className="space-y-6">
                {/* Match overview */}
                <div className="grid grid-cols-4 gap-4">
                  <ScoreCard label="Overall" score={analysis.overall_score} />
                  <ScoreCard label="Job words" score={analysis.keyword_score} />
                  <ScoreCard label="Format" score={analysis.format_score} />
                  <ScoreCard label="Details included" score={analysis.completeness_score} />
                </div>

                {/* Job words found */}
                {analysis.keyword_matches.length > 0 && (
                  <div>
                    <h4 className="text-sm font-semibold text-surface-800 dark:text-surface-200 mb-3">
                      Words Found ({analysis.keyword_matches.length})
                    </h4>
                    <div className="flex flex-wrap gap-2">
                      {analysis.keyword_matches.map((match, idx) => (
                        <Tooltip
                          key={idx}
                          content={`Found in: ${formatResumeEvidenceSections(match.found_in)}`}
                          position="top"
                        >
                          <Badge
                            variant={
                              match.importance === "Required"
                                ? "danger"
                                : match.importance === "Preferred"
                                ? "alert"
                                : "success"
                            }
                            size="sm"
                          >
                            {match.keyword}
                          </Badge>
                        </Tooltip>
                      ))}
                    </div>
                  </div>
                )}

                {requirementEvidence.length > 0 && (
                  <div>
                    <h4 className="mb-3 text-sm font-semibold text-surface-800 dark:text-surface-200">
                      Where Matching Wording Was Found
                    </h4>
                    <div className="space-y-3">
                      {requirementEvidence.map((review, index) => (
                        <div
                          key={`${review.keyword}-${index}`}
                          className="rounded-lg border border-surface-200 bg-surface-50 p-3 dark:border-surface-600 dark:bg-surface-700"
                        >
                          <p className="text-sm font-medium text-surface-800 dark:text-surface-200">
                            {review.keyword}
                          </p>
                          <ResumeRequirementEvidence review={review} />
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Must-haves to check */}
                {hardConstraintRisks.length > 0 && (
                  <div>
                    <h4 className="text-sm font-semibold text-surface-800 dark:text-surface-200 mb-3">
                      Must-Haves To Check ({hardConstraintRisks.length})
                    </h4>
                    <div className="space-y-2">
                      {hardConstraintRisks.map((risk, idx) => (
                        <div
                          key={`${risk.category}-${risk.requirement}-${idx}`}
                          className="p-3 bg-danger/5 dark:bg-danger/10 rounded-lg border border-danger/20"
                        >
                          <div className="flex items-start gap-2">
                            <Badge variant="danger" size="sm">
                              {formatHardConstraintRiskCategory(risk)}
                            </Badge>
                            <div className="flex-1">
                              <p className="text-sm font-medium text-surface-800 dark:text-surface-200">
                                Check {risk.requirement}
                              </p>
                              <p className="text-xs text-surface-600 dark:text-surface-400 mt-1">
                                {risk.action}
                              </p>
                              <p className="text-xs text-surface-500 dark:text-surface-500 mt-1">
                                {risk.reason}
                              </p>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Words to review */}
                {getMissingKeywordDetails(analysis).length > 0 && (
                  <div>
                    <h4 className="text-sm font-semibold text-surface-800 dark:text-surface-200 mb-3">
                      Words To Review ({getMissingKeywordDetails(analysis).length})
                    </h4>
                    {(() => {
                      const { required, preferred, other } = getMissingKeywordGroups(analysis);

                      return (
                        <div className="space-y-3">
                          {required.length > 0 && (
                            <div>
                              <p className="text-xs font-semibold text-danger mb-2">Required to Review</p>
                              <div className="flex flex-wrap gap-2">
                                {required.map((gap, idx) => (
                                  <Badge key={idx} variant="danger" size="sm">
                                    {gap.keyword}
                                  </Badge>
                                ))}
                              </div>
                            </div>
                          )}
                          {preferred.length > 0 && (
                            <div>
                              <p className="text-xs font-semibold text-alert-700 dark:text-alert-400 mb-2">
                                Preferred to Review
                              </p>
                              <div className="flex flex-wrap gap-2">
                                {preferred.map((gap, idx) => (
                                  <Badge key={idx} variant="alert" size="sm">
                                    {gap.keyword}
                                  </Badge>
                                ))}
                              </div>
                            </div>
                          )}
                          {other.length > 0 && (
                            <div>
                              <p className="text-xs font-semibold text-surface-700 dark:text-surface-300 mb-2">
                                Nice-to-Have or Other to Review
                              </p>
                              <div className="flex flex-wrap gap-2">
                                {other.map((gap, idx) => (
                                  <Badge key={idx} variant="surface" size="sm">
                                    {gap.keyword}
                                  </Badge>
                                ))}
                              </div>
                            </div>
                          )}
                          <div className="space-y-1 text-xs text-surface-500 dark:text-surface-400">
                            <p>Start with required job-post language. Preferred words can help later.</p>
                            <p>Only use these words when they honestly fit your experience and improve clarity.</p>
                          </div>
                        </div>
                      );
                    })()}
                  </div>
                )}

                {/* Details to check */}
                {analysis.format_issues.length > 0 && (
                  <div>
                    <h4 className="text-sm font-semibold text-surface-800 dark:text-surface-200 mb-3">
                      Details to Check ({analysis.format_issues.length})
                    </h4>
                    <div className="space-y-2">
                      {analysis.format_issues.map((issue, idx) => (
                        <div
                          key={idx}
                          className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
                        >
                          <div className="flex items-start gap-2">
                            <Badge
                              variant={
                                issue.severity === "Critical"
                                  ? "danger"
                                  : issue.severity === "Warning"
                                  ? "alert"
                                  : "sentinel"
                              }
                              size="sm"
                            >
                              {formatIssueSeverity(issue.severity)}
                            </Badge>
                            <div className="flex-1">
                              <p className="text-sm text-surface-800 dark:text-surface-200">{issue.issue}</p>
                              <p className="text-xs text-sentinel-600 dark:text-sentinel-400 mt-1">
                                Possible edit to review: {issue.fix}
                              </p>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Suggestions */}
                {analysis.suggestions.length > 0 && (
                  <div>
                    <h4 className="text-sm font-semibold text-surface-800 dark:text-surface-200 mb-3">
                      Suggestions ({analysis.suggestions.length})
                    </h4>
                    <div className="space-y-2">
                      {analysis.suggestions.map((suggestion, idx) => (
                        <details
                          key={idx}
                          className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
                        >
                          <summary className="cursor-pointer text-sm font-medium text-surface-800 dark:text-surface-200 flex items-center gap-2">
                            <Badge variant="sentinel" size="sm">
                              {formatSuggestionCategory(suggestion.category)}
                            </Badge>
                            {suggestion.suggestion}
                          </summary>
                          <p className="text-xs text-surface-600 dark:text-surface-400 mt-2 ml-6">
                            Why it helps: {suggestion.impact}
                          </p>
                        </details>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
            <ModalFooter>
              <Button onClick={onClose}>Close</Button>
            </ModalFooter>
    </Modal>
  );
}

// Helper component for readability cards in modal.
const ScoreCard = memo(function ScoreCard({ label, score }: { label: string; score: number }) {
  return (
    <div className="text-center p-3 bg-surface-50 dark:bg-surface-700 rounded-lg">
      <div className={`text-sm font-semibold ${getScoreColor(score)}`}>
        {getScoreLabel(score)}
      </div>
      <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">{label}</p>
    </div>
  );
});
