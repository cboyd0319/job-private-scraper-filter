import { useState } from "react";
import { Card } from "../../../ui/Card";
import { ScoreDisplay } from "../../../ui/score-display/ScoreDisplay";
import { CheckIcon, XIcon } from "./ResumeIcons";
import { ResumeScoreBreakdownRow } from "./ResumeScoreBreakdownRow";
import { MatchDebuggerModal } from "./MatchDebuggerModal";
import { MilitaryTransitionReviewModal } from "./MilitaryTransitionReviewModal";
import {
  isScoreFraction,
  parseGapAnalysisLine,
  type MatchResult,
  type ResumeMatchFeedbackLabel,
} from "./resumePageModel";

interface ResumeRecentMatchesProps {
  matches: MatchResult[];
  savingFeedback: boolean;
  onFeedback: (match: MatchResult, label: ResumeMatchFeedbackLabel) => void;
}

export function ResumeRecentMatches({
  matches,
  savingFeedback,
  onFeedback,
}: ResumeRecentMatchesProps) {
  const [debugMatch, setDebugMatch] = useState<MatchResult | null>(null);
  const [militaryTransitionMatch, setMilitaryTransitionMatch] = useState<MatchResult | null>(null);

  return (
    <Card className="lg:col-span-3 dark:bg-surface-800">
      <h2 className="font-display text-display-sm text-surface-900 dark:text-white mb-4">
        Recent Resume Matches
      </h2>

      {matches.length === 0 ? (
        <div className="text-center py-8">
          <p className="text-surface-500 dark:text-surface-400 mb-2">
            No job matches yet
          </p>
          <p className="text-sm text-surface-400 dark:text-surface-500">
            Match results will appear here when you view job details from the dashboard
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {matches.map((match) => {
            const skillsScore = isScoreFraction(match.skills_match_score)
              ? match.skills_match_score
              : null;
            const experienceScore = isScoreFraction(match.experience_match_score)
              ? match.experience_match_score
              : null;
            const educationScore = isScoreFraction(match.education_match_score)
              ? match.education_match_score
              : null;

            return (
              <div
                key={match.job_hash}
                className="p-4 border border-surface-200 dark:border-surface-700 rounded-lg hover:border-surface-300 dark:hover:border-surface-600 transition-colors"
              >
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="font-medium text-surface-800 dark:text-surface-200">
                      {match.job_title}
                    </h3>
                    <p className="text-sm text-surface-500 dark:text-surface-400">
                      {match.company}
                    </p>
                  </div>
                  <ScoreDisplay score={match.overall_match_score} size="sm" />
                </div>

                {(skillsScore !== null ||
                  experienceScore !== null ||
                  educationScore !== null) && (
                  <div className="mb-4 p-3 bg-surface-50 dark:bg-surface-700 rounded-lg">
                    <p className="text-xs font-medium text-surface-600 dark:text-surface-400 mb-2">
                      Fit Details
                    </p>
                    <div className="space-y-2">
                      {skillsScore !== null && (
                        <ResumeScoreBreakdownRow
                          label="Skills fit"
                          score={skillsScore}
                          barClassName="bg-sentinel-500"
                        />
                      )}
                      {experienceScore !== null && (
                        <ResumeScoreBreakdownRow
                          label="Experience fit"
                          score={experienceScore}
                          barClassName="bg-info"
                        />
                      )}
                      {educationScore !== null && (
                        <ResumeScoreBreakdownRow
                          label="Education fit"
                          score={educationScore}
                          barClassName="bg-blue-500"
                        />
                      )}
                    </div>
                  </div>
                )}

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <p className="text-xs font-medium text-green-600 dark:text-green-400 mb-2 flex items-center gap-1">
                      <CheckIcon className="w-3.5 h-3.5" />
                      Skills found in both ({match.matching_skills.length})
                    </p>
                    <div className="flex flex-wrap gap-1.5">
                      {match.matching_skills.length > 0 ? (
                        match.matching_skills.map((skill) => (
                          <span
                            key={skill}
                            className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-md border border-green-200 dark:border-green-800"
                          >
                            <CheckIcon className="w-3 h-3" />
                            {skill}
                          </span>
                        ))
                      ) : (
                        <span className="text-xs text-surface-400 dark:text-surface-500 italic">
                          No shared skills found in the available text
                        </span>
                      )}
                    </div>
                  </div>

                  <div>
                    <p className="text-xs font-medium text-red-600 dark:text-red-400 mb-2 flex items-center gap-1">
                      <XIcon className="w-3.5 h-3.5" />
                      Skills to review ({match.missing_skills.length})
                    </p>
                    <div className="flex flex-wrap gap-1.5">
                      {match.missing_skills.length > 0 ? (
                        match.missing_skills.map((skill) => (
                          <span
                            key={skill}
                            className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded-md border border-red-200 dark:border-red-800"
                          >
                            <XIcon className="w-3 h-3" />
                            {skill}
                          </span>
                        ))
                      ) : (
                        <span className="text-xs text-green-600 dark:text-green-400">
                          No obvious skill gaps found in the available text.
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {match.gap_analysis && (
                  <div className="mt-3 pt-3 border-t border-surface-200 dark:border-surface-700">
                    <p className="text-xs font-medium text-surface-600 dark:text-surface-400 mb-2">
                      What to Review
                    </p>
                    <ul className="space-y-1">
                      {match.gap_analysis.split("\n").map((line, idx) => {
                        const parsedLine = parseGapAnalysisLine(line);
                        if (!parsedLine.text) return null;
                        const isMatch = parsedLine.status === "match";
                        const isMissing = parsedLine.status === "missing";
                        return (
                          <li
                            key={idx}
                            className={`text-sm flex items-start gap-2 ${
                              isMatch
                                ? "text-green-600 dark:text-green-400"
                                : isMissing
                                  ? "text-red-600 dark:text-red-400"
                                  : "text-surface-500 dark:text-surface-400"
                            }`}
                          >
                            {isMatch && (
                              <CheckIcon className="w-4 h-4 mt-0.5 flex-shrink-0" />
                            )}
                            {isMissing && (
                              <XIcon className="w-4 h-4 mt-0.5 flex-shrink-0" />
                            )}
                            <span>{parsedLine.text}</span>
                          </li>
                        );
                      })}
                    </ul>
                  </div>
                )}

                <div className="mt-3 pt-3 border-t border-surface-200 dark:border-surface-700">
                  <p className="text-xs text-surface-500 dark:text-surface-400 mb-2">
                    Was this local match useful? This stays on this device and does not change
                    ranking yet.
                  </p>
                  <div className="flex flex-wrap gap-2">
                    {Number.isSafeInteger(match.id) && match.id > 0 && (
                      <>
                        <button
                          type="button"
                          onClick={() => setDebugMatch(match)}
                          className="rounded-md border border-surface-300 px-3 py-1.5 text-sm font-medium text-surface-600 hover:bg-surface-50 dark:border-surface-600 dark:text-surface-300 dark:hover:bg-surface-700"
                        >
                          Inspect evidence
                        </button>
                        <button
                          type="button"
                          onClick={() => setMilitaryTransitionMatch(match)}
                          className="rounded-md border border-surface-300 px-3 py-1.5 text-sm font-medium text-surface-600 hover:bg-surface-50 dark:border-surface-600 dark:text-surface-300 dark:hover:bg-surface-700"
                        >
                          Review military wording
                        </button>
                      </>
                    )}
                    {([
                      ["useful", "Useful"],
                      ["not_relevant", "Not relevant"],
                    ] as const).map(([label, text]) => {
                      const selected = match.feedback?.label === label;
                      return (
                        <button
                          key={label}
                          type="button"
                          aria-pressed={selected}
                          disabled={savingFeedback}
                          onClick={() => onFeedback(match, label)}
                          className={`rounded-md border px-3 py-1.5 text-sm font-medium disabled:opacity-50 ${
                            selected
                              ? "border-sentinel-600 bg-sentinel-50 text-sentinel-800 dark:bg-sentinel-900/30 dark:text-sentinel-200"
                              : "border-surface-300 text-surface-600 hover:bg-surface-50 dark:border-surface-600 dark:text-surface-300 dark:hover:bg-surface-700"
                          }`}
                        >
                          {text}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
      <MatchDebuggerModal
        isOpen={debugMatch !== null}
        match={debugMatch}
        onClose={() => setDebugMatch(null)}
      />
      <MilitaryTransitionReviewModal
        isOpen={militaryTransitionMatch !== null}
        match={militaryTransitionMatch}
        onClose={() => setMilitaryTransitionMatch(null)}
      />
    </Card>
  );
}
