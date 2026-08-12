import { memo, useState, useEffect, useCallback } from "react";
// memo applied to ApplyButton and application-form badges
import { Button } from "../../ui/Button";
import { Modal, ModalFooter } from "../../ui/Modal";
import { useToast } from "../../shared/toast/useToast";
import { logError } from "../../shared/errorReporting/logger";
import { invoke, safeInvoke, safeInvokeWithToast } from "../../platform/tauri";
import { getUserFriendlyError } from "../../shared/errorReporting/messages";
import { ApplicationPreview } from "./ApplicationPreview";
import { getApplicationFormDisplayName } from "./applicationFormLabels";
import { readStorageValue, removeStorageValue, writeStorageValue } from "../../shared/browserStorage";

interface Job {
  id: number;
  hash: string;
  title: string;
  company: string;
  location: string;
  url: string;
  description?: string;
  score?: number;
}

interface AtsDetectionResponse {
  platform: string;
  commonFields: string[];
  automationNotes: string | null;
}

interface ApplyButtonProps {
  job: Job;
  onApplied?: () => void;
  onOpenApplicationAssist?: () => void;
}

const APPLICATION_PLATFORM_HELP = "Recognized application form";

function getSafeFormPreparationError(error: unknown) {
  const friendly = getUserFriendlyError(error);
  return {
    title: friendly.title,
    message: friendly.message,
    action: friendly.action,
  };
}

function formatScreeningTopicList(topics: string[]) {
  const uniqueTopics = topics
    .map((topic) => topic.trim())
    .filter(Boolean)
    .filter((topic, index, allTopics) => allTopics.indexOf(topic) === index);

  if (uniqueTopics.length === 0) {
    return "";
  }

  if (uniqueTopics.length === 1) {
    return uniqueTopics[0];
  }

  if (uniqueTopics.length === 2) {
    return `${uniqueTopics[0]} and ${uniqueTopics[1]}`;
  }

  return `${uniqueTopics.slice(0, -1).join(", ")}, and ${
    uniqueTopics[uniqueTopics.length - 1]
  }`;
}

const APPLICATION_FORM_COLORS: Record<string, string> = {
  greenhouse: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  lever: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  workday: "bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300",
  taleo: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  icims: "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  bamboohr: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300",
  ashbyhq: "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300",
  smartrecruiters: "bg-sky-100 text-sky-800 dark:bg-sky-900/30 dark:text-sky-300",
  workable: "bg-teal-100 text-teal-800 dark:bg-teal-900/30 dark:text-teal-300",
  recruitee: "bg-lime-100 text-lime-800 dark:bg-lime-900/30 dark:text-lime-300",
  breezyhr: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300",
  jazzhr: "bg-violet-100 text-violet-800 dark:bg-violet-900/30 dark:text-violet-300",
  bullhorn: "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
  jobvite: "bg-rose-100 text-rose-800 dark:bg-rose-900/30 dark:text-rose-300",
  teamtailor: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  successfactors: "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  oracle_recruiting: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300",
  eightfold: "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300",
  unknown: "bg-surface-100 text-surface-600 dark:bg-surface-700 dark:text-surface-300",
};

export const ApplyButton = memo(function ApplyButton({ job, onApplied, onOpenApplicationAssist }: ApplyButtonProps) {
  const [atsPlatform, setAtsPlatform] = useState<string | null>(null);
  const [atsLoading, setAtsLoading] = useState(true);
  const [hasProfile, setHasProfile] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [isFilling, setIsFilling] = useState(false);
  const [browserRunning, setBrowserRunning] = useState(false);
  const [lastAttemptId, setLastAttemptId] = useState<number | null>(null);
  const [showSubmitConfirm, setShowSubmitConfirm] = useState(false);
  const [fillError, setFillError] = useState<string | null>(null);
  const toast = useToast();

  // Check for previous attempt on mount
  useEffect(() => {
    const stored = readStorageValue("local", `lastAttempt_${job.hash}`);
    if (stored) {
      const attemptId = parseInt(stored, 10);
      if (!isNaN(attemptId)) {
        setLastAttemptId(attemptId);
      }
    }
  }, [job.hash]);

  // Detect recognized application form from URL
  const detectPlatform = useCallback(async () => {
    try {
      setAtsLoading(true);
      const result = await safeInvoke<AtsDetectionResponse>("detect_ats_platform", {
        url: job.url,
      }, { silent: true }); // Silent mode - form detection is optional
      setAtsPlatform(result.platform);
    } catch {
      // Silently fail - form detection is optional
    } finally {
      setAtsLoading(false);
    }
  }, [job.url]);

  // Check if user has configured a profile
  const checkProfile = useCallback(async () => {
    try {
      const profileExists = await safeInvoke<boolean>("has_application_profile", undefined, {
        silent: true,
      });
      setHasProfile(profileExists);
    } catch {
      // Silently fail - will show error when user tries to apply
    }
  }, []);

  // Check if browser is running
  const checkBrowser = useCallback(async () => {
    try {
      const running = await safeInvoke<boolean>("is_browser_running", undefined, { silent: true });
      setBrowserRunning(running);
    } catch {
      // Silently fail - default to false
    }
  }, []);

  useEffect(() => {
    detectPlatform();
    checkProfile();
    checkBrowser();
  }, [detectPlatform, checkProfile, checkBrowser]);

  const handleOpenApplicationAssist = () => {
    if (onOpenApplicationAssist) {
      onOpenApplicationAssist();
      return;
    }

    toast.info(
      "Set up profile first",
      "Open Application Assist from the sidebar and save the details you want JobSentinel to prepare."
    );
  };

  const handlePrepareApplication = () => {
    if (!hasProfile) {
      toast.error(
        "Set up profile first",
        "Open Application Assist from the sidebar and save the details you want JobSentinel to prepare."
      );
      return;
    }
    setShowPreview(true);
  };

  const handleFillForm = async () => {
    try {
      setIsFilling(true);
      setFillError(null);
      toast.info("Opening browser...", "Review-ready form help will start shortly");

      const result = await invoke<{
        filledFields: string[];
        unfilledFields: string[];
        captchaDetected: boolean;
        readyForReview: boolean;
        errorMessage: string | null;
        screeningAnswerTopics?: string[];
        manualReviewTopics?: string[];
        attemptId: number | null;
        durationMs: number;
        atsPlatform: string;
      }>("fill_application_form", { jobUrl: job.url, jobHash: job.hash });

      // Count screening questions filled
      const screeningCount = result.filledFields.filter(f => f.startsWith("screening:")).length;
      const basicCount = result.filledFields.length - screeningCount;

      if (result.captchaDetected) {
        toast.warning(
          "Site asked for a human check",
          "Complete the check in the browser, then continue review"
        );
      } else if (result.errorMessage) {
        const safeError = getSafeFormPreparationError(result.errorMessage);
        const actionHint = safeError.action ? `\n\n${safeError.action}` : "";
        // Keep modal open with error for retry
        setFillError(safeError.message + actionHint);
        toast.error("Could not prepare details", safeError.message + actionHint);
        return; // Don't close modal
      } else {
        const unfilled = result.unfilledFields.length;
        let message = `Prepared ${basicCount} profile fields`;
        if (screeningCount > 0) {
          message += ` and ${screeningCount} saved screening answers`;
        }
        const screeningTopicList = formatScreeningTopicList(
          result.screeningAnswerTopics ?? [],
        );
        if (screeningTopicList) {
          message += `. Check saved answers for ${screeningTopicList}`;
        }
        if (
          result.manualReviewTopics?.includes(
            "voluntary or sensitive personal questions",
          )
        ) {
          message += ". Voluntary or sensitive personal questions were left untouched";
        }
        if (unfilled > 0) {
          message += `. ${unfilled} fields need attention`;
        }
        message += `. Review every field and submit it yourself.`;

        toast.success("Form ready for review", message);
      }

      setShowPreview(false);
      setBrowserRunning(true);

      // Store attempt ID for later tracking
      if (result.attemptId) {
        writeStorageValue("local", `lastAttempt_${job.hash}`, result.attemptId.toString());
      }

      onApplied?.();
    } catch (error: unknown) {
      // Check if browser is still running after error for recovery guidance
      let stillRunning = false;
      try {
        stillRunning = await safeInvoke<boolean>("is_browser_running", undefined, { silent: true });
        setBrowserRunning(stillRunning);
      } catch {
        // Ignore check failure
      }

      const safeError = getSafeFormPreparationError(error);
      const errorMsg = safeError.message;
      const recoveryHint = stillRunning ? " Browser is still open." : "";
      const actionHint = safeError.action ? `\n\n${safeError.action}` : "";

      // Keep modal open with error for retry
      setFillError(errorMsg + recoveryHint + actionHint);
      logError("Failed to prepare form:", error);
      toast.error(
        safeError.title || "Could not prepare details",
        errorMsg + (recoveryHint || actionHint ? recoveryHint + actionHint : "")
      );
    } finally {
      setIsFilling(false);
    }
  };

  const closeBrowser = async () => {
    try {
      await safeInvokeWithToast("close_automation_browser", undefined, toast, {
        logContext: "Close application review browser",
      });
      setBrowserRunning(false);

      // If we have a pending attempt, ask if they submitted
      if (lastAttemptId) {
        setShowSubmitConfirm(true);
      } else {
        toast.info("Browser closed", "The browser window has been closed");
      }
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleMarkSubmitted = async () => {
    if (!lastAttemptId) return;

    try {
      await safeInvokeWithToast("mark_attempt_submitted", { attemptId: lastAttemptId }, toast, {
        logContext: "Mark application submitted",
      });
      toast.success("Marked as submitted", "Your application has been tracked");
      removeStorageValue("local", `lastAttempt_${job.hash}`);
      setLastAttemptId(null);
      setShowSubmitConfirm(false);
      onApplied?.();
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleSkipTracking = () => {
    removeStorageValue("local", `lastAttempt_${job.hash}`);
    setLastAttemptId(null);
    setShowSubmitConfirm(false);
    toast.info("Browser closed", "Not added to your board");
  };

  const applicationFormName = getApplicationFormDisplayName(atsPlatform);

  return (
    <>
      <div className="flex items-center gap-2">
        {/* Application form badge */}
        {atsLoading ? (
          <span className="w-16 h-6 bg-surface-200 dark:bg-surface-700 rounded-full animate-pulse" />
        ) : applicationFormName ? (
          <span
            className={`px-2 py-1 text-xs font-medium rounded-full ${(atsPlatform && APPLICATION_FORM_COLORS[atsPlatform]) || APPLICATION_FORM_COLORS.unknown}`}
            title={APPLICATION_PLATFORM_HELP}
          >
            {applicationFormName}
          </span>
        ) : null}

        {/* Main Action Button */}
        {browserRunning ? (
          <Button variant="secondary" onClick={closeBrowser} size="sm">
            <XIcon className="w-4 h-4 mr-1" />
            Close Browser
          </Button>
        ) : (
          <Button
            onClick={handlePrepareApplication}
            disabled={!hasProfile}
            size="sm"
            data-testid="btn-apply"
            title={
              !hasProfile
                ? "Save your application profile in Application Assist"
                : atsLoading
                  ? "Form check is still running. You can prepare details now."
                  : "Prepare application form for your review"
            }
          >
            <BoltIcon className="w-4 h-4 mr-1" />
            Prepare Form
          </Button>
        )}
        {!browserRunning && !atsLoading && !hasProfile && (
          <Button
            variant="secondary"
            size="sm"
            onClick={handleOpenApplicationAssist}
            title="Open Application Assist to set up your profile"
          >
            Set Up Profile
          </Button>
        )}
      </div>

      {/* Preview Modal */}
      <Modal
        isOpen={showPreview}
        onClose={() => { setShowPreview(false); setFillError(null); }}
        title="Review Application"
        size="lg"
      >
        <ApplicationPreview job={job} atsPlatform={atsPlatform} />

        {/* Error state with retry */}
        {fillError && (
          <div className="mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 text-red-500">
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
              </div>
              <div className="flex-1">
                <h4 className="text-sm font-medium text-red-800 dark:text-red-200">
                  Could not prepare details
                </h4>
                <p className="mt-1 text-sm text-red-700 dark:text-red-300">
                  {fillError}
                </p>
              </div>
            </div>
          </div>
        )}

        <ModalFooter>
          <Button variant="secondary" onClick={() => { setShowPreview(false); setFillError(null); }}>
            Cancel
          </Button>
          <Button onClick={handleFillForm} loading={isFilling} loadingText="Preparing...">
            <BoltIcon className="w-4 h-4 mr-2" />
            {fillError ? "Try Again" : "Prepare Details"}
          </Button>
        </ModalFooter>
      </Modal>

      {/* Submit Confirmation Modal */}
      <Modal
        isOpen={showSubmitConfirm}
        onClose={handleSkipTracking}
        title="Did you submit the application?"
        size="sm"
      >
        <div className="py-4">
          <p className="text-surface-700 dark:text-surface-300 mb-4">
            Did you personally submit the application form?
          </p>
          <p className="text-sm text-surface-500 dark:text-surface-400">
            This helps track your application status.
          </p>
        </div>
        <ModalFooter>
          <Button variant="secondary" onClick={handleSkipTracking}>
            No, skip
          </Button>
          <Button onClick={handleMarkSubmitted}>
            <CheckIcon className="w-4 h-4 mr-2" />
            Yes, I submitted it
          </Button>
        </ModalFooter>
      </Modal>
    </>
  );
});

// Standalone application-form badge for use in job cards
export const AtsBadge = memo(function AtsBadge({ url }: { url: string }) {
  const [platform, setPlatform] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    invoke<AtsDetectionResponse>("detect_ats_platform", { url })
      .then((result) => setPlatform(result.platform))
      .catch((err) => {
        logError("Application form detection failed for badge:", err);
        setError(true);
      });
  }, [url]);

  // Don't show badge on error or unknown platform
  const applicationFormName = getApplicationFormDisplayName(platform);
  if (error || !applicationFormName) return null;

  // Use inline styles since Badge doesn't support className
  return (
    <span
      className={`px-2 py-1 text-xs font-medium rounded-full ${(platform && APPLICATION_FORM_COLORS[platform]) || APPLICATION_FORM_COLORS.unknown}`}
    >
      {applicationFormName}
    </span>
  );
});

// Icons
function BoltIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function XIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function CheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
    </svg>
  );
}
