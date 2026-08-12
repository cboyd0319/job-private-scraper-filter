import { useEffect, useState, useCallback, lazy, Suspense } from "react";
import { invoke } from "../platform/tauri";
import type { Page } from "./routes";
import { default as ErrorBoundary } from "./errors/ErrorBoundary";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { SkipToContent } from "../ui/SkipToContent";
import { CommandPalette } from "./commands/CommandPalette";
import { default as PageErrorBoundary } from "./errors/PageErrorBoundary";
import { KeyboardShortcutsHelp } from "./commands/KeyboardShortcutsHelp";
import {
  OnboardingProvider,
  TourHelpButton,
} from "./onboarding/OnboardingProvider";
import { useOnboarding } from "./onboarding/useOnboarding";
import { Navigation } from "./Navigation";
import { KeyboardShortcutsProvider } from "./keyboard/KeyboardShortcutsProvider";
import { useKeyboardShortcuts } from "./keyboard/useKeyboardShortcuts";
import { logError } from "../shared/errorReporting/logger";
import { defaultTourSteps } from "./onboarding/tourSteps";
import type { CompanyResearchPanelProps } from "../shared/companyResearch";
import { StartupRecovery } from "./StartupRecovery";
import { NativeFileDropReview } from "./NativeFileDropReview";

export { StartupRecovery } from "./StartupRecovery";

// Lazy load pages for better initial load performance
const SetupWizard = lazy(() => import("../features/onboarding"));
const Dashboard = lazy(() => import("../features/dashboard"));
const Applications = lazy(() => import("../features/applications"));
const Settings = lazy(() => import("../features/settings"));
const LinkedInWorkbench = lazy(() =>
  import("../features/linkedin-workbench").then((module) => ({
    default: module.LinkedInWorkbench,
  })),
);
const CompanyResearchPanel = lazy(() =>
  import("../features/company-research").then((module) => ({
    default: module.CompanyResearchPanel,
  })),
);
const ResumeLibraryPage = lazy(() =>
  import("../features/resumes").then((module) => ({
    default: module.ResumeLibraryPage,
  })),
);
const ResumeBuilderPage = lazy(() =>
  import("../features/resumes").then((module) => ({
    default: module.ResumeBuilderPage,
  })),
);
const ResumeMatchPage = lazy(() =>
  import("../features/resumes").then((module) => ({
    default: module.ResumeMatchPage,
  })),
);
const Salary = lazy(() => import("../features/salary"));
const Market = lazy(() => import("../features/market"));
const SearchLinks = lazy(() => import("../features/search-links"));
const ApplicationProfile = lazy(() => import("../features/application-assist"));
const ApplyButton = lazy(() =>
  import("../features/application-assist").then((module) => ({
    default: module.ApplyButton,
  })),
);
const OpportunityCaseAction = lazy(() =>
  import("../features/opportunity-case/OpportunityCaseAction").then((module) => ({
    default: module.OpportunityCaseAction,
  })),
);

// Loading fallback for lazy-loaded pages
function PageLoader({ message = "Loading..." }: { message?: string }) {
  return (
    <div className="min-h-[60vh] flex items-center justify-center">
      <LoadingSpinner message={message} />
    </div>
  );
}

function renderCompanyResearch(props: CompanyResearchPanelProps) {
  return (
    <Suspense fallback={<LoadingSpinner message="Loading company research..." />}>
      <CompanyResearchPanel {...props} />
    </Suspense>
  );
}

// Global keyboard help modal - consumes context
function GlobalKeyboardHelp() {
  const { isHelpOpen, closeHelp } = useKeyboardShortcuts();
  return <KeyboardShortcutsHelp isOpen={isHelpOpen} onClose={closeHelp} />;
}

// Auto-start tour after setup completion
function TourStartTrigger({
  shouldStart,
  onStarted,
}: {
  shouldStart: boolean;
  onStarted: () => void;
}) {
  const { startTour, hasCompletedTour } = useOnboarding();

  useEffect(() => {
    if (shouldStart && !hasCompletedTour) {
      // Small delay to let the dashboard render first
      const timer = setTimeout(() => {
        startTour();
        onStarted();
      }, 500);
      return () => clearTimeout(timer);
    }
    return undefined;
  }, [shouldStart, hasCompletedTour, startTour, onStarted]);

  return null;
}

function App() {
  const [isFirstRun, setIsFirstRun] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [startupError, setStartupError] = useState(false);
  const [currentPage, setCurrentPage] = useState<Page>("dashboard");
  const [showSettings, setShowSettings] = useState(false);
  const [settingsInitialTab, setSettingsInitialTab] = useState<
    "basic" | "advanced"
  >("basic");
  const [openImportOnDashboard, setOpenImportOnDashboard] = useState(false);
  const [shouldStartTour, setShouldStartTour] = useState(false);

  const checkFirstRun = useCallback(async () => {
    try {
      setStartupError(false);
      setLoading(true);
      const firstRun = await invoke<boolean>("is_first_run");
      setIsFirstRun(firstRun);
    } catch (error) {
      logError("Failed to check first run:", error);
      setStartupError(true);
      setIsFirstRun(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    checkFirstRun();
  }, [checkFirstRun]);

  const handleSetupComplete = () => {
    setIsFirstRun(false);
    // Trigger tour after setup completes
    setShouldStartTour(true);
  };

  const handleSetupSkip = () => {
    setIsFirstRun(false);
  };

  const navigateTo = (page: Page) => {
    setCurrentPage(page);
  };

  // Moved before early returns to comply with hooks rules
  const openSettings = useCallback(() => {
    setSettingsInitialTab("basic");
    setShowSettings(true);
  }, []);

  const openSourcesFromApplications = useCallback(() => {
    setSettingsInitialTab("advanced");
    setShowSettings(true);
    setCurrentPage("dashboard");
  }, []);

  const openJobImportFromApplications = useCallback(() => {
    setOpenImportOnDashboard(true);
    setCurrentPage("dashboard");
  }, []);

  if (loading) {
    return <LoadingSpinner message="Initializing JobSentinel..." />;
  }

  if (startupError) {
    return <StartupRecovery onRetry={() => void checkFirstRun()} />;
  }

  if (isFirstRun) {
    return (
      <ErrorBoundary>
        <SkipToContent />
        <div className="min-h-screen" id="main-content" tabIndex={-1}>
          <Suspense
            fallback={<PageLoader message="Getting JobSentinel ready..." />}
          >
            <SetupWizard onComplete={handleSetupComplete} onSkip={handleSetupSkip} />
          </Suspense>
        </div>
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary>
      <KeyboardShortcutsProvider
        onNavigate={(page) => navigateTo(page as Page)}
        onOpenSettings={openSettings}
      >
        <OnboardingProvider steps={defaultTourSteps}>
          <SkipToContent />
          <NativeFileDropReview />
          <CommandPalette />
          <GlobalKeyboardHelp />
          <TourStartTrigger
            shouldStart={shouldStartTour}
            onStarted={() => setShouldStartTour(false)}
          />

          {/* Navigation sidebar */}
          <Navigation currentPage={currentPage} onNavigate={navigateTo} />

          {/* Main content with left margin for sidebar */}
          <div
            className="min-h-screen ml-16 overflow-x-hidden"
            id="main-content"
            tabIndex={-1}
          >
            <Suspense fallback={<PageLoader />}>
              {currentPage === "dashboard" && (
                <PageErrorBoundary pageName="Dashboard">
                  <Dashboard
                    onNavigate={navigateTo}
                    tourAction={<TourHelpButton />}
                    renderApplicationAssistAction={(
                      job,
                      onOpenApplicationAssist,
                    ) => (
                      <Suspense fallback={null}>
                        <div className="flex flex-wrap items-center gap-2">
                          {job.hash && <OpportunityCaseAction jobHash={job.hash} />}
                          <ApplyButton
                            job={{
                              id: job.id,
                              hash: job.hash ?? `job-${job.id}`,
                              title: job.title,
                              company: job.company,
                              location: job.location ?? "",
                              url: job.url,
                              description: job.description ?? undefined,
                              score: job.score ?? undefined,
                            }}
                            onOpenApplicationAssist={onOpenApplicationAssist}
                          />
                        </div>
                      </Suspense>
                    )}
                    settingsPage={Settings}
                    settingsInitialTab={settingsInitialTab}
                    linkedinWorkbench={<LinkedInWorkbench />}
                    renderCompanyResearch={renderCompanyResearch}
                    showSettings={showSettings}
                    onShowSettingsChange={setShowSettings}
                    openImportOnMount={openImportOnDashboard}
                    onImportHandled={() => setOpenImportOnDashboard(false)}
                  />
                </PageErrorBoundary>
              )}
              {currentPage === "applications" && (
                <PageErrorBoundary
                  pageName="Applications"
                  onBack={() => navigateTo("dashboard")}
                >
                  <Applications
                    onBack={() => navigateTo("dashboard")}
                    onImportJob={openJobImportFromApplications}
                    onOpenSalary={() => navigateTo("salary")}
                    onOpenSources={openSourcesFromApplications}
                    renderCompanyResearch={renderCompanyResearch}
                  />
                </PageErrorBoundary>
              )}
              {currentPage === "resume" && (
                <PageErrorBoundary
                  pageName="Resume"
                  onBack={() => navigateTo("dashboard")}
                >
                  <ResumeLibraryPage onBack={() => navigateTo("dashboard")} />
                </PageErrorBoundary>
              )}
              {currentPage === "resume-builder" && (
                <PageErrorBoundary
                  pageName="Resume Builder"
                  onBack={() => navigateTo("dashboard")}
                >
                  <ResumeBuilderPage onBack={() => navigateTo("dashboard")} />
                </PageErrorBoundary>
              )}
              {currentPage === "ats-optimizer" && (
                <PageErrorBoundary
                  pageName="Resume Match"
                  onBack={() => navigateTo("dashboard")}
                >
                  <ResumeMatchPage
                    onBack={() => navigateTo("dashboard")}
                    onNavigate={navigateTo}
                  />
                </PageErrorBoundary>
              )}
              {currentPage === "salary" && (
                <PageErrorBoundary
                  pageName="Salary"
                  onBack={() => navigateTo("dashboard")}
                >
                  <Salary onBack={() => navigateTo("dashboard")} />
                </PageErrorBoundary>
              )}
              {currentPage === "market" && (
                <PageErrorBoundary
                  pageName="Market"
                  onBack={() => navigateTo("dashboard")}
                >
                  <Market onBack={() => navigateTo("dashboard")} />
                </PageErrorBoundary>
              )}
              {currentPage === "search-links" && (
                <PageErrorBoundary
                  pageName="Search Links"
                  onBack={() => navigateTo("dashboard")}
                >
                  <SearchLinks />
                </PageErrorBoundary>
              )}
              {currentPage === "automation" && (
                <PageErrorBoundary
                  pageName="Application Assist"
                  onBack={() => navigateTo("dashboard")}
                >
                  <ApplicationProfile onBack={() => navigateTo("dashboard")} />
                </PageErrorBoundary>
              )}
            </Suspense>
          </div>
        </OnboardingProvider>
      </KeyboardShortcutsProvider>
    </ErrorBoundary>
  );
}

export default App;
