// Dashboard - Main job search interface
// Refactored for v1.5 modularization - uses extracted hooks and components

import { useCallback, useEffect, useRef, useState } from "react";
import { DashboardSkeleton } from "../../ui/Skeleton";
import { useToast } from "../../shared/toast/useToast";
import { logError } from "../../shared/errorReporting/logger";
import { GOOD_JOB_MATCH_THRESHOLD } from "../../shared/jobMatchScore";
import { invalidateCacheByCommand } from "../../platform/tauri";
import { isValidJobUrl } from "./jobUrlValidation";
import { openDeepLink } from "../../shared/search-links";

// Extracted modules
import type {
  DashboardProps,
  Job,
  SavedSearch,
  ScrapingStatus,
  Statistics,
} from "./types";
import { useDashboardFilters } from "./hooks/useDashboardFilters";
import { useDashboardSearch } from "./hooks/useDashboardSearch";
import { useDashboardJobOps } from "./hooks/useDashboardJobOps";
import { useDashboardSavedSearches } from "./hooks/useDashboardSavedSearches";
import { useDashboardAutoRefresh } from "./hooks/useDashboardAutoRefresh";
import { useDashboardDataLifecycle } from "./hooks/useDashboardDataLifecycle";
import { useDashboardKeyboard } from "./hooks/useDashboardKeyboard";
import { useDashboardManualSearch } from "./hooks/useDashboardManualSearch";
import { DashboardFiltersBar } from "./components/DashboardFiltersBar";
import { DashboardHeader } from "./components/DashboardHeader";
import { DashboardStats } from "./components/DashboardStats";
import { DashboardCompareModal } from "./components/DashboardCompareModal";
import { DashboardJobList } from "./components/DashboardJobList";
import { DashboardNotesModal } from "./components/DashboardNotesModal";
import { DashboardSaveSearchModal } from "./components/DashboardSaveSearchModal";
import { DashboardErrorState } from "./components/DashboardErrorState";
import {
  DashboardCompanyResearchOverlay,
  DashboardDuplicateGroupsModal,
  DashboardImportJobModal,
  DashboardLinkedInWorkbenchModal,
  DashboardSettingsPanel,
} from "./components/DashboardOverlays";
import type { CompanyResearchTarget } from "../../shared/companyResearch";
import { DashboardWidgetsSection } from "./components/DashboardWidgetsSection";
import { QuickActions } from "./components/QuickActions";
import { getNoJobsEmptyStateCopy } from "./components/noJobsEmptyStateCopy";

export default function Dashboard({
  onNavigate,
  tourAction,
  renderApplicationAssistAction,
  renderCompanyResearch,
  settingsPage: SettingsPage,
  settingsInitialTab,
  linkedinWorkbench,
  showSettings: showSettingsProp,
  onShowSettingsChange,
  openImportOnMount = false,
  onImportHandled,
}: DashboardProps) {
  // Core state
  const [jobs, setJobs] = useState<Job[]>([]);
  const [statistics, setStatistics] = useState<Statistics>({
    total_jobs: 0,
    high_matches: 0,
    average_score: 0,
  });
  const [scrapingStatus, setScrapingStatus] = useState<ScrapingStatus>({
    last_scrape: null,
    next_scrape: null,
    is_running: false,
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showSettingsLocal, setShowSettingsLocal] = useState(false);
  const [researchTarget, setResearchTarget] = useState<CompanyResearchTarget | null>(null);
  const [showImportModal, setShowImportModal] = useState(false);
  const [showLinkedInWorkbench, setShowLinkedInWorkbench] = useState(false);
  const [salaryFloorUsd, setSalaryFloorUsd] = useState<number | null>(null);
  const [anyJobSourceEnabled, setAnyJobSourceEnabled] = useState<
    boolean | null
  >(null);

  const toast = useToast();

  // Use prop if provided, otherwise use local state
  const showSettings = showSettingsProp ?? showSettingsLocal;
  const setShowSettings = onShowSettingsChange ?? setShowSettingsLocal;

  // Refs
  const jobListRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null!);

  useEffect(() => {
    if (!openImportOnMount) return;
    setShowImportModal(true);
    onImportHandled?.();
  }, [openImportOnMount, onImportHandled]);

  // Stable callback for data updates
  const handleDataUpdate = useCallback(
    (data: { jobs: Job[]; stats: Statistics; status: ScrapingStatus }) => {
      setJobs(data.jobs);
      setStatistics(data.stats);
      setScrapingStatus(data.status);
    },
    [],
  );

  // Extracted hooks
  const filters = useDashboardFilters(jobs);
  const {
    searchHistory,
    addToSearchHistory,
    clearSearchHistory,
    showSearchHistory,
    setShowSearchHistory,
  } = useDashboardSearch();
  const jobOps = useDashboardJobOps(jobs, setJobs);
  const savedSearches = useDashboardSavedSearches();
  const { cooldownSeconds, handleSearchNow, searchCooldown, searching } =
    useDashboardManualSearch({
      jobs,
      setAnyJobSourceEnabled,
      setError,
      setJobs,
      setScrapingStatus,
      setStatistics,
    });

  // Auto-refresh hook
  const autoRefresh = useDashboardAutoRefresh({
    searching,
    showSettings,
    jobs,
    statistics,
    onDataUpdate: handleDataUpdate,
  });
  const { setAutoRefreshEnabled, setAutoRefreshInterval } = autoRefresh;
  const { fetchData, fetchDataRef } = useDashboardDataLifecycle({
    autoRefreshEnabled: autoRefresh.autoRefreshEnabled,
    jobs,
    setAnyJobSourceEnabled,
    setAutoRefreshEnabled,
    setAutoRefreshInterval,
    setError,
    setJobs,
    setLoading,
    setSalaryFloorUsd,
    setScrapingStatus,
    setStatistics,
  });

  // Open job link.
  const handleOpenJob = useCallback(
    async (job: Job) => {
      // Security: validate job link before opening.
      if (!isValidJobUrl(job.url)) {
        toast.error(
          "Check job link",
          "This saved link does not look safe to open.",
        );
        return;
      }

      try {
        await openDeepLink(job.url);
      } catch (err: unknown) {
        logError("Failed to open job link via Tauri command:", err);
        toast.error(
          "Could not open job link",
          "Copy the link and open it in your browser.",
        );
      }
    },
    [toast],
  );

  useEffect(() => {
    if (!researchTarget) return;

    const handleResearchEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setResearchTarget(null);
      }
    };

    document.addEventListener("keydown", handleResearchEscape);
    return () => {
      document.removeEventListener("keydown", handleResearchEscape);
    };
  }, [researchTarget]);

  const { selectedIndex, isKeyboardActive } = useDashboardKeyboard({
    items: filters.filteredAndSortedJobs,
    enabled: !showSettings && !loading && !jobOps.notesModalOpen,
    jobListRef,
    onOpen: handleOpenJob,
    onHide: (job) => jobOps.handleHideJob(job.id),
    onBookmark: (job) => jobOps.handleToggleBookmark(job.id),
    onNotes: (job) => jobOps.handleEditNotes(job.id, job.notes),
    onResearch: (job) => setResearchTarget({ companyName: job.company, jobHash: job.hash }),
    onToggleSelect: (job) => {
      jobOps.setSelectedJobIds((prev) => {
        const next = new Set(prev);
        if (next.has(job.id)) {
          next.delete(job.id);
        } else {
          next.add(job.id);
        }
        return next;
      });
      jobOps.setBulkMode(true);
    },
    onRefresh: () => handleSearchNow(),
    searchInputRef,
  });

  // Memoized callbacks for QuickActions and DashboardFiltersBar
  const handleExportHighMatches = useCallback(() => {
    const highMatchJobs = jobs.filter(
      (j) => (j.score ?? 0) >= GOOD_JOB_MATCH_THRESHOLD,
    );
    jobOps.handleExportJobs(highMatchJobs);
  }, [jobs, jobOps]);

  const handleShowHighMatchesOnly = useCallback(() => {
    filters.setScoreFilter("high");
  }, [filters]);

  const handleShowRemoteOnly = useCallback(() => {
    filters.setRemoteFilter("remote");
  }, [filters]);

  const handleExportFilteredJobs = useCallback(() => {
    jobOps.handleExportJobs(filters.filteredAndSortedJobs);
  }, [jobOps, filters.filteredAndSortedJobs]);

  const handleOpenLinkedInWorkbench = useCallback(() => {
    setShowLinkedInWorkbench(true);
  }, []);

  const handleLoadSearch = useCallback(
    (search: SavedSearch) => {
      savedSearches.handleLoadSearch(search, filters.loadFilters);
    },
    [savedSearches, filters.loadFilters],
  );

  const handleSelectAllJobs = useCallback(() => {
    jobOps.selectAllJobs(filters.filteredAndSortedJobs);
  }, [jobOps, filters.filteredAndSortedJobs]);

  const handleExportSelectedJobs = useCallback(() => {
    jobOps.handleBulkExport(
      filters.filteredAndSortedJobs.filter((j) =>
        jobOps.selectedJobIds.has(j.id),
      ),
    );
  }, [jobOps, filters.filteredAndSortedJobs]);

  const handleSettingsClose = useCallback(() => {
    invalidateCacheByCommand("get_dashboard_preferences");
    setShowSettings(false);
    void fetchDataRef.current?.();
  }, [fetchDataRef, setShowSettings]);

  // Settings modal
  if (showSettings) {
    return (
      <DashboardSettingsPanel onClose={handleSettingsClose}>
        {SettingsPage ? (
          <SettingsPage
            initialTab={settingsInitialTab}
            linkedinWorkbench={linkedinWorkbench}
            onClose={handleSettingsClose}
          />
        ) : null}
      </DashboardSettingsPanel>
    );
  }

  // Loading state - use skeleton for better perceived performance
  if (loading) {
    return <DashboardSkeleton />;
  }

  // Error state
  if (error) {
    return <DashboardErrorState error={error} onRetry={fetchData} />;
  }

  const noJobsCopy = getNoJobsEmptyStateCopy(anyJobSourceEnabled);
  const noSourcesEnabled = anyJobSourceEnabled === false;

  return (
    <div className="min-h-screen bg-surface-50 dark:bg-surface-900">
      <DashboardHeader
        scrapingStatus={scrapingStatus}
        autoRefreshEnabled={autoRefresh.autoRefreshEnabled}
        nextRefreshTime={autoRefresh.nextRefreshTime}
        formatTimeUntil={autoRefresh.formatTimeUntil}
        searching={searching}
        searchCooldown={searchCooldown}
        cooldownSeconds={cooldownSeconds}
        onSearchNow={handleSearchNow}
        onOpenSettings={() => setShowSettings(true)}
        tourAction={tourAction}
      />

      <main className="max-w-7xl mx-auto px-4 py-8 sm:px-6">
        <DashboardStats statistics={statistics} />

        <DashboardWidgetsSection />

        {/* Quick Actions */}
        <QuickActions
          totalJobs={statistics.total_jobs}
          highMatches={statistics.high_matches}
          filteredCount={filters.filteredAndSortedJobs.length}
          onExportHighMatches={handleExportHighMatches}
          onShowHighMatchesOnly={handleShowHighMatchesOnly}
          onShowRemoteOnly={handleShowRemoteOnly}
          onClearFilters={filters.clearFilters}
          hasActiveFilters={!!filters.hasActiveFilters}
          onImportJob={() => setShowImportModal(true)}
          onOpenLinkedInWorkbench={handleOpenLinkedInWorkbench}
        />

        <section className="mt-4">
          <DashboardFiltersBar
            jobs={jobs}
            filteredJobs={filters.filteredAndSortedJobs}
            textSearch={filters.textSearch}
            setTextSearch={filters.setTextSearch}
            searchInputRef={searchInputRef}
            showSearchHistory={showSearchHistory}
            setShowSearchHistory={setShowSearchHistory}
            searchHistory={searchHistory}
            addToSearchHistory={addToSearchHistory}
            clearSearchHistory={clearSearchHistory}
            sortBy={filters.sortBy}
            setSortBy={filters.setSortBy}
            scoreFilter={filters.scoreFilter}
            setScoreFilter={filters.setScoreFilter}
            sourceFilter={filters.sourceFilter}
            setSourceFilter={filters.setSourceFilter}
            availableSources={filters.availableSources}
            remoteFilter={filters.remoteFilter}
            setRemoteFilter={filters.setRemoteFilter}
            bookmarkFilter={filters.bookmarkFilter}
            setBookmarkFilter={filters.setBookmarkFilter}
            notesFilter={filters.notesFilter}
            setNotesFilter={filters.setNotesFilter}
            ghostFilter={filters.ghostFilter}
            setGhostFilter={filters.setGhostFilter}
            postedDateFilter={filters.postedDateFilter}
            setPostedDateFilter={filters.setPostedDateFilter}
            salaryMinFilter={filters.salaryMinFilter}
            setSalaryMinFilter={filters.setSalaryMinFilter}
            salaryMaxFilter={filters.salaryMaxFilter}
            setSalaryMaxFilter={filters.setSalaryMaxFilter}
            hasActiveFilters={!!filters.hasActiveFilters}
            clearFilters={filters.clearFilters}
            checkingDuplicates={jobOps.checkingDuplicates}
            handleCheckDuplicates={jobOps.handleCheckDuplicates}
            bulkMode={jobOps.bulkMode}
            toggleBulkMode={jobOps.toggleBulkMode}
            onExportJobs={handleExportFilteredJobs}
            onOpenSaveSearch={() => savedSearches.setSaveSearchModalOpen(true)}
            savedSearches={savedSearches.savedSearches}
            onLoadSearch={handleLoadSearch}
            selectedJobIds={jobOps.selectedJobIds}
            selectAllJobs={handleSelectAllJobs}
            clearSelection={jobOps.clearSelection}
            handleBulkExport={handleExportSelectedJobs}
            handleCompareJobs={jobOps.handleCompareJobs}
            handleBulkBookmark={jobOps.handleBulkBookmark}
            handleBulkHide={jobOps.handleBulkHide}
          />

          <DashboardJobList
            jobs={jobs}
            filteredJobs={filters.filteredAndSortedJobs}
            noJobsCopy={noJobsCopy}
            noSourcesEnabled={noSourcesEnabled}
            searching={searching}
            jobListRef={jobListRef}
            bulkMode={jobOps.bulkMode}
            selectedJobIds={jobOps.selectedJobIds}
            isKeyboardActive={isKeyboardActive}
            selectedIndex={selectedIndex}
            salaryFloorUsd={salaryFloorUsd}
            onSearchNow={handleSearchNow}
            onOpenSettings={() => setShowSettings(true)}
            onOpenImport={() => setShowImportModal(true)}
            onClearFilters={filters.clearFilters}
            onToggleJobSelection={jobOps.toggleJobSelection}
            onHideJob={jobOps.handleHideJob}
            onToggleBookmark={jobOps.handleToggleBookmark}
            onEditNotes={jobOps.handleEditNotes}
            onResearchCompany={(companyName, jobHash) => setResearchTarget({ companyName, jobHash })}
            renderApplicationAssistAction={
              renderApplicationAssistAction && onNavigate
                ? (job) =>
                    renderApplicationAssistAction(job, () =>
                      onNavigate("automation"),
                    )
                : undefined
            }
          />
        </section>
      </main>

      <DashboardNotesModal
        isOpen={jobOps.notesModalOpen}
        notesText={jobOps.notesText}
        onChange={jobOps.setNotesText}
        onClose={jobOps.handleCloseNotesModal}
        onSave={jobOps.handleSaveNotes}
      />

      <DashboardSaveSearchModal
        currentFilters={{
          sortBy: filters.sortBy,
          scoreFilter: filters.scoreFilter,
          sourceFilter: filters.sourceFilter,
          remoteFilter: filters.remoteFilter,
          bookmarkFilter: filters.bookmarkFilter,
          notesFilter: filters.notesFilter,
          postedDateFilter: filters.postedDateFilter,
          salaryMinFilter: filters.salaryMinFilter,
          salaryMaxFilter: filters.salaryMaxFilter,
        }}
        isOpen={savedSearches.saveSearchModalOpen}
        onClose={() => {
          savedSearches.setSaveSearchModalOpen(false);
          savedSearches.setNewSearchName("");
        }}
        newSearchName={savedSearches.newSearchName}
        savedSearches={savedSearches.savedSearches}
        onDeleteSearch={savedSearches.handleDeleteSearch}
        onLoadSearch={(search) => {
          savedSearches.handleLoadSearch(search, filters.loadFilters);
          savedSearches.setSaveSearchModalOpen(false);
        }}
        onNameChange={savedSearches.setNewSearchName}
        onSave={() => savedSearches.handleSaveSearch(filters.getCurrentFilters)}
      />

      <DashboardDuplicateGroupsModal
        isOpen={jobOps.duplicatesModalOpen}
        duplicateGroups={jobOps.duplicateGroups}
        onClose={() => jobOps.setDuplicatesModalOpen(false)}
        onMerge={jobOps.handleMergeDuplicates}
        onMergeAll={() => jobOps.handleMergeAllDuplicates(fetchData)}
      />

      <DashboardCompareModal
        isOpen={jobOps.compareModalOpen}
        onClose={() => jobOps.setCompareModalOpen(false)}
        comparedJobs={jobOps.comparedJobs}
      />

      <DashboardLinkedInWorkbenchModal
        isOpen={showLinkedInWorkbench}
        workbench={linkedinWorkbench}
        onClose={() => setShowLinkedInWorkbench(false)}
      />

      <DashboardCompanyResearchOverlay
        researchTarget={researchTarget}
        renderCompanyResearch={renderCompanyResearch}
        onClose={() => setResearchTarget(null)}
      />

      <DashboardImportJobModal
        isOpen={showImportModal}
        onClose={() => setShowImportModal(false)}
        onImportSuccess={() => {
          fetchData();
        }}
      />
    </div>
  );
}
