/** Renders interview details, feedback, debrief, and job-scoped research actions. */

import { useState } from "react";
import { Button } from "../../ui/Button";
import { Modal } from "../../ui/Modal";
import { formatInterviewDate } from "../../shared/dateFormatting";
import type {
  CompanyResearchTarget,
  RenderCompanyResearch,
} from "../../shared/companyResearch";
import {
  INTERVIEW_TYPES,
  OUTCOME_COLORS,
  PREP_CHECKLIST,
  TYPE_COLORS,
  formatOutcomeLabel,
  type Interview,
  type PrepProgress,
} from "./InterviewSchedulerModel";
import { CalendarIcon, DownloadIcon, LocationIcon, SearchIcon, UserIcon } from "./InterviewSchedulerIcons";

const DEBRIEF_TEXT_FIELDS = [
  { id: "questions", label: "Questions asked", placeholder: "Questions the interviewer asked" },
  { id: "concerns", label: "Concerns", placeholder: "Anything that needs clarification" },
  { id: "nextSteps", label: "Promised next steps", placeholder: "Who promised what happens next" },
] as const;

type Debrief = Record<(typeof DEBRIEF_TEXT_FIELDS)[number]["id"] | "followUpDeadline", string>;

const EMPTY_DEBRIEF: Debrief = { questions: "", concerns: "", nextSteps: "", followUpDeadline: "" };

function formatDebrief(debrief: Debrief) {
  const rows: Array<[string, string]> = [
    ["Questions asked", debrief.questions],
    ["Concerns", debrief.concerns],
    ["Promised next steps", debrief.nextSteps],
    ["Follow-up deadline", debrief.followUpDeadline],
  ];
  return rows.filter(([, value]) => value.trim())
    .map(([label, value]) => `${label}: ${value.trim()}`)
    .join("\n");
}

interface InterviewDetailPanelsProps {
  completing: boolean;
  deleting: boolean;
  interview: Interview;
  onClose: () => void;
  onComplete: (interview: Interview, outcome: string, notes?: string) => void;
  onDelete: (interviewId: number) => void;
  onExportICal: (interview: Interview) => void;
  onPrepToggle: (itemId: string) => void;
  prepProgress: PrepProgress;
  renderCompanyResearch?: RenderCompanyResearch;
}

export function InterviewDetailPanels({
  completing,
  deleting,
  interview,
  onClose,
  onComplete,
  onDelete,
  onExportICal,
  onPrepToggle,
  prepProgress,
  renderCompanyResearch,
}: InterviewDetailPanelsProps) {
  const [debrief, setDebrief] = useState(EMPTY_DEBRIEF);
  const [feedbackOutcome, setFeedbackOutcome] = useState("");
  const [researchTarget, setResearchTarget] = useState<CompanyResearchTarget | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showFeedbackForm, setShowFeedbackForm] = useState(false);
  const handleOpenCompanyResearch = () => setResearchTarget({
    companyName: interview.company,
    jobHash: interview.job_hash,
  });

  return (
    <>
      <Modal isOpen onClose={onClose} title={interview.job_title} description={interview.company} size="md">
          <div className="space-y-4">
              <span className={`inline-flex px-2 py-0.5 text-xs rounded-full font-medium ${TYPE_COLORS[interview.interview_type] || TYPE_COLORS.other}`}>
                {INTERVIEW_TYPES.find((t) => t.value === interview.interview_type)?.label || interview.interview_type}
              </span>

              <div className="space-y-2 text-sm">
                <div className="flex items-center gap-2">
                  <CalendarIcon className="w-4 h-4 text-surface-400" />
                  <span>{formatInterviewDate(interview.scheduled_at)}</span>
                  <span className="text-surface-400">({interview.duration_minutes} min)</span>
                </div>
                {interview.location && (
                  <div className="flex items-center gap-2">
                    <LocationIcon />
                    <span>{interview.location}</span>
                  </div>
                )}
                {interview.interviewer_name && (
                  <div className="flex items-center gap-2">
                    <UserIcon />
                    <span>
                      {interview.interviewer_name}
                      {interview.interviewer_title && ` - ${interview.interviewer_title}`}
                    </span>
                  </div>
                )}
              </div>

              {interview.notes && (
                <div className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg text-sm">
                  <p className="font-medium text-surface-700 dark:text-surface-300 mb-1">Prep Notes</p>
                  <p className="text-surface-600 dark:text-surface-400">{interview.notes}</p>
                </div>
              )}

              <div className="p-3 bg-surface-50 dark:bg-surface-700 rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <p className="font-medium text-surface-700 dark:text-surface-300 text-sm">Interview Prep</p>
                  <span className="text-xs text-surface-500">
                    {Object.values(prepProgress).filter(Boolean).length}/{PREP_CHECKLIST.length} done
                  </span>
                </div>
                <div className="space-y-1.5">
                  {PREP_CHECKLIST.map((item) => (
                    <label
                      key={item.id}
                      className="flex items-center gap-2 cursor-pointer group"
                    >
                      <input
                        type="checkbox"
                        checked={prepProgress[item.id] || false}
                        onChange={() => onPrepToggle(item.id)}
                        className="w-4 h-4 rounded border-surface-300 dark:border-surface-600 text-sentinel-500 focus-visible:ring-sentinel-500"
                      />
                      <span className={`text-sm ${prepProgress[item.id] ? 'text-surface-400 line-through' : 'text-surface-600 dark:text-surface-400'}`}>
                        {item.label}
                      </span>
                      {item.id === "research" && renderCompanyResearch && (
                        <button
                          onClick={(e) => {
                            e.preventDefault();
                            handleOpenCompanyResearch();
                          }}
                          className="text-xs text-sentinel-600 dark:text-sentinel-400 hover:underline ml-auto opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                          Research →
                        </button>
                      )}
                    </label>
                  ))}
                </div>
              </div>

              <div className="flex gap-2">
                <Button variant="secondary" onClick={() => onExportICal(interview)} className="flex items-center gap-1">
                  <DownloadIcon />
                  Add to Calendar
                </Button>
                {renderCompanyResearch && (
                  <Button variant="secondary" onClick={handleOpenCompanyResearch} className="flex items-center gap-1">
                    <SearchIcon />
                    Research
                  </Button>
                )}
              </div>

              <div className="border-t border-surface-200 dark:border-surface-600 pt-4">
                {!showFeedbackForm ? (
                  <>
                    <p className="text-sm font-medium text-surface-700 dark:text-surface-300 mb-3">
                      How did it go?
                    </p>
                    <div className="grid gap-2 sm:grid-cols-3">
                      <Button
                        variant="secondary"
                        onClick={() => { setFeedbackOutcome("passed"); setShowFeedbackForm(true); }}
                        className="flex-1"
                      >
                        Went well
                      </Button>
                      <Button
                        variant="secondary"
                        onClick={() => { setFeedbackOutcome("pending"); setShowFeedbackForm(true); }}
                        className="flex-1"
                      >
                        Not sure yet
                      </Button>
                      <Button
                        variant="secondary"
                        onClick={() => { setFeedbackOutcome("failed"); setShowFeedbackForm(true); }}
                        className="flex-1"
                      >
                        Not a fit
                      </Button>
                    </div>
                  </>
                ) : (
                  <div className="space-y-3">
                    <div>
                      <h3 className="font-semibold text-surface-900 dark:text-white">
                        Post-interview debrief
                      </h3>
                      <p className="text-sm text-surface-500">
                        Saved locally when you complete the interview. Nothing is sent and application status is unchanged.
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-medium text-surface-700 dark:text-surface-300">
                        Signal strength:
                      </p>
                      <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${OUTCOME_COLORS[feedbackOutcome] || OUTCOME_COLORS.pending}`}>
                        {formatOutcomeLabel(feedbackOutcome)}
                      </span>
                    </div>
                    {DEBRIEF_TEXT_FIELDS.map((field) => (
                      <label className="block text-sm font-medium text-surface-700 dark:text-surface-300" key={field.id}>
                        {field.label}
                        <textarea
                          value={debrief[field.id]}
                          onChange={(event) => setDebrief((current) => ({ ...current, [field.id]: event.target.value }))}
                          rows={2}
                          placeholder={field.placeholder}
                          maxLength={200}
                          className="mt-1 w-full resize-none rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
                        />
                      </label>
                    ))}
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300">
                      Follow-up deadline
                      <input
                        type="date"
                        value={debrief.followUpDeadline}
                        onChange={(event) => setDebrief((current) => ({ ...current, followUpDeadline: event.target.value }))}
                        className="mt-1 w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
                      />
                    </label>
                    <div className="flex flex-col gap-2 sm:flex-row">
                      <Button
                        variant="secondary"
                        onClick={() => { setShowFeedbackForm(false); setFeedbackOutcome(""); setDebrief(EMPTY_DEBRIEF); }}
                        disabled={completing}
                        className="flex-1"
                      >
                        Back
                      </Button>
                      <Button
                        variant="primary"
                        onClick={() => onComplete(interview, feedbackOutcome, formatDebrief(debrief))}
                        loading={completing}
                        loadingText="Saving..."
                        className="flex-1"
                      >
                        Save debrief
                      </Button>
                    </div>
                  </div>
                )}
              </div>

              <div className="flex gap-3 pt-2">
                {showDeleteConfirm ? (
                  <>
                    <span className="text-sm text-red-600 dark:text-red-400 self-center">
                      Delete this interview?
                    </span>
                    <Button
                      variant="secondary"
                      onClick={() => setShowDeleteConfirm(false)}
                      disabled={deleting}
                    >
                      Cancel
                    </Button>
                    <Button
                      variant="danger"
                      onClick={() => onDelete(interview.id)}
                      loading={deleting}
                      loadingText="Deleting..."
                    >
                      Confirm Delete
                    </Button>
                  </>
                ) : (
                  <Button
                    variant="secondary"
                    onClick={() => setShowDeleteConfirm(true)}
                    className="text-red-600 dark:text-red-400"
                  >
                    Delete
                  </Button>
                )}
              </div>
          </div>
      </Modal>

      {researchTarget &&
        renderCompanyResearch?.({
          companyName: researchTarget.companyName,
          jobHash: researchTarget.jobHash,
          onClose: () => setResearchTarget(null),
        })}
    </>
  );
}
