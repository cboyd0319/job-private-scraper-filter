import { useEffect, useState, type ClipboardEvent } from "react";
import { Button } from "../../ui/Button";
import { useToast } from "../../shared/toast/useToast";
import { openDeepLink } from "../../shared/search-links";
import {
  getLinkedInWorkbenchReviewStatus,
  recordLinkedInWorkbenchEvent,
  reviewLinkedInWorkbench,
  revokeLinkedInWorkbenchReview,
  type LinkedInWorkbenchReviewStatus,
} from "./internal/linkedinWorkbenchClient";
import { LinkedInWorkbenchLearning } from "./internal/LinkedInWorkbenchLearning";
import {
  clearBrowserAssistLearningSignals,
  readBrowserAssistLearningEnabled,
  readBrowserAssistLearningSignals,
  recordBrowserAssistLearningSignal,
  summarizeBrowserAssistLearningSignals,
  writeBrowserAssistLearningEnabled,
  type BrowserAssistLearningSummary,
} from "../../shared/browserAssistLearning";
import {
  defaultLinkedInWorkbenchPrefill,
  LINKEDIN_WORKBENCH_PRIVACY_REMINDER_MINUTES,
  parseUserProvidedLinkedInText,
  sanitizeLinkedInWorkbenchTextForStorage,
  sanitizeLinkedInWorkbenchUrl,
  shouldShowLinkedInWorkbenchPrivacyReminder,
  type LinkedInWorkbenchEventType,
  type LinkedInWorkbenchPrefill,
} from "./linkedinWorkbenchPolicy";
import { logError } from "../../shared/errorReporting/logger";
const LINKEDIN_JOBS_URL = "https://www.linkedin.com/jobs/";
const LINKEDIN_TRACKER_URL = "https://www.linkedin.com/jobs-tracker/?stage=applied";
export function LinkedInWorkbench() {
  const toast = useToast();
  const [reviewStatus, setReviewStatus] =
    useState<LinkedInWorkbenchReviewStatus | null>(null);
  const [reviewBusy, setReviewBusy] = useState(false);
  const [sessionStartedAt, setSessionStartedAt] = useState<number | null>(null);
  const [now, setNow] = useState<number | null>(null);
  const [draft, setDraft] = useState<LinkedInWorkbenchPrefill>(
    defaultLinkedInWorkbenchPrefill,
  );
  const [pastedText, setPastedText] = useState("");
  const [busyAction, setBusyAction] = useState<LinkedInWorkbenchEventType | "open" | null>(
    null,
  );
  const [learningEnabled, setLearningEnabled] = useState(
    readBrowserAssistLearningEnabled,
  );
  const [learningSummary, setLearningSummary] =
    useState<BrowserAssistLearningSummary>(() =>
      summarizeBrowserAssistLearningSignals(readBrowserAssistLearningSignals()),
    );
  const acknowledged = reviewStatus === "reviewed";
  const policyRefreshRequired = reviewStatus === "policy_refresh_required";
  useEffect(() => {
    let active = true;
    const refreshReviewStatus = () =>
      getLinkedInWorkbenchReviewStatus()
      .then((status) => {
        if (active) setReviewStatus(status);
      })
      .catch((error) => {
        logError("Failed to check LinkedIn Workbench review:", error);
      });
    void refreshReviewStatus();
    setNow(Date.now());
    const timer = window.setInterval(() => {
      setNow(Date.now());
      void refreshReviewStatus();
    }, 60_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);
  const showPrivacyReminder =
    now !== null && shouldShowLinkedInWorkbenchPrivacyReminder(sessionStartedAt, now);
  const updateAcknowledgement = async (accepted: boolean) => {
    setReviewBusy(true);
    try {
      if (accepted) {
        setReviewStatus(
          (await reviewLinkedInWorkbench()) ? "reviewed" : "review_required",
        );
      } else {
        await revokeLinkedInWorkbenchReview();
        setReviewStatus("review_required");
      }
    } catch (error) {
      logError("Failed to update LinkedIn Workbench review:", error);
      toast.error("LinkedIn Workbench review was not changed", "Try again.");
    } finally {
      setReviewBusy(false);
    }
  };
  const handleOpen = async (url: string) => {
    if (!acknowledged) {
      toast.warning("Review LinkedIn workbench first", "Check the box before opening LinkedIn.");
      return;
    }
    setBusyAction("open");
    try {
      await openDeepLink(url);
      setSessionStartedAt(Date.now());
      toast.success("LinkedIn opened", "Use the local buttons here to record what you choose.");
    } catch (error) {
      logError("Failed to open LinkedIn workbench URL:", error);
      toast.error("LinkedIn could not open", "Try opening LinkedIn in your browser.");
    } finally {
      setBusyAction(null);
    }
  };
  const applyPastedTextValue = (value: string) => {
    const parsed = parseUserProvidedLinkedInText(value);
    setDraft((current) => ({
      title: parsed.title || current.title,
      company: parsed.company || current.company,
      url: parsed.url || current.url,
      notes: parsed.notes || current.notes,
    }));
  };
  const applyPastedText = () => {
    applyPastedTextValue(pastedText);
  };
  const handlePasteSelectedText = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const text = event.clipboardData.getData("text");
    if (!text.trim()) {
      return;
    }
    event.preventDefault();
    setPastedText(text);
    applyPastedTextValue(text);
  };
  const recordAction = async (eventType: LinkedInWorkbenchEventType) => {
    if (!acknowledged) {
      toast.warning("Review LinkedIn workbench first", "Check the box before recording work.");
      return;
    }
    setBusyAction(eventType);
    try {
      const result = await recordLinkedInWorkbenchEvent({
        eventType,
        title: cleanOptional(draft.title),
        company: cleanOptional(draft.company),
        url: cleanOptional(sanitizeLinkedInWorkbenchUrl(draft.url)),
        notes: cleanOptional(sanitizeLinkedInWorkbenchTextForStorage(draft.notes)),
      });
      if (learningEnabled) {
        setLearningSummary(
          recordBrowserAssistLearningSignal({
            source: "linkedin-workbench",
            action: eventType,
            title: cleanOptional(draft.title),
            company: cleanOptional(draft.company),
            recordedAt: new Date().toISOString(),
          }),
        );
      }
      toast.success(successTitle(eventType), successMessage(eventType, result.needsDetails));
    } catch (error) {
      logError("Failed to record LinkedIn workbench event:", error);
      toast.error("LinkedIn work was not saved", "Try again or add the job manually.");
    } finally {
      setBusyAction(null);
    }
  };
  const updateLearningEnabled = (enabled: boolean) => {
    setLearningEnabled(enabled);
    writeBrowserAssistLearningEnabled(enabled);
    setLearningSummary(
      summarizeBrowserAssistLearningSignals(readBrowserAssistLearningSignals()),
    );
  };
  const clearLearning = () => {
    setLearningSummary(clearBrowserAssistLearningSignals());
    toast.success("Learning cleared", "Local Workbench suggestions were cleared.");
  };
  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-sentinel-200 bg-sentinel-50 p-4 dark:border-sentinel-800 dark:bg-sentinel-900/20">
        <div className="space-y-2">
          <p className="text-sm font-semibold text-sentinel-900 dark:text-sentinel-100">
            LinkedIn Workbench
          </p>
          <p className="text-sm leading-6 text-sentinel-800 dark:text-sentinel-200">
            Use LinkedIn yourself. JobSentinel keeps a private local record of
            jobs you save, apply to, track, follow up on, interview for, or
            review.
          </p>
          <p className="text-sm leading-6 text-sentinel-800 dark:text-sentinel-200">
            JobSentinel can learn from local job actions you choose, not from
            hidden LinkedIn page watching.
          </p>
          <div className="rounded-lg border border-sentinel-200 bg-white p-3 dark:border-sentinel-800 dark:bg-surface-900">
            <label className="flex items-start gap-3 text-sm font-medium text-sentinel-900 dark:text-sentinel-100">
              <input
                type="checkbox"
                className="mt-0.5 h-5 w-5 rounded border-sentinel-300 text-sentinel-700 focus:ring-sentinel-500"
                checked={learningEnabled}
                onChange={(event) => updateLearningEnabled(event.target.checked)}
              />
              <span>
                Help JobSentinel learn from my local job actions.
              </span>
            </label>
            <p className="mt-2 text-xs leading-5 text-sentinel-700 dark:text-sentinel-200">
              When this is on, JobSentinel saves a small local list of buttons
              you click, saved searches, and the job title and company you
              reviewed. It does not read LinkedIn pages, browser storage,
              cookies, screenshots, hidden fields, or network traffic.
            </p>
          </div>
          <label className="flex items-start gap-3 text-sm font-medium text-sentinel-900 dark:text-sentinel-100">
            <input
              type="checkbox"
              className="mt-0.5 h-5 w-5 rounded border-sentinel-300 text-sentinel-700 focus:ring-sentinel-500"
              checked={acknowledged}
              disabled={reviewBusy || reviewStatus === null || policyRefreshRequired}
              onChange={(event) => void updateAcknowledgement(event.target.checked)}
            />
            <span>
              I understand. Remember this on this computer unless JobSentinel
              changes how this works.
            </span>
          </label>
          {policyRefreshRequired && (
            <p className="text-sm leading-6 text-amber-800 dark:text-amber-200" role="status">
              LinkedIn Workbench is paused while JobSentinel refreshes its LinkedIn
              policy review. Your local records remain available.
            </p>
          )}
        </div>
      </div>
      <div className="rounded-lg border border-surface-200 bg-surface-50 p-4 dark:border-surface-700 dark:bg-surface-800">
        <p className="text-sm font-semibold text-surface-900 dark:text-surface-100">
          Browse normally, then add only the details you choose.
        </p>
        <p className="mt-2 text-sm leading-6 text-surface-700 dark:text-surface-300">
          LinkedIn does not permit third-party page capture. Type or paste
          details you selected yourself, then use these buttons for what you
          did: applied, saved, tracking, interview, follow-up, reminder, note,
          rejected, or not interested.
        </p>
        <p className="mt-2 text-xs leading-5 text-surface-500 dark:text-surface-400">
          JobSentinel does not read the page, browser storage, cookies, hidden
          fields, screenshots, or network traffic.
        </p>
      </div>

      <div className="flex flex-col gap-2 sm:flex-row">
        <Button
          variant="primary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "open"}
          loadingText="Opening..."
          onClick={() => void handleOpen(LINKEDIN_JOBS_URL)}
        >
          Open LinkedIn Jobs
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "open"}
          loadingText="Opening..."
          onClick={() => void handleOpen(LINKEDIN_TRACKER_URL)}
        >
          Open applied jobs
        </Button>
      </div>

      {sessionStartedAt !== null && (
        <p
          className={`rounded-lg border p-3 text-sm ${
            showPrivacyReminder
              ? "border-amber-200 bg-amber-50 text-amber-800 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-200"
              : "border-surface-200 bg-surface-50 text-surface-600 dark:border-surface-700 dark:bg-surface-800 dark:text-surface-300"
          }`}
          aria-live="polite"
        >
          {showPrivacyReminder
            ? "Privacy reminder: close LinkedIn when you are done, or keep going if you are still working."
            : `Privacy reminder appears after ${LINKEDIN_WORKBENCH_PRIVACY_REMINDER_MINUTES} minutes. JobSentinel will not force this session closed.`}
        </p>
      )}

      <LinkedInWorkbenchLearning
        enabled={learningEnabled}
        summary={learningSummary}
        onClear={clearLearning}
      />

      <div className="grid gap-3 sm:grid-cols-3">
        <Button
          variant="success"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "applied"}
          loadingText="Saving..."
          onClick={() => void recordAction("applied")}
        >
          Log applied
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "saved"}
          loadingText="Saving..."
          onClick={() => void recordAction("saved")}
        >
          Save job
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "tracking"}
          loadingText="Saving..."
          onClick={() => void recordAction("tracking")}
        >
          Track job
        </Button>
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "interview"}
          loadingText="Saving..."
          onClick={() => void recordAction("interview")}
        >
          Log interview
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "follow_up"}
          loadingText="Saving..."
          onClick={() => void recordAction("follow_up")}
        >
          Log follow-up
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "reminder"}
          loadingText="Saving..."
          onClick={() => void recordAction("reminder")}
        >
          Add reminder
        </Button>
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <Button
          variant="ghost"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "note"}
          loadingText="Saving..."
          onClick={() => void recordAction("note")}
        >
          Add note
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "rejected"}
          loadingText="Saving..."
          onClick={() => void recordAction("rejected")}
        >
          Log rejected
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={!acknowledged}
          loading={busyAction === "not_interested"}
          loadingText="Saving..."
          onClick={() => void recordAction("not_interested")}
        >
          Not interested
        </Button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-sm">
          <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
            Job title
          </span>
          <input
            className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus-visible:ring-2 focus-visible:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
            value={draft.title}
            onChange={(event) => setDraft({ ...draft, title: event.target.value })}
            placeholder="Optional"
          />
        </label>
        <label className="block text-sm">
          <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
            Company
          </span>
          <input
            className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus-visible:ring-2 focus-visible:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
            value={draft.company}
            onChange={(event) => setDraft({ ...draft, company: event.target.value })}
            placeholder="Optional"
          />
        </label>
      </div>

      <label className="block text-sm">
        <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
          Job link
        </span>
        <input
          className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus-visible:ring-2 focus-visible:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
          value={draft.url}
          onChange={(event) => setDraft({ ...draft, url: event.target.value })}
          placeholder="Optional"
        />
      </label>

      <label className="block text-sm">
        <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
          Paste selected job text
        </span>
        <textarea
          className="min-h-24 w-full resize-y rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus-visible:ring-2 focus-visible:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-700 dark:text-surface-100"
          value={pastedText}
          onChange={(event) => setPastedText(event.target.value)}
          onPaste={handlePasteSelectedText}
          placeholder="Optional"
        />
      </label>

      <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
        <Button
          variant="secondary"
          size="sm"
          disabled={!pastedText.trim()}
          onClick={applyPastedText}
        >
          Use pasted details
        </Button>
        <p className="text-xs leading-5 text-surface-500 dark:text-surface-400">
          Pasted details become suggestions. Review before saving.
        </p>
      </div>
    </div>
  );
}

function cleanOptional(value: string): string | undefined {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function successTitle(eventType: LinkedInWorkbenchEventType): string {
  switch (eventType) {
    case "applied":
      return "Application logged";
    case "saved":
      return "Job saved";
    case "tracking":
      return "Job tracked";
    case "rejected":
      return "Rejection logged";
    case "interview":
      return "Interview logged";
    case "follow_up":
      return "Follow-up logged";
    case "reminder":
      return "Reminder added";
    case "note":
      return "Note saved";
    case "not_interested":
      return "Feedback saved";
  }
}

function successMessage(eventType: LinkedInWorkbenchEventType, needsDetails: boolean): string {
  if (eventType === "applied" && needsDetails) {
    return "Draft created. Add title, company, or link when you have a moment.";
  }
  if (needsDetails) {
    return "Saved as a draft. Add details later.";
  }

  return "Saved locally in JobSentinel.";
}
