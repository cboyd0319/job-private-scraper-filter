import { Badge } from "../../ui/Badge";
import {
  CalendarIcon,
  DownloadIcon,
  HistoryIcon,
  LocationIcon,
  UserIcon,
} from "./InterviewSchedulerIcons";
import {
  OUTCOME_COLORS,
  TYPE_COLORS,
  formatFollowUpSentDate,
  formatInterviewTypeLabel,
  formatOutcomeLabel,
  type FollowUpReminder,
  type Interview,
  type InterviewTab,
} from "./InterviewSchedulerModel";
import { formatInterviewDate, getRelativeTimeUntil } from "../../shared/dateFormatting";

interface InterviewSchedulerTabsProps {
  activeTab: InterviewTab;
  upcomingCount: number;
  pastCount: number;
  onTabChange: (tab: InterviewTab) => void;
}

export function InterviewSchedulerTabs({
  activeTab,
  upcomingCount,
  pastCount,
  onTabChange,
}: InterviewSchedulerTabsProps) {
  return (
    <div className="flex gap-1 p-1 bg-surface-100 dark:bg-surface-700 rounded-lg">
      <button
        onClick={() => onTabChange("upcoming")}
        onKeyDown={(e) => e.key === "Enter" && onTabChange("upcoming")}
        className={`flex-1 px-4 py-2 text-sm font-medium rounded-md transition-colors ${
          activeTab === "upcoming"
            ? "bg-white dark:bg-surface-600 text-surface-900 dark:text-white shadow-sm"
            : "text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white"
        }`}
        aria-label="View open interviews (press Tab to switch)"
      >
        Open ({upcomingCount})
      </button>
      <button
        onClick={() => onTabChange("past")}
        onKeyDown={(e) => e.key === "Enter" && onTabChange("past")}
        className={`flex-1 px-4 py-2 text-sm font-medium rounded-md transition-colors ${
          activeTab === "past"
            ? "bg-white dark:bg-surface-600 text-surface-900 dark:text-white shadow-sm"
            : "text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white"
        }`}
        aria-label="View past interviews (press Tab to switch)"
      >
        Past ({pastCount})
      </button>
    </div>
  );
}

interface InterviewScheduleListProps {
  activeTab: InterviewTab;
  interviews: Interview[];
  pastInterviews: Interview[];
  followUpReminders: Record<number, FollowUpReminder>;
  onSelectInterview: (interview: Interview) => void;
  onExportICal: (interview: Interview) => void;
  onFollowUpToggle: (interviewId: number) => void;
}

export function InterviewScheduleList({
  activeTab,
  interviews,
  pastInterviews,
  followUpReminders,
  onSelectInterview,
  onExportICal,
  onFollowUpToggle,
}: InterviewScheduleListProps) {
  if (activeTab === "upcoming") {
    return (
      <UpcomingInterviewsList
        interviews={interviews}
        onSelectInterview={onSelectInterview}
        onExportICal={onExportICal}
      />
    );
  }

  return (
    <PastInterviewsList
      interviews={pastInterviews}
      followUpReminders={followUpReminders}
      onFollowUpToggle={onFollowUpToggle}
    />
  );
}

interface UpcomingInterviewsListProps {
  interviews: Interview[];
  onSelectInterview: (interview: Interview) => void;
  onExportICal: (interview: Interview) => void;
}

function UpcomingInterviewsList({
  interviews,
  onSelectInterview,
  onExportICal,
}: UpcomingInterviewsListProps) {
  if (interviews.length === 0) {
    return (
      <div className="text-center py-8 text-surface-500 dark:text-surface-400">
        <CalendarIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
        <p>No open interviews</p>
        <p className="text-sm mt-1">Schedule an interview to start preparation.</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {interviews.map((interview) => (
        <div
          key={interview.id}
          className="p-4 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600 hover:border-sentinel-300 dark:hover:border-sentinel-600 transition-colors cursor-pointer"
          onClick={() => onSelectInterview(interview)}
          onKeyDown={(e) => e.key === "Enter" && onSelectInterview(interview)}
          role="button"
          tabIndex={0}
        >
          <div className="flex items-start justify-between">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${TYPE_COLORS[interview.interview_type] || TYPE_COLORS.other}`}>
                  {formatInterviewTypeLabel(interview.interview_type)}
                </span>
                <Badge variant="surface">{getRelativeTimeUntil(interview.scheduled_at)}</Badge>
              </div>
              <h3 className="break-words font-medium text-surface-900 [overflow-wrap:anywhere] dark:text-white">
                {interview.job_title}
              </h3>
              <p className="break-words text-sm text-surface-500 [overflow-wrap:anywhere] dark:text-surface-400">
                {interview.company}
              </p>
            </div>
            <div className="text-right text-sm flex flex-col items-end gap-1">
              <p className="font-medium text-surface-900 dark:text-white">
                {formatInterviewDate(interview.scheduled_at)}
              </p>
              <p className="text-surface-500 dark:text-surface-400">
                {interview.duration_minutes} min
              </p>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onExportICal(interview);
                }}
                className="text-xs text-sentinel-600 dark:text-sentinel-400 hover:underline flex items-center gap-1"
                title="Add to calendar"
              >
                <DownloadIcon />
                Add to calendar
              </button>
            </div>
          </div>
          {(interview.location || interview.interviewer_name) && (
            <div className="mt-2 pt-2 border-t border-surface-200 dark:border-surface-600 flex items-center gap-4 text-sm text-surface-500 dark:text-surface-400">
              {interview.location && (
                <span className="flex items-center gap-1">
                  <LocationIcon />
                  {interview.location}
                </span>
              )}
              {interview.interviewer_name && (
                <span className="flex items-center gap-1">
                  <UserIcon />
                  {interview.interviewer_name}
                  {interview.interviewer_title && ` (${interview.interviewer_title})`}
                </span>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

interface PastInterviewsListProps {
  interviews: Interview[];
  followUpReminders: Record<number, FollowUpReminder>;
  onFollowUpToggle: (interviewId: number) => void;
}

function PastInterviewsList({
  interviews,
  followUpReminders,
  onFollowUpToggle,
}: PastInterviewsListProps) {
  if (interviews.length === 0) {
    return (
      <div className="text-center py-8 text-surface-500 dark:text-surface-400">
        <HistoryIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
        <p>No past interviews yet</p>
        <p className="text-sm mt-1">Completed interviews will appear here</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {interviews.map((interview) => (
        <div
          key={interview.id}
          className="p-4 bg-surface-50 dark:bg-surface-700 rounded-lg border border-surface-200 dark:border-surface-600"
        >
          <div className="flex items-start justify-between">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${TYPE_COLORS[interview.interview_type] || TYPE_COLORS.other}`}>
                  {formatInterviewTypeLabel(interview.interview_type)}
                </span>
                {interview.outcome && (
                  <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${OUTCOME_COLORS[interview.outcome] || OUTCOME_COLORS.pending}`}>
                    {formatOutcomeLabel(interview.outcome)}
                  </span>
                )}
              </div>
              <h3 className="break-words font-medium text-surface-900 [overflow-wrap:anywhere] dark:text-white">
                {interview.job_title}
              </h3>
              <p className="break-words text-sm text-surface-500 [overflow-wrap:anywhere] dark:text-surface-400">
                {interview.company}
              </p>
            </div>
            <div className="text-right text-sm">
              <p className="font-medium text-surface-900 dark:text-white">
                {formatInterviewDate(interview.scheduled_at)}
              </p>
              <p className="text-surface-500 dark:text-surface-400">
                {interview.duration_minutes} min
              </p>
            </div>
          </div>
          {(interview.interviewer_name || interview.post_interview_notes) && (
            <div className="mt-2 pt-2 border-t border-surface-200 dark:border-surface-600 space-y-2">
              {interview.interviewer_name && (
                <span className="flex items-center gap-1 text-sm text-surface-500 dark:text-surface-400">
                  <UserIcon />
                  {interview.interviewer_name}
                  {interview.interviewer_title && ` - ${interview.interviewer_title}`}
                </span>
              )}
              {interview.post_interview_notes && (
                <div className="text-sm">
                  <p className="font-medium text-surface-700 dark:text-surface-300 mb-0.5">Post-interview notes:</p>
                  <p className="whitespace-pre-wrap text-surface-600 dark:text-surface-400">{interview.post_interview_notes}</p>
                </div>
              )}
            </div>
          )}
          <div className="mt-2 pt-2 border-t border-surface-200 dark:border-surface-600">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={followUpReminders[interview.id]?.thankYouSent || false}
                onChange={() => onFollowUpToggle(interview.id)}
                className="w-4 h-4 rounded border-surface-300 dark:border-surface-600 text-sentinel-500 focus-visible:ring-sentinel-500"
              />
              <span className={`text-sm ${followUpReminders[interview.id]?.thankYouSent ? "text-green-600 dark:text-green-400" : "text-surface-600 dark:text-surface-400"}`}>
                {followUpReminders[interview.id]?.thankYouSent ? (
                  <>Thank you note sent</>
                ) : (
                  <>Send thank you note</>
                )}
              </span>
              {followUpReminders[interview.id]?.sentAt && (
                <span className="text-xs text-surface-400 ml-auto">
                  {formatFollowUpSentDate(followUpReminders[interview.id]?.sentAt)}
                </span>
              )}
            </label>
          </div>
        </div>
      ))}
    </div>
  );
}
