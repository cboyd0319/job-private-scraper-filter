import { useEffect, useState, useCallback, useId, useMemo, useRef } from "react";
import {
  cachedInvoke,
  invalidateCacheByCommand,
  invoke,
  safeInvokeWithToast,
} from "../../platform/tauri";
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragStartEvent,
  DragEndEvent,
  DragOverEvent,
} from "@dnd-kit/core";
import { sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { useToast } from "../../shared/toast/useToast";
import { useUndo } from "../../shared/undo/useUndo";
import { logError } from "../../shared/errorReporting/logger";
import { formatEventDate } from "../../shared/dateFormatting";
import {
  findApplicationById,
  findColumnForApplication,
  getApplicationReviewSummary,
  formatReminderType,
  getApplicationStats,
  hasAnyApplications,
  STATUS_COLUMNS,
  type Application,
  type ApplicationReviewAction,
  type ApplicationsByStatus,
  type PendingReminder,
  type StatusKey,
} from "./applicationsModel";
import { APPLICATION_DRAG_COLLISION_DETECTION } from "./applicationsDnd";
import {
  DroppableColumn,
  KanbanSkeleton,
} from "./ApplicationsBoard";
import { ApplicationsReviewPanel } from "./ApplicationsReviewPanel";
import { ApplicationsOverlays } from "./ApplicationsOverlays";
import { ApplicationsHeader } from "./ApplicationsHeader";
import type { RenderCompanyResearch } from "../../shared/companyResearch";

interface ApplicationsProps {
  onBack: () => void;
  onImportJob?: () => void;
  onOpenSalary?: () => void;
  onOpenSources?: () => void;
  renderCompanyResearch?: RenderCompanyResearch;
}

export default function ApplicationsPage({
  onBack,
  onImportJob,
  onOpenSalary,
  onOpenSources,
  renderCompanyResearch,
}: ApplicationsProps) {
  const [applications, setApplications] = useState<ApplicationsByStatus | null>(null);
  const [reminders, setReminders] = useState<PendingReminder[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedApp, setSelectedApp] = useState<Application | null>(null);
  const [notes, setNotes] = useState("");
  const [activeId, setActiveId] = useState<number | null>(null);
  const [showAnalytics, setShowAnalytics] = useState(false);
  const [showInterviews, setShowInterviews] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const dragStartColumnRef = useRef<StatusKey | null>(null);
  const toast = useToast();
  const { pushAction } = useUndo();

  // Accessibility IDs (SSR-safe)
  const appStatusId = useId();
  const appNotesId = useId();

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8, // 8px movement required before drag starts
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      // Use cached invoke with short TTL (5s) - data changes frequently via drag/drop
      const [appsData, remindersData] = await Promise.all([
        cachedInvoke<ApplicationsByStatus>("get_applications_kanban", undefined, 5_000),
        cachedInvoke<PendingReminder[]>("get_pending_reminders", undefined, 10_000),
      ]);
      setApplications(appsData);
      setReminders(remindersData);
    } catch (err: unknown) {
      logError("Failed to fetch applications:", err);
      toast.error(
        "Could not load saved applications",
        "Save a safe support report if this keeps happening, then close and reopen JobSentinel."
      );
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const findColumnForApp = (appId: number): StatusKey | null => {
    return findColumnForApplication(applications, appId);
  };

  const openApplicationDetail = (app: Application) => {
    setSelectedApp(app);
    setNotes(app.notes ?? "");
  };

  const closeApplicationDetail = () => {
    setSelectedApp(null);
    setNotes("");
  };

  const handleDragStart = (event: DragStartEvent) => {
    const appId = event.active.id as number;
    setActiveId(appId);
    dragStartColumnRef.current = findColumnForApp(appId);
  };

  const handleDragOver = (event: DragOverEvent) => {
    const { active, over } = event;
    if (!over || !applications) return;

    const activeId = active.id as number;
    const overId = over.id;

    const activeColumn = findColumnForApp(activeId);

    // Check if we're over a column (the column has the status key as id)
    const overColumn = STATUS_COLUMNS.find((c) => c.key === overId)?.key || findColumnForApp(overId as number);

    if (!activeColumn || !overColumn || activeColumn === overColumn) return;

    // Move the item to the new column optimistically
    setApplications((prev) => {
      if (!prev) return prev;

      const activeApp = prev[activeColumn].find((a) => a.id === activeId);
      if (!activeApp) return prev;

      return {
        ...prev,
        [activeColumn]: prev[activeColumn].filter((a) => a.id !== activeId),
        [overColumn]: [...prev[overColumn], { ...activeApp, status: overColumn }],
      };
    });
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveId(null);
    const oldColumn = dragStartColumnRef.current;
    dragStartColumnRef.current = null;

    if (!over || !applications) return;

    const activeId = active.id as number;
    const newColumn = findColumnForApp(activeId);

    if (!newColumn || !oldColumn) return;
    if (newColumn === oldColumn) return;

    const app = STATUS_COLUMNS
      .flatMap((column) => applications[column.key])
      .find((candidate) => candidate.id === activeId);
    if (!app) return;

    // Persist the change to backend
    try {
      await safeInvokeWithToast("update_application_status", { applicationId: activeId, status: newColumn }, toast, {
        logContext: "Update application status",
      });
      // Invalidate cache after mutation
      invalidateCacheByCommand("get_applications_kanban");
      const newLabel = STATUS_COLUMNS.find((c) => c.key === newColumn)?.label;

      // Push undoable action
      pushAction({
        type: "status",
        description: `Moved ${app.job_title} to ${newLabel}`,
        undo: async () => {
          await invoke("update_application_status", { applicationId: activeId, status: oldColumn });
          invalidateCacheByCommand("get_applications_kanban");
          fetchData();
        },
        redo: async () => {
          await invoke("update_application_status", { applicationId: activeId, status: newColumn });
          invalidateCacheByCommand("get_applications_kanban");
          fetchData();
        },
      });
    } catch {
      // Error already logged and shown to user via safeInvokeWithToast
      // Revert by refetching
      invalidateCacheByCommand("get_applications_kanban");
      fetchData();
    }
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!selectedApp) return;

    try {
      await invoke("update_application_status", { applicationId: selectedApp.id, status: newStatus });
      invalidateCacheByCommand("get_applications_kanban");
      const newLabel = STATUS_COLUMNS.find((column) => column.key === newStatus)?.label ?? newStatus;
      toast.success("Status updated", `Application moved to ${newLabel}`);
      setSelectedApp({ ...selectedApp, status: newStatus });
      fetchData();
    } catch (error: unknown) {
      logError("Failed to update status:", error);
      toast.error(
        "Could not update status",
        "The application status wasn't changed. Try again, or copy a safe support report before closing and reopening JobSentinel.",
      );
    }
  };

  const handleAddNotes = async () => {
    if (!selectedApp || !notes.trim()) return;
    const appId = selectedApp.id;
    const appTitle = selectedApp.job_title;
    const previousNotes = selectedApp.notes;
    const newNotes = notes;

    try {
      await safeInvokeWithToast("add_application_notes", { applicationId: appId, notes: newNotes }, toast, {
        logContext: "Add application notes",
      });
      // Invalidate cache after mutation
      invalidateCacheByCommand("get_applications_kanban");
      closeApplicationDetail();
      fetchData();

      // Push undoable action
      pushAction({
        type: "notes",
        description: `Updated notes for ${appTitle}`,
        undo: async () => {
          await invoke("add_application_notes", { applicationId: appId, notes: previousNotes });
          invalidateCacheByCommand("get_applications_kanban");
          fetchData();
        },
        redo: async () => {
          await invoke("add_application_notes", { applicationId: appId, notes: newNotes });
          invalidateCacheByCommand("get_applications_kanban");
          fetchData();
        },
      });
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleCompleteReminder = async (reminderId: number) => {
    try {
      await safeInvokeWithToast("complete_reminder", { reminderId }, toast, {
        logContext: "Complete reminder",
      });
      // Invalidate cache after mutation
      invalidateCacheByCommand("get_pending_reminders");
      toast.success("Reminder completed", "Marked as done");
      fetchData();
    } catch {
      // Error already logged and shown to user
    }
  };

  const handleMissionAction = (action: ApplicationReviewAction) => {
    switch (action.kind) {
      case "reminders":
        document
          .querySelector<HTMLButtonElement>(`[data-reminder-id="${action.reminderId}"] button`)
          ?.focus();
        return;
      case "no_response":
      case "to_apply": {
        const application = findApplicationById(applications, action.applicationId ?? null);
        if (application) {
          openApplicationDetail(application);
        } else {
          (onImportJob ?? onBack)();
        }
        return;
      }
      case "interviews":
        setShowInterviews(true);
        return;
      case "offers":
        if (onOpenSalary) {
          onOpenSalary();
        } else {
          setShowAnalytics(true);
        }
        return;
      case "weekly_review":
      case "steady":
        setShowAnalytics(true);
        return;
      case "source_review":
        (onOpenSources ?? onBack)();
    }
  };

  const getActiveApp = (): Application | null =>
    findApplicationById(applications, activeId);

  // Memoized application stats to avoid recalculating on every render
  const stats = useMemo(() => getApplicationStats(applications), [applications]);

  const hasTrackedApplications = useMemo(
    () => hasAnyApplications(applications),
    [applications],
  );

  if (loading) {
    return <KanbanSkeleton />;
  }

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      {/* Hidden instructions for screen readers during drag */}
      <div id="drag-instructions" className="sr-only" aria-live="polite">
        Use arrow keys to move between columns. Press space or enter to drop.
      </div>

      {/* Live region for drag announcements */}
      <div className="sr-only" aria-live="assertive" aria-atomic="true">
        {activeId && getActiveApp() && (
          `Moving ${getActiveApp()?.job_title} at ${getActiveApp()?.company}`
        )}
      </div>

      <ApplicationsHeader
        onBack={onBack}
        onOpenInterviews={() => setShowInterviews(true)}
        onOpenSummary={() => setShowAnalytics(true)}
        onOpenTemplates={() => setShowTemplates(true)}
        stats={stats}
      />

      <main className="p-6">
        <ApplicationsReviewPanel
          summary={getApplicationReviewSummary(applications, reminders)}
          onSelectAction={handleMissionAction}
          onOpenSummary={() => setShowAnalytics(true)}
          onGoToJobs={onBack}
          onImportJob={onImportJob}
        />

        {/* Reminders */}
        {reminders.length > 0 && (
          <Card className="mb-6 dark:bg-surface-800" data-testid="pending-reminders">
            <h2 className="font-display text-display-sm text-surface-900 dark:text-white mb-4">
              Pending Reminders ({reminders.length})
            </h2>
            <div className="space-y-2">
              {reminders.map((reminder) => (
                <div
                  key={reminder.id}
                  data-testid="pending-reminder"
                  data-reminder-id={reminder.id}
                  className="flex items-center justify-between p-3 bg-alert-50 dark:bg-alert-900/20 rounded-lg"
                >
                  <div>
                    <p className="font-medium text-surface-800 dark:text-surface-200">
                      {reminder.job_title} at {reminder.company}
                    </p>
                    <p className="text-sm text-surface-500 dark:text-surface-400">
                      {formatReminderType(reminder.reminder_type)} due: {formatEventDate(reminder.reminder_time)}
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handleCompleteReminder(reminder.id)}
                  >
                    Done
                  </Button>
                </div>
              ))}
            </div>
          </Card>
        )}

        {/* Drag and Drop Kanban Board */}
        <DndContext
          sensors={sensors}
          collisionDetection={APPLICATION_DRAG_COLLISION_DETECTION}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDragEnd={handleDragEnd}
        >
          <div className="pb-4" data-testid="kanban-board">
            {!hasTrackedApplications && (
              <Card
                className="mb-4 max-w-xl dark:bg-surface-800"
                role="status"
                aria-live="polite"
              >
                <h2 className="font-display text-display-sm text-surface-900 dark:text-white mb-2">
                  No applications tracked yet
                </h2>
                <p className="text-sm text-surface-600 dark:text-surface-400 mb-4">
                  Save or import a job to start tracking it here.
                </p>
                <div className="flex flex-wrap gap-3">
                  <Button onClick={onBack}>Go to Jobs</Button>
                  {onImportJob && (
                    <Button onClick={onImportJob} variant="secondary">
                      Import Job
                    </Button>
                  )}
                </div>
              </Card>
            )}
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
              {STATUS_COLUMNS.map((column) => {
                const apps = applications?.[column.key] || [];
                return (
                  <DroppableColumn
                    key={column.key}
                    column={column}
                    apps={apps}
                    onCardClick={openApplicationDetail}
                    formatDate={formatEventDate}
                    showDropHint={hasTrackedApplications}
                  />
                );
              })}
            </div>
          </div>

          {/* Drag Overlay - shows the card being dragged */}
          <DragOverlay>
            {activeId ? (
              <div className="p-3 bg-white dark:bg-surface-700 rounded-lg shadow-xl ring-2 ring-sentinel-500 cursor-grabbing">
                <h4 className="font-medium text-surface-800 dark:text-surface-200 mb-1">
                  {getActiveApp()?.job_title}
                </h4>
                <p className="text-sm text-surface-500 dark:text-surface-400">
                  {getActiveApp()?.company}
                </p>
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      </main>

      <ApplicationsOverlays
        appNotesId={appNotesId}
        appStatusId={appStatusId}
        applications={applications}
        notes={notes}
        onCloseAnalytics={() => setShowAnalytics(false)}
        onCloseApplication={closeApplicationDetail}
        onCloseInterviews={() => setShowInterviews(false)}
        onCloseTemplates={() => setShowTemplates(false)}
        onNotesChange={setNotes}
        onSaveNotes={handleAddNotes}
        onStatusChange={handleStatusChange}
        selectedApp={selectedApp}
        showAnalytics={showAnalytics}
        showInterviews={showInterviews}
        showTemplates={showTemplates}
        renderCompanyResearch={renderCompanyResearch}
      />
    </div>
  );
}
