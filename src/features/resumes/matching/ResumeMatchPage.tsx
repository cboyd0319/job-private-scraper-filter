import { useState, useCallback, useEffect } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import { Card, CardHeader } from "../../../ui/Card";
import { Badge } from "../../../ui/Badge";
import { useToast } from "../../../shared/toast/useToast";
import { logError } from "../../../shared/errorReporting/logger";
import { ResumeMatchTools } from "./ResumeMatchTools";
import { ResumeMatchResultsPanel } from "./ResumeMatchResultsPanel";
import { ResumeMatchingProfileFields } from "./ResumeMatchingProfileFields";
import {
  readResumeMatchDraft,
  writeResumeMatchDraft,
} from "./resumeMatchDraft";
import {
  getResumeAnalysisErrorAction,
  getSelectedResumeReadableStatus,
  isResumeSummary,
  parseAtsResumeInput,
  type AtsAnalysisResult,
  type ResumeSummary,
} from "./resumeMatchModel";
import { writeStoredResumeJobContext } from "../shared/resumeJobContext";
import type { ResumeMatchingProfile } from "../shared/atsAnalysisContracts";

type Page = "dashboard" | "applications" | "resume" | "resume-builder" | "ats-optimizer" | "salary" | "market" | "automation";

interface ResumeMatchProps {
  onBack: () => void;
  onNavigate?: (page: Page) => void;
}
async function getActiveResumeSummary(): Promise<ResumeSummary | null> {
  const selected = await invoke<unknown>("get_active_resume");
  return isResumeSummary(selected) ? selected : null;
}

export default function ResumeMatch({ onBack, onNavigate }: ResumeMatchProps) {
  const [initialDraft] = useState(readResumeMatchDraft);
  const [jobDescription, setJobDescription] = useState(initialDraft?.jobDescription ?? "");
  const [resumeJson, setResumeJson] = useState(initialDraft?.resumeJson ?? "");
  const [analyzing, setAnalyzing] = useState(false);
  const [analysisResult, setAnalysisResult] = useState<AtsAnalysisResult | null>(initialDraft?.analysisResult ?? null);
  const [actionWords, setActionWords] = useState<string[]>([]);
  const [showActionWords, setShowActionWords] = useState(false);
  const [bulletInput, setBulletInput] = useState("");
  const [improvedBullet, setImprovedBullet] = useState("");
  const [improvingBullet, setImprovingBullet] = useState(false);
  const [showBulletImprover, setShowBulletImprover] = useState(false);
  const [showComparison, setShowComparison] = useState(initialDraft?.showComparison ?? false);
  const [showAdvancedResumeImport, setShowAdvancedResumeImport] = useState(initialDraft?.showAdvancedResumeImport ?? false);
  const [activeResume, setActiveResume] = useState<ResumeSummary | null>(null);
  const [analysisInputSource, setAnalysisInputSource] = useState<"active" | "copied" | null>(
    initialDraft?.analysisInputSource ?? null,
  );
  const [matchingProfile, setMatchingProfile] = useState<ResumeMatchingProfile | null>(
    initialDraft?.matchingProfile ?? null,
  );

  const toast = useToast();

  useEffect(() => {
    let cancelled = false;

    const loadActiveResume = async () => {
      try {
        const selected = await getActiveResumeSummary();
        if (cancelled || !selected) return;

        setActiveResume(selected);
      } catch (err: unknown) {
        if (!cancelled) {
          logError("Could not load active resume:", err);
        }
      }
    };

    void loadActiveResume();

    return () => {
      cancelled = true;
    };
  }, []);

  const handleChooseResume = async () => {
    try {
      const selected = await getActiveResumeSummary();
      if (selected) {
        setActiveResume(selected);
        setAnalysisResult(null);
        setAnalysisInputSource(null);
        setShowComparison(false);
        toast.success("Resume selected", `${selected.name} is ready for job match review.`);
        return;
      }
    } catch (err: unknown) {
      logError("Could not load active resume:", err);
    }

    if (onNavigate) {
      writeResumeMatchDraft({
        jobDescription,
        resumeJson,
        analysisResult,
        analysisInputSource,
        matchingProfile,
        showAdvancedResumeImport,
        showComparison,
      });
      onNavigate("resume");
      return;
    }

    toast.info("Open Resume Match", "Use the Resumes page to choose or add a resume.");
  };

  // Load action words on mount
  const loadActionWords = useCallback(async () => {
    try {
      const words = await invoke<string[]>("get_ats_power_words");
      setActionWords(words);
    } catch (err: unknown) {
      logError("Failed to load action words:", err);
    }
  }, []);

  // Analyze resume
  const handleAnalyze = async () => {
    if (!jobDescription.trim()) {
      toast.error("Add job post", "Paste the job post, then review again.");
      return;
    }

    const useCopiedResume = showAdvancedResumeImport && resumeJson.trim().length > 0;

    if (!useCopiedResume && !activeResume) {
      toast.error(
        "Choose a resume first",
        "Choose or add a resume, or use Import from Resume App if you already have an export.",
      );
      return;
    }

    try {
      setAnalyzing(true);
      const result = useCopiedResume
        ? await (() => {
            const resume = parseAtsResumeInput(resumeJson);
            if (!resume) {
              throw new Error("invalid-copied-resume-details");
            }
            return invoke<AtsAnalysisResult>("analyze_resume_for_job", {
              resume,
              jobDescription,
              ...(matchingProfile ? { matchingProfile } : {}),
            });
          })()
        : await invoke<AtsAnalysisResult>("analyze_active_resume_for_job", {
            jobDescription,
            ...(matchingProfile ? { matchingProfile } : {}),
          });

      setAnalysisResult(result);
      setAnalysisInputSource(useCopiedResume ? "copied" : "active");
      setShowComparison(false);
      toast.success(
        "Review ready",
        "Use the details below as a guide before you apply.",
      );
    } catch (err: unknown) {
      if (err instanceof Error && err.message === "invalid-copied-resume-details") {
        toast.error(
          "Could not read copied resume details",
          "Choose or add a resume instead, or paste copied resume details from JobSentinel or another resume app.",
        );
      } else {
        toast.error("Review could not run", getResumeAnalysisErrorAction(err));
        logError("Analysis error:", err);
      }
    } finally {
      setAnalyzing(false);
    }
  };

  // Analyze format only
  const handleAnalyzeFormat = async () => {
    if (!resumeJson.trim()) {
      toast.error(
        "Choose a resume first",
        "Choose or add a resume, or use Import from Resume App if you already have an export.",
      );
      return;
    }

    const resume = parseAtsResumeInput(resumeJson);
    if (!resume) {
      toast.error(
        "Could not read copied resume details",
        "Choose or add a resume instead, or paste copied resume details from JobSentinel or another resume app.",
      );
      return;
    }

    try {
      setAnalyzing(true);
      const result = await invoke<AtsAnalysisResult>("analyze_resume_format", {
        resume,
      });

      setAnalysisResult(result);
      toast.success("Format review complete", "Review the format details below.");
    } catch (err: unknown) {
      toast.error("Review could not run", getResumeAnalysisErrorAction(err));
      logError("Format analysis error:", err);
    } finally {
      setAnalyzing(false);
    }
  };

  // Draft a reviewed alternative for a resume bullet.
  const handleImproveBullet = async () => {
    if (!bulletInput.trim()) {
      toast.error("Add a bullet point", "Paste or write one bullet, then draft again.");
      return;
    }

    try {
      setImprovingBullet(true);
      const improved = await invoke<string>("improve_bullet_point", {
        bullet: bulletInput,
        jobContext: jobDescription.trim() || null,
      });

      setImprovedBullet(improved);
      toast.success("Draft ready", "Review the suggested version below");
    } catch (err: unknown) {
      toast.error("Could not draft bullet", getResumeAnalysisErrorAction(err));
      logError("Bullet draft error:", err);
    } finally {
      setImprovingBullet(false);
    }
  };

  // Send the saved job post to Resume Builder.
  const handleReviewInResumeBuilder = () => {
    if (!onNavigate) {
      toast.error(
        "Could not open Resume Builder",
        "Open Resume Builder from the sidebar, then paste this job post."
      );
      return;
    }

    const saved = writeStoredResumeJobContext(jobDescription);
    if (!saved) {
      toast.error(
        "Could not open Resume Builder with this job",
        "Copy the job post and paste it in Resume Builder instead."
      );
      return;
    }

    onNavigate("resume-builder");
    toast.success("Opening Resume Builder", "This job post is ready there.");
  };

  const canShowComparison =
    analysisInputSource === "copied" && Boolean(jobDescription.trim()) && Boolean(resumeJson.trim());

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      {/* Header */}
      <header className="bg-white dark:bg-surface-800 border-b border-surface-200 dark:border-surface-700 shadow-soft">
        <div className="max-w-7xl mx-auto px-6 py-6">
          <div className="flex items-center gap-4">
            <button
              onClick={onBack}
              className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 transition-colors"
              title="Back to Dashboard"
              aria-label="Back to Dashboard"
            >
              <svg className="w-5 h-5 text-surface-600 dark:text-surface-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
              </svg>
            </button>
            <div>
              <h1 className="font-display text-display-lg text-surface-900 dark:text-white">
                Resume Match Helper
              </h1>
              <p className="text-surface-500 dark:text-surface-400 mt-1">
                Check how your resume lines up with a job post before you apply
              </p>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-6 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Left Panel - Input */}
          <div className="space-y-6">
            <Card>
              <CardHeader title="Job Post" />
              <label htmlFor="job-description-input" className="sr-only">Job Post</label>
              <textarea
                id="job-description-input"
                value={jobDescription}
                onChange={(e) => setJobDescription(e.target.value)}
                placeholder="Paste the job post here..."
                className="w-full h-64 px-3 py-2 text-sm rounded-lg border border-surface-200 dark:border-surface-600 bg-white dark:bg-surface-700 text-surface-900 dark:text-surface-100 placeholder-surface-400 focus:border-sentinel-500 focus-visible:ring-1 focus-visible:ring-sentinel-500 dark:focus:border-sentinel-400 dark:focus-visible:ring-sentinel-400 resize-none font-mono"
              />
            </Card>

            <Card>
              <ResumeMatchingProfileFields
                disabled={analyzing}
                initialProfile={matchingProfile}
                onProfileChange={(profile) => {
                  setMatchingProfile(profile);
                  setAnalysisResult(null);
                  setAnalysisInputSource(null);
                  setShowComparison(false);
                }}
              />
            </Card>

            <Card>
              <CardHeader title="Resume" />
              <div className="space-y-4">
                <p className="text-sm text-surface-600 dark:text-surface-300">
                  Choose a saved resume or add one. That is the easiest way
                  to compare your resume with a job post.
                </p>
                {activeResume && (
                  <div className="rounded-lg border border-success/30 bg-success/10 px-3 py-2 text-sm text-surface-700 dark:text-surface-200">
                    <p className="break-words [overflow-wrap:anywhere]">
                      <span className="font-medium">Selected resume:</span> {activeResume.name}
                    </p>
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      {activeResume.format_label && (
                        <Badge variant="surface" size="sm">
                          {activeResume.format_label}
                        </Badge>
                      )}
                      <span className="text-xs text-surface-600 dark:text-surface-300">
                        {getSelectedResumeReadableStatus(activeResume)}
                      </span>
                    </div>
                  </div>
                )}
                <div className="flex flex-col sm:flex-row gap-3">
                  <Button
                    onClick={handleChooseResume}
                    className="flex-1"
                  >
                    Choose or Add Resume
                  </Button>
                  <Button
                    type="button"
                    variant="secondary"
                    className="flex-1"
                    aria-expanded={showAdvancedResumeImport}
                    aria-controls="resume-app-import-panel"
                    onClick={() => setShowAdvancedResumeImport((current) => !current)}
                  >
                    Import from Resume App
                  </Button>
                </div>

                {showAdvancedResumeImport && (
                  <div id="resume-app-import-panel" className="pt-4 border-t border-surface-200 dark:border-surface-700">
                    <h2 className="text-sm font-semibold text-surface-900 dark:text-surface-100 mb-3">
                      Import from Resume App
                    </h2>
                    <label htmlFor="resume-json-input" className="sr-only">Copied resume details</label>
                    <textarea
                      id="resume-json-input"
                      value={resumeJson}
                      onChange={(e) => setResumeJson(e.target.value)}
                      placeholder="Paste copied resume details here"
                      aria-describedby="resume-json-hint"
                      className="w-full h-96 px-3 py-2 text-sm rounded-lg border border-surface-200 dark:border-surface-600 bg-white dark:bg-surface-700 text-surface-900 dark:text-surface-100 placeholder-surface-400 focus:border-sentinel-500 focus-visible:ring-1 focus-visible:ring-sentinel-500 dark:focus:border-sentinel-400 dark:focus-visible:ring-sentinel-400 resize-none font-mono"
                    />
                    <p id="resume-json-hint" className="text-xs text-surface-500 dark:text-surface-400 mt-2">
                      Use this only if a resume app gave you details to copy. Most
                      users should choose or add a resume.
                    </p>
                  </div>
                )}
              </div>
            </Card>

            <div className="flex gap-3">
              <Button onClick={handleAnalyze} loading={analyzing} className="flex-1">
                Review Match
              </Button>
              {showAdvancedResumeImport && (
                <Button onClick={handleAnalyzeFormat} loading={analyzing} variant="secondary" className="flex-1">
                  Review Format Only
                </Button>
              )}
            </div>

            {showAdvancedResumeImport && (
              <div className="flex gap-3">
                <p className="text-xs text-surface-500 dark:text-surface-400">
                  Copied resume details stay in JobSentinel for this local review
                  and can be restored during this app session if you add a
                  resume.
                </p>
              </div>
            )}

            <div className="flex gap-3">
              <Button
                onClick={() => {
                  loadActionWords();
                  setShowActionWords(true);
                }}
                variant="ghost"
                size="sm"
                className="flex-1"
              >
                View Action Words
              </Button>
              <Button
                onClick={() => setShowBulletImprover(true)}
                variant="ghost"
                size="sm"
                className="flex-1"
              >
                Draft Alternative Bullet
              </Button>
            </div>
          </div>

          {/* Right Panel - Results */}
          <div className="space-y-6">
            <ResumeMatchResultsPanel
              analyzing={analyzing}
              analysisResult={analysisResult}
              canShowComparison={canShowComparison}
              showComparison={showComparison}
              jobDescription={jobDescription}
              comparisonResumeText={canShowComparison ? resumeJson : ""}
              onToggleComparison={() => setShowComparison((current) => !current)}
              onReviewInResumeBuilder={onNavigate ? handleReviewInResumeBuilder : undefined}
            />
          </div>
        </div>
      </main>

      <ResumeMatchTools
        actionWords={actionWords}
        bulletInput={bulletInput}
        improvedBullet={improvedBullet}
        improvingBullet={improvingBullet}
        showActionWords={showActionWords}
        showBulletImprover={showBulletImprover}
        onBulletInputChange={setBulletInput}
        onCloseActionWords={() => setShowActionWords(false)}
        onCloseBullet={() => {
          setShowBulletImprover(false);
          setBulletInput("");
          setImprovedBullet("");
        }}
        onDraftBullet={handleImproveBullet}
      />
    </div>
  );
}
