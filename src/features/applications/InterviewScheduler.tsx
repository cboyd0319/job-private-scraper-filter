import { useEffect, useState, useCallback, memo } from "react";
import { cachedInvoke, invalidateCacheByCommand, safeInvoke, safeInvokeWithToast } from "../../platform/tauri";
import { Button } from "../../ui/Button";
import {
  InterviewScheduleFormModal,
  type InterviewScheduleFormData,
} from "./InterviewScheduleFormModal";
import { Modal } from "../../ui/Modal";
import { useToast } from "../../shared/toast/useToast";
import { getSafeErrorToastCopy } from "../../shared/errorReporting/safeToastCopy";
import { downloadInterviewICalFile } from "./InterviewCalendarExport";
import {
  InterviewSchedulerTabs,
  InterviewScheduleList,
} from "./InterviewSchedulerLists";
import {
  INTERVIEW_TYPES,
  formatInterviewTypeLabel,
  formatOutcomeLabel,
  type FollowUpReminder,
  type Interview,
  type InterviewSchedulerProps,
  type InterviewTab,
  type PrepChecklistItem,
  type PrepProgress,
} from "./InterviewSchedulerModel";
import {
  PlusIcon,
} from "./InterviewSchedulerIcons";
import { InterviewDetailPanels } from "./InterviewDetailPanels";

const MIN_INTERVIEW_DURATION_MINUTES = 15;
const MAX_INTERVIEW_DURATION_MINUTES = 8 * 60;

export const InterviewScheduler = memo(function InterviewScheduler({
  onClose,
  applications = [],
  renderCompanyResearch,
}: InterviewSchedulerProps) {
  const [interviews, setInterviews] = useState<Interview[]>([]);
  const [pastInterviews, setPastInterviews] = useState<Interview[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [selectedInterview, setSelectedInterview] = useState<Interview | null>(null);
  const [activeTab, setActiveTab] = useState<InterviewTab>('upcoming');
  const [prepProgress, setPrepProgress] = useState<PrepProgress>({});
  const [followUpReminders, setFollowUpReminders] = useState<Record<number, FollowUpReminder>>({});
  const [scheduling, setScheduling] = useState(false);
  const [completing, setCompleting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [dateError, setDateError] = useState<string | null>(null);
  const toast = useToast();

  // Load follow-up reminders from backend
  const loadFollowUpReminders = useCallback(async () => {
    try {
      // Load reminders for all past interviews
      const reminders: Record<number, FollowUpReminder> = {};
      for (const interview of pastInterviews) {
        const result = await safeInvoke<{ thankYouSent: boolean; sentAt: string | null } | null>(
          'get_interview_followup',
          { interviewId: interview.id },
          { logContext: "Load interview follow-up", silent: true }
        );
        if (result) {
          reminders[interview.id] = {
            interviewId: interview.id,
            thankYouSent: result.thankYouSent,
            sentAt: result.sentAt,
          };
        }
      }
      setFollowUpReminders(reminders);
    } catch {
      // Silent failure - non-critical on load
    }
  }, [pastInterviews]);

  // Load follow-up reminders only when the user opens completed history.
  useEffect(() => {
    if (activeTab === 'past' && pastInterviews.length > 0) {
      loadFollowUpReminders();
    }
  }, [activeTab, pastInterviews, loadFollowUpReminders]);

  // Load prep progress when interview is selected
  useEffect(() => {
    const loadPrepProgress = async () => {
      if (!selectedInterview) return;
      try {
        const items = await safeInvoke<PrepChecklistItem[]>(
          'get_interview_prep_checklist',
          { interviewId: selectedInterview.id },
          { logContext: "Load interview prep checklist", silent: true }
        );
        const progress: PrepProgress = {};
        for (const item of items) {
          progress[item.itemId] = item.completed;
        }
        setPrepProgress(progress);
      } catch {
        setPrepProgress({});
      }
    };
    loadPrepProgress();
  }, [selectedInterview]);

  const handlePrepToggle = async (itemId: string) => {
    if (!selectedInterview) return;
    const newCompleted = !prepProgress[itemId];
    // Optimistic update
    setPrepProgress(prev => ({ ...prev, [itemId]: newCompleted }));
    try {
      await safeInvokeWithToast('save_interview_prep_item', {
        interviewId: selectedInterview.id,
        itemId,
        completed: newCompleted,
      }, toast, {
        logContext: "Save interview prep item"
      });
    } catch {
      // Revert on error
      setPrepProgress(prev => ({ ...prev, [itemId]: !newCompleted }));
    }
  };

  const handleFollowUpToggle = async (interviewId: number) => {
    const current = followUpReminders[interviewId];
    const newValue = !(current?.thankYouSent);
    // Optimistic update
    setFollowUpReminders(prev => ({
      ...prev,
      [interviewId]: {
        interviewId,
        thankYouSent: newValue,
        sentAt: newValue ? new Date().toISOString() : null,
      },
    }));
    try {
      await safeInvokeWithToast('save_interview_followup', {
        interviewId,
        thankYouSent: newValue,
      }, toast, {
        logContext: "Save interview follow-up"
      });
      if (newValue) {
        toast.success("Follow-up marked", "Thank you note sent!");
      }
    } catch {
      // Revert on error
      setFollowUpReminders(prev => ({
        ...prev,
        [interviewId]: current || { interviewId, thankYouSent: false, sentAt: null },
      }));
    }
  };

  // Form state
  const [formData, setFormData] = useState<InterviewScheduleFormData>({
    application_id: 0,
    interview_type: "phone",
    scheduled_at: "",
    duration_minutes: 60,
    location: "",
    interviewer_name: "",
    interviewer_title: "",
    notes: "",
  });

  const fetchInterviews = useCallback(async () => {
    try {
      setLoading(true);
      const [upcomingData, pastData] = await Promise.all([
        cachedInvoke<Interview[]>("get_upcoming_interviews", undefined, 30_000),
        cachedInvoke<Interview[]>("get_past_interviews", undefined, 30_000).catch(() => [] as Interview[]),
      ]);
      setInterviews(upcomingData);
      setPastInterviews(pastData);
    } catch (err: unknown) {
      const safeError = getSafeErrorToastCopy(err, {
        fallbackTitle: "Could not load interviews",
      });
      toast.error(safeError.title, safeError.message);
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchInterviews();
  }, [fetchInterviews]);

  // Keyboard shortcuts: Tab to switch tabs, R to refresh, Cmd+N to add new
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Tab key to switch between upcoming/past
      if (e.key === 'Tab' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
        const target = e.target as HTMLElement;
        // Only switch tabs if not focused on an input/button
        if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA' && target.tagName !== 'BUTTON') {
          e.preventDefault();
          setActiveTab(prev => prev === 'upcoming' ? 'past' : 'upcoming');
        }
      }
      // R to refresh
      if (e.key === 'r' && !e.ctrlKey && !e.metaKey) {
        const target = e.target as HTMLElement;
        if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
          e.preventDefault();
          fetchInterviews();
        }
      }
      // Cmd+N to schedule new interview
      if (e.key === 'n' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setShowAddForm(true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [fetchInterviews]);

  const handleScheduleInterview = async () => {
    if (!formData.application_id || !formData.scheduled_at) {
      toast.error("Choose interview details", "Choose an application and time before scheduling.");
      return;
    }

    // Validate date is not in the past
    const scheduledDate = new Date(formData.scheduled_at);
    const now = new Date();
    if (scheduledDate < now) {
      toast.error("Choose a future time", "Pick a time that has not passed.");
      return;
    }

    // Validate duration is reasonable
    if (formData.duration_minutes < MIN_INTERVIEW_DURATION_MINUTES || formData.duration_minutes > MAX_INTERVIEW_DURATION_MINUTES) {
      toast.error("Choose a valid length", `Pick between ${MIN_INTERVIEW_DURATION_MINUTES} minutes and ${MAX_INTERVIEW_DURATION_MINUTES / 60} hours.`);
      return;
    }

    try {
      setScheduling(true);
      await safeInvokeWithToast("schedule_interview", {
        applicationId: formData.application_id,
        interviewType: formData.interview_type,
        scheduledAt: formData.scheduled_at,
        durationMinutes: formData.duration_minutes,
        location: formData.location || null,
        interviewerName: formData.interviewer_name || null,
        interviewerTitle: formData.interviewer_title || null,
        notes: formData.notes || null,
      }, toast, {
        logContext: "Schedule interview"
      });
      invalidateCacheByCommand("get_upcoming_interviews");
      toast.success("Interview scheduled", "Your interview has been added to the calendar");
      setShowAddForm(false);
      setFormData({
        application_id: 0,
        interview_type: "phone",
        scheduled_at: "",
        duration_minutes: 60,
        location: "",
        interviewer_name: "",
        interviewer_title: "",
        notes: "",
      });
      fetchInterviews();
    } catch {
      // Error already logged and shown to user
    } finally {
      setScheduling(false);
    }
  };

  const handleCompleteInterview = async (interview: Interview, outcome: string, postNotes?: string) => {
    try {
      setCompleting(true);
      await safeInvokeWithToast("complete_interview", {
        interviewId: interview.id,
        outcome,
        notes: postNotes || null,
      }, toast, {
        logContext: "Complete interview"
      });
      invalidateCacheByCommand("get_upcoming_interviews");
      invalidateCacheByCommand("get_past_interviews");
      toast.success("Interview completed", formatOutcomeLabel(outcome));
      setSelectedInterview(null);
      fetchInterviews();
    } catch {
      // Error already logged and shown to user
    } finally {
      setCompleting(false);
    }
  };

  const handleExportICal = (interview: Interview) => {
    downloadInterviewICalFile(
      interview,
      formatInterviewTypeLabel(interview.interview_type),
    );
    toast.success("Calendar downloaded", "Add to your calendar app");
  };

  const handleDeleteInterview = async (interviewId: number) => {
    try {
      setDeleting(true);
      await safeInvokeWithToast("delete_interview", { interviewId }, toast, {
        logContext: "Delete interview"
      });
      invalidateCacheByCommand("get_upcoming_interviews");
      toast.success("Interview deleted", "The interview has been removed");
      setSelectedInterview(null);
      fetchInterviews();
    } catch {
      // Error already logged and shown to user
    } finally {
      setDeleting(false);
    }
  };


  if (loading) {
    return (
      <Modal
        isOpen
        onClose={onClose}
        title="Interview Schedule"
        description="Loading interviews"
        size="wide"
        closeButtonLabel="Close interview scheduler"
      >
        <div className="space-y-4" role="status" aria-label="Loading interviews">
          <div className="h-8 w-48 rounded bg-surface-200 motion-safe:animate-pulse dark:bg-surface-700" />
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-24 rounded bg-surface-200 motion-safe:animate-pulse dark:bg-surface-700" />
          ))}
        </div>
      </Modal>
    );
  }

  return (
    <>
      <Modal
        isOpen
        onClose={onClose}
        title="Interview Schedule"
        size="wide"
        closeButtonLabel="Close interview scheduler"
      >
        <div className="space-y-6">
          <div className="flex justify-end">
            <Button variant="primary" onClick={() => setShowAddForm(true)}>
              <PlusIcon />
              Schedule
            </Button>
          </div>

          <InterviewSchedulerTabs
            activeTab={activeTab}
            upcomingCount={interviews.length}
            pastCount={pastInterviews.length}
            onTabChange={setActiveTab}
          />

          <InterviewScheduleList
            activeTab={activeTab}
            interviews={interviews}
            pastInterviews={pastInterviews}
            followUpReminders={followUpReminders}
            onSelectInterview={setSelectedInterview}
            onExportICal={handleExportICal}
            onFollowUpToggle={handleFollowUpToggle}
          />
        </div>
      </Modal>

      {showAddForm && (
        <InterviewScheduleFormModal
          applications={applications}
          dateError={dateError}
          formData={formData}
          interviewTypes={INTERVIEW_TYPES}
          scheduling={scheduling}
          onClose={() => setShowAddForm(false)}
          onDateErrorChange={setDateError}
          onFormDataChange={setFormData}
          onSchedule={handleScheduleInterview}
        />
      )}

      {selectedInterview && (
        <InterviewDetailPanels
          completing={completing}
          deleting={deleting}
          interview={selectedInterview}
          onClose={() => setSelectedInterview(null)}
          onComplete={handleCompleteInterview}
          onDelete={handleDeleteInterview}
          onExportICal={handleExportICal}
          onPrepToggle={handlePrepToggle}
          prepProgress={prepProgress}
          renderCompanyResearch={renderCompanyResearch}
        />
      )}
    </>
  );
});
