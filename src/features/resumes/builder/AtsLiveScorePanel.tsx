/**
 * AtsLiveScorePanel - real-time resume readability feedback for Resume Builder
 *
 * Shows live readability feedback as users build their resume, with detailed
 * breakdowns for job-word fit, format, and details included.
 *
 * @version 2.5.5
 */

import { memo, useState, useEffect, useMemo } from "react";
import { invoke } from "../../../platform/tauri";
import { Badge } from "../../../ui/Badge";
import { Button } from "../../../ui/Button";
import { Tooltip } from "../../../ui/Tooltip";
import { logError } from "../../../shared/errorReporting/logger";
import { getUserFriendlyError } from "../../../shared/errorReporting/messages";
import {
  getScoreBg,
  getScoreColor,
  getScoreDisplayValue,
  getScoreLabel,
  getScoreProgressPercent,
} from "../shared/resumeScore";
import { takeStoredResumeJobContext } from "../shared/resumeJobContext";
import { AtsLiveScoreDetailsModal } from "./AtsLiveScoreDetailsModal";
import type { ResumeData } from "./resumeBuilderData";
import { toResumeAnalysisInput } from "./resumeBuilderTransforms";
import {
  formatHardConstraintRiskCategory,
  getHardConstraintRisks,
  getStepTips,
  type AtsAnalysisResult,
} from "./AtsLiveScorePanelModel";

export type { AtsAnalysisResult } from "./AtsLiveScorePanelModel";

interface AtsLiveScorePanelProps {
  resumeData: ResumeData | null;
  currentStep: number;
  debounceMs?: number;
  showFullAnalysis?: boolean;
}

export const AtsLiveScorePanel = memo(function AtsLiveScorePanel({
  resumeData,
  currentStep,
  debounceMs = 1000,
  showFullAnalysis = true,
}: AtsLiveScorePanelProps) {
  const [analysis, setAnalysis] = useState<AtsAnalysisResult | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [showDetailModal, setShowDetailModal] = useState(false);
  const [jobDescription, setJobDescription] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retryTrigger, setRetryTrigger] = useState(0);
  const [evidenceRevision, setEvidenceRevision] = useState(() => crypto.randomUUID());

  useEffect(() => {
    setEvidenceRevision(crypto.randomUUID());
  }, [resumeData]);

  // Load job context from sessionStorage (set by Resume Match)
  useEffect(() => {
    const stored = takeStoredResumeJobContext();
    if (stored) {
      setJobDescription(stored.description);
    }
  }, []);

  // Analyze resume with debouncing
  useEffect(() => {
    if (!resumeData) {
      setAnalysis(null);
      return;
    }

    // Check minimum data requirements
    if (!resumeData.contact.name || !resumeData.contact.email) {
      return;
    }

    const timeoutId = setTimeout(async () => {
      try {
        setAnalyzing(true);
        setError(null);

        const atsData = toResumeAnalysisInput(resumeData, evidenceRevision);

        let result: AtsAnalysisResult;
        if (jobDescription) {
          // Full analysis with job context
          result = await invoke<AtsAnalysisResult>("analyze_resume_for_job", {
            resume: atsData,
            jobDescription,
          });
        } else {
          // Format-only analysis
          result = await invoke<AtsAnalysisResult>("analyze_resume_format", {
            resume: atsData,
          });
        }

        setAnalysis(result);
      } catch (err: unknown) {
        logError("Resume readability analysis error:", err);
        const friendly = getUserFriendlyError(err);
        setError(friendly.action ?? friendly.message);
      } finally {
        setAnalyzing(false);
      }
    }, debounceMs);

    return () => clearTimeout(timeoutId);
  }, [resumeData, jobDescription, debounceMs, evidenceRevision, retryTrigger]);

  // Memoize tips
  const tips = useMemo(() => getStepTips(currentStep, analysis), [currentStep, analysis]);
  const hardConstraintRisks = useMemo(() => getHardConstraintRisks(analysis), [analysis]);

  // Calculate ring progress
  const ringScore = analysis ? getScoreProgressPercent(analysis.overall_score) : 0;
  const ringProgress = 2 * Math.PI * 32 * (1 - ringScore / 100);

  return (
    <div className="bg-white dark:bg-surface-800 rounded-lg border border-surface-200 dark:border-surface-700 shadow-sm">
      {/* Header */}
      <div className="px-4 py-3 border-b border-surface-200 dark:border-surface-700">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-surface-800 dark:text-surface-200 flex items-center gap-2">
            <svg className="w-4 h-4 text-sentinel-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            Resume Readability
            {analyzing && (
              <span className="text-xs text-surface-400 animate-pulse">checking...</span>
            )}
          </h3>
          {jobDescription && (
            <Tooltip content="Checking against saved job description" position="left">
              <Badge variant="sentinel" size="sm">Saved Job</Badge>
            </Tooltip>
          )}
        </div>
      </div>

      {/* Readability display */}
      <div className="p-4">
        {error ? (
          <div className="text-center py-4">
            <div className="w-10 h-10 mx-auto mb-3 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
              <svg className="w-5 h-5 text-red-600 dark:text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <p className="text-sm text-red-600 dark:text-red-400 mb-3">{error}</p>
            <div className="flex gap-2 justify-center">
              <Button size="sm" variant="ghost" onClick={() => setError(null)}>
                Close Message
              </Button>
              <Button size="sm" onClick={() => { setError(null); setRetryTrigger((n) => n + 1); }}>
                Try Again
              </Button>
            </div>
          </div>
        ) : analysis ? (
          <div className="space-y-4">
            {/* Circular readability summary */}
            <div className="flex items-center gap-4">
              <div className="relative w-20 h-20 flex-shrink-0">
                <svg className="w-20 h-20 transform -rotate-90">
                  <circle
                    cx="40"
                    cy="40"
                    r="32"
                    stroke="currentColor"
                    strokeWidth="6"
                    fill="none"
                    className="text-surface-200 dark:text-surface-700"
                  />
                  <circle
                    cx="40"
                    cy="40"
                    r="32"
                    stroke="currentColor"
                    strokeWidth="6"
                    fill="none"
                    strokeDasharray={`${2 * Math.PI * 32}`}
                    strokeDashoffset={ringProgress}
                    className={getScoreBg(analysis.overall_score).replace("bg-", "text-")}
                    strokeLinecap="round"
                  />
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className={`text-xl font-bold ${getScoreColor(analysis.overall_score)}`}>
                    {getScoreDisplayValue(analysis.overall_score)}
                  </span>
                  <span className={`text-[10px] font-medium ${getScoreColor(analysis.overall_score)}`}>
                    {getScoreLabel(analysis.overall_score)}
                  </span>
                </div>
              </div>

              {/* Readability breakdown */}
              <div className="flex-1 space-y-2">
                <ScoreBar label="Job words" score={analysis.keyword_score} />
                <ScoreBar label="Format" score={analysis.format_score} />
                <ScoreBar label="Details" score={analysis.completeness_score} />
              </div>
            </div>

            {/* Quick Stats */}
            {(analysis.keyword_matches.length > 0 ||
              analysis.missing_keywords.length > 0 ||
              analysis.format_issues.length > 0 ||
              hardConstraintRisks.length > 0) && (
              <div className="flex flex-wrap gap-2 text-xs">
                {analysis.keyword_matches.length > 0 && (
                  <span className="px-2 py-1 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded">
                    {analysis.keyword_matches.length} job words found
                  </span>
                )}
                {analysis.missing_keywords.length > 0 && (
                  <span className="px-2 py-1 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 rounded">
                    {analysis.missing_keywords.length} to review
                  </span>
                )}
                {analysis.format_issues.length > 0 && (
                  <span className="px-2 py-1 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 rounded">
                    {analysis.format_issues.length} details to check
                  </span>
                )}
                {hardConstraintRisks.length > 0 && (
                  <span className="px-2 py-1 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 rounded">
                    {hardConstraintRisks.length} must-have{hardConstraintRisks.length === 1 ? "" : "s"} to check
                  </span>
                )}
              </div>
            )}

            {hardConstraintRisks.length > 0 && (
              <div className="border-l-4 border-danger bg-danger/5 dark:bg-danger/10 px-3 py-2">
                <p className="text-xs font-semibold text-danger dark:text-red-300">
                  Must-haves need review before tailoring
                </p>
                <ul className="mt-1 space-y-1">
                  {hardConstraintRisks.slice(0, 2).map((risk, idx) => (
                    <li
                      key={`${risk.category}-${risk.requirement}-${idx}`}
                      className="text-xs text-surface-700 dark:text-surface-300"
                    >
                      <span className="font-medium">{formatHardConstraintRiskCategory(risk)}:</span>{" "}
                      Check {risk.requirement} before editing this resume.
                    </li>
                  ))}
                </ul>
                {hardConstraintRisks.length > 2 && (
                  <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
                    {hardConstraintRisks.length - 2} more must-have{hardConstraintRisks.length - 2 === 1 ? "" : "s"} in details.
                  </p>
                )}
              </div>
            )}

            {/* Review Details Button */}
            {showFullAnalysis && (
              <Button
                size="sm"
                variant="secondary"
                className="w-full"
                onClick={() => setShowDetailModal(true)}
              >
                Review Details
              </Button>
            )}
          </div>
        ) : (
          <div className="text-center py-4">
            <div className="w-12 h-12 bg-surface-100 dark:bg-surface-700 rounded-full flex items-center justify-center mx-auto mb-3">
              <svg className="w-6 h-6 text-surface-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </div>
            <p className="text-sm text-surface-500 dark:text-surface-400">
              Fill in your resume to see readability feedback
            </p>
          </div>
        )}
      </div>

      {/* Tips Section */}
      {tips.length > 0 && (
        <div className="px-4 py-3 bg-surface-50 dark:bg-surface-900/50 border-t border-surface-200 dark:border-surface-700">
          <p className="text-xs font-semibold text-surface-700 dark:text-surface-300 mb-2">
            Quick Tips
          </p>
          <ul className="space-y-1">
            {tips.map((tip, idx) => (
              <li key={idx} className="text-xs text-surface-600 dark:text-surface-400 flex items-start gap-1.5">
                <span className="text-sentinel-500 mt-0.5">•</span>
                <span>{tip}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <AtsLiveScoreDetailsModal
        analysis={analysis}
        hardConstraintRisks={hardConstraintRisks}
        isOpen={showDetailModal}
        onClose={() => setShowDetailModal(false)}
      />
    </div>
  );
});

// Helper component for readability evidence bars.
const ScoreBar = memo(function ScoreBar({ label, score }: { label: string; score: number }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-surface-600 dark:text-surface-400 w-16 truncate">{label}</span>
      <div className="flex-1 h-1.5 bg-surface-200 dark:bg-surface-700 rounded-full overflow-hidden">
        <div
          className={`h-full transition-all duration-300 ${getScoreBg(score)}`}
          style={{ width: `${getScoreProgressPercent(score)}%` }}
        />
      </div>
      <span className="text-xs font-semibold text-surface-700 dark:text-surface-300 w-24 text-right">
        {getScoreLabel(score)}
      </span>
    </div>
  );
});

export default AtsLiveScorePanel;
