import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "../../platform/tauri";
import { Button } from "../../ui/Button";
import { HelpIcon } from "../../ui/HelpIcon";
import { Modal } from "../../ui/Modal";
import { ScraperHealthDashboard } from "./sources/health/ScraperHealthDashboard";
import { FeedbackModal } from "./support/feedback/FeedbackModal";
import { BrowserImportSection } from "./sources/browser-import/BrowserImportSection";
import { useToast } from "../../shared/toast/useToast";
import { logError } from "../../shared/errorReporting/logger";
import { getUserFriendlyError } from "../../shared/errorReporting/messages";
import { normalizeRestrictedSourceAcknowledgements } from "../../shared/restrictedSourceTaxonomy";
import {
  cacheDetectedLocation,
  readCachedDetectedLocation,
  type DetectedLocation,
} from "../../shared/location/detectedLocationCache";
import {
  buildJobsWithGptPayload,
  isCurrentJobsWithGptPayloadApproved,
  type Config,
  type SettingsProps,
  type SourceRequestSummary,
} from "./config/SettingsConfig";
import { normalizeExternalAiSettings } from "./external-ai/externalAiProviders";
import { useJobBoardRecommendations } from "./sources/SettingsJobBoardRecommendations";
import { SettingsJobSourcesSection } from "./sources/SettingsJobSourcesSection";
import { SettingsPostingRiskSection } from "./search/SettingsPostingRiskSection";
import { SettingsResumeMatchingSection } from "./matching/SettingsResumeMatchingSection";
import { SettingsNotificationsSection } from "./notifications/SettingsNotificationsSection";
import { SettingsExternalAiSection } from "./external-ai/SettingsExternalAiSection";
import { SettingsSearchPreferencesSection } from "./search/SettingsSearchPreferencesSection";
import { SettingsCredentialLockSection } from "./credentials/SettingsCredentialLockSection";
import {
  SettingsBackupSection,
  SettingsHelpStatusSection,
  SettingsSupportSection,
} from "./support/SettingsSupportSections";
import { useSettingsCredentials } from "./credentials/useSettingsCredentials";
import { useSettingsSupportReports } from "./support/useSettingsSupportReports";
import { useSettingsLocalDataBackup } from "./support/useSettingsLocalDataBackup";
import { useSettingsSearchPreferenceInputs } from "./search/useSettingsSearchPreferenceInputs";
import { useSettingsGhostConfig } from "./search/useSettingsGhostConfig";
import { SettingsTabs } from "./shared/SettingsTabs";
import { useSettingsSave } from "./useSettingsSave";
import { LocalRecoveryPanel } from "./support/LocalRecoveryPanel";
import { PortableRecoveryPanel } from "./support/PortableRecoveryPanel";
import {
  SettingsLoadFailure,
  SettingsLoadingState,
} from "./SettingsPageStartup";

export default function Settings({ initialTab = "basic", linkedinWorkbench, onClose }: SettingsProps) {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [showHealthDashboard, setShowHealthDashboard] = useState(false);
  const [jobsWithGptLastRequest, setJobsWithGptLastRequest] =
    useState<SourceRequestSummary | null>(null);
  const [showJobsWithGptEndpoint, setShowJobsWithGptEndpoint] = useState(false);
  const [activeTab, setActiveTab] = useState<"basic" | "advanced">(initialTab);
  const toast = useToast();
  const {
    error: toastError,
    success: toastSuccess,
    warning: toastWarning,
  } = toast;
  const {
    credentials,
    credentialStatus,
    getCredentialSaveEntries,
    initializeCredentialStatus,
    markCredentialNeedsAttention,
    markCredentialsSaved,
    setCredentials,
  } = useSettingsCredentials(toast);
  const { handleSave, saving } = useSettingsSave({
    config,
    credentials,
    credentialStatus,
    getCredentialSaveEntries,
    markCredentialsSaved,
    onClose,
    toastError,
    toastSuccess,
    toastWarning,
  });
  const {
    ghostConfig,
    ghostConfigLoading,
    ghostPreset,
    handleResetGhostConfig,
    handleSaveGhostConfig,
    setGhostConfig,
    setGhostPreset,
  } = useSettingsGhostConfig({ toastError, toastSuccess, toastWarning });
  const {
    closeFeedbackModal,
    copyingDebugReport,
    handleCopyDebugReport,
    handleSaveDebugReport,
    openFeedbackModal,
    savingDebugReport,
    showFeedbackModal,
  } = useSettingsSupportReports(toast);
  const { handleExportConfig, handleImportConfig } = useSettingsLocalDataBackup(
    {
      config,
      setConfig,
      toastError,
      toastSuccess,
    },
  );

  // Location detection state
  const [detectedLocation, setDetectedLocation] = useState<DetectedLocation | null>(
    () => readCachedDetectedLocation(),
  );
  const [isDetectingLocation, setIsDetectingLocation] = useState(false);
  const searchPreferenceInputs = useSettingsSearchPreferenceInputs({
    config,
    detectedLocation,
    setConfig,
    toastSuccess,
  });

  const jobBoardRecommendations = useJobBoardRecommendations(config, setConfig);

  const jobsWithGptPayload = useMemo(
    () => (config ? buildJobsWithGptPayload(config) : null),
    [config],
  );
  const jobsWithGptPayloadApproved = useMemo(
    () =>
      config
        ? isCurrentJobsWithGptPayloadApproved(config, jobsWithGptPayload)
        : false,
    [config, jobsWithGptPayload],
  );
  const approveJobsWithGptPayload = useCallback(() => {
    if (!config || !jobsWithGptPayload) return;

    setConfig({
      ...config,
      jobswithgpt_approval: {
        enabled: true,
        payload: jobsWithGptPayload,
        approved_at: new Date().toISOString(),
      },
    });
  }, [config, jobsWithGptPayload]);

  const clearJobsWithGptApproval = useCallback(() => {
    if (!config) return;

    setConfig({
      ...config,
      jobswithgpt_approval: {
        enabled: false,
        payload: null,
        approved_at: null,
      },
    });
  }, [config]);

  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);

      // Load config (non-sensitive settings)
      const configData = await invoke<Config>("get_config");
      const alerts = configData.alerts;
      const emailAlerts = alerts.email as
        Partial<Config["alerts"]["email"]> | undefined;
      const nextConfig = {
        ...configData,
        preferred_companies: configData.preferred_companies ?? [],
        blocked_companies: configData.blocked_companies ?? [],
        bookmarklet_port: configData.bookmarklet_port ?? 4321,
        alerts: {
          ...alerts,
          slack: alerts.slack ?? { enabled: false },
          email: {
            enabled: false,
            smtp_server: "",
            smtp_port: 587,
            smtp_username: "",
            from_email: "",
            to_emails: [],
            use_starttls: true,
            ...emailAlerts,
          },
          discord: alerts.discord ?? { enabled: false },
          telegram: alerts.telegram ?? { enabled: false },
          teams: alerts.teams ?? { enabled: false },
          desktop: {
            enabled: alerts.desktop?.enabled ?? true,
            show_when_focused: alerts.desktop?.show_when_focused ?? false,
            play_sound: alerts.desktop?.play_sound ?? false,
          },
        },
        jobswithgpt_endpoint: configData.jobswithgpt_endpoint ?? "",
        jobswithgpt_approval: configData.jobswithgpt_approval ?? {
          enabled: false,
          payload: null,
          approved_at: null,
        },
        external_ai: normalizeExternalAiSettings(configData.external_ai),
        linkedin: {
          ...configData.linkedin,
          enabled: false,
        },
        restricted_source_acknowledgements:
          normalizeRestrictedSourceAcknowledgements(
            configData.restricted_source_acknowledgements,
          ),
      };
      setConfig(nextConfig);
      initializeCredentialStatus(nextConfig);

      try {
        const lastRequest = await invoke<SourceRequestSummary | null>(
          "get_latest_source_request",
          { source: "jobswithgpt" },
        );
        setJobsWithGptLastRequest(lastRequest);
      } catch (error) {
        logError("Could not load source request history:", error);
        setJobsWithGptLastRequest(null);
      }
    } catch (error: unknown) {
      logError("Failed to load config:", error);
      const friendly = getUserFriendlyError(error);
      toastError(friendly.title, friendly.message);
    } finally {
      setLoading(false);
    }
  }, [initializeCredentialStatus, toastError]);

  const handleDetectLocation = useCallback(async () => {
    setIsDetectingLocation(true);
    try {
      const location = await invoke<DetectedLocation>("detect_location");
      setDetectedLocation(location);
      cacheDetectedLocation(location);
    } catch {
      toastWarning("Location unavailable", "Enter a city manually.");
    } finally {
      setIsDetectingLocation(false);
    }
  }, [toastWarning]);

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  if (loading) {
    return (
      <Modal
        isOpen
        onClose={onClose}
        title="Settings"
        description="Loading settings"
        size="wide"
        closeButtonLabel="Close settings"
      >
        <SettingsLoadingState />
      </Modal>
    );
  }

  if (!config) {
    return (
      <>
        <Modal
          isOpen
          onClose={onClose}
          title="Settings could not load"
          description="Try again. If this keeps happening, copy or save a safe support report before closing and reopening JobSentinel."
          size="wide"
          closeButtonLabel="Close settings"
        >
          <SettingsLoadFailure
            copyingDebugReport={copyingDebugReport}
            onClose={onClose}
            onCopyDebugReport={handleCopyDebugReport}
            onOpenFeedback={openFeedbackModal}
            onRetry={() => void loadConfig()}
            onSaveDebugReport={handleSaveDebugReport}
            savingDebugReport={savingDebugReport}
          />
        </Modal>
        <FeedbackModal
          isOpen={showFeedbackModal}
          onClose={closeFeedbackModal}
        />
      </>
    );
  }

  return (
    <>
      <Modal
        isOpen
        onClose={onClose}
        title="Settings"
        description="Update your job search preferences"
        size="wide"
        closeButtonLabel="Close settings"
      >
        <SettingsTabs activeTab={activeTab} onActiveTabChange={setActiveTab} />

        {/* BASIC SETTINGS TAB */}
        {activeTab === "basic" && (
          <div
            role="tabpanel"
            id="basic-settings-panel"
            aria-labelledby="basic-settings-tab"
          >
            <SettingsSearchPreferencesSection
              config={config}
              titleInput={searchPreferenceInputs.titleInput}
              blockedTitleInput={searchPreferenceInputs.blockedTitleInput}
              skillInput={searchPreferenceInputs.skillInput}
              excludeKeywordInput={searchPreferenceInputs.excludeKeywordInput}
              cityInput={searchPreferenceInputs.cityInput}
              preferredCompanyInput={
                searchPreferenceInputs.preferredCompanyInput
              }
              blockedCompanyInput={searchPreferenceInputs.blockedCompanyInput}
              detectedLocation={detectedLocation}
              isDetectingLocation={isDetectingLocation}
              onConfigChange={setConfig}
              onTitleInputChange={searchPreferenceInputs.setTitleInput}
              onBlockedTitleInputChange={
                searchPreferenceInputs.setBlockedTitleInput
              }
              onSkillInputChange={searchPreferenceInputs.setSkillInput}
              onExcludeKeywordInputChange={
                searchPreferenceInputs.setExcludeKeywordInput
              }
              onCityInputChange={searchPreferenceInputs.setCityInput}
              onPreferredCompanyInputChange={
                searchPreferenceInputs.setPreferredCompanyInput
              }
              onBlockedCompanyInputChange={
                searchPreferenceInputs.setBlockedCompanyInput
              }
              onAddTitle={searchPreferenceInputs.handleAddTitle}
              onRemoveTitle={searchPreferenceInputs.handleRemoveTitle}
              onAddBlockedTitle={searchPreferenceInputs.handleAddBlockedTitle}
              onRemoveBlockedTitle={
                searchPreferenceInputs.handleRemoveBlockedTitle
              }
              onAddSkill={searchPreferenceInputs.handleAddSkill}
              onRemoveSkill={searchPreferenceInputs.handleRemoveSkill}
              onAddExcludeKeyword={
                searchPreferenceInputs.handleAddExcludeKeyword
              }
              onRemoveExcludeKeyword={
                searchPreferenceInputs.handleRemoveExcludeKeyword
              }
              onDetectLocation={handleDetectLocation}
              onUseDetectedLocation={
                searchPreferenceInputs.handleUseDetectedLocation
              }
              onAddCity={searchPreferenceInputs.handleAddCity}
              onRemoveCity={searchPreferenceInputs.handleRemoveCity}
              onAddPreferredCompany={
                searchPreferenceInputs.handleAddPreferredCompany
              }
              onRemovePreferredCompany={
                searchPreferenceInputs.handleRemovePreferredCompany
              }
              onAddBlockedCompany={
                searchPreferenceInputs.handleAddBlockedCompany
              }
              onRemoveBlockedCompany={
                searchPreferenceInputs.handleRemoveBlockedCompany
              }
            />
          </div>
        )}

        {/* ADVANCED SETTINGS TAB */}
        {activeTab === "advanced" && (
          <div
            role="tabpanel"
            id="advanced-settings-panel"
            aria-labelledby="advanced-settings-tab"
          >
            <SettingsNotificationsSection
              config={config}
              credentialStatus={credentialStatus}
              credentials={credentials}
              markCredentialNeedsAttention={markCredentialNeedsAttention}
              setConfig={setConfig}
              setCredentials={setCredentials}
            />

            <SettingsCredentialLockSection />

            <SettingsJobSourcesSection
              config={config}
              credentialStatus={credentialStatus}
              credentials={credentials}
              jobBoardRecommendations={jobBoardRecommendations}
              jobsWithGptLastRequest={jobsWithGptLastRequest}
              jobsWithGptPayload={jobsWithGptPayload}
              jobsWithGptPayloadApproved={jobsWithGptPayloadApproved}
              linkedinWorkbench={linkedinWorkbench}
              onApproveJobsWithGptPayload={approveJobsWithGptPayload}
              onClearJobsWithGptApproval={clearJobsWithGptApproval}
              setConfig={setConfig}
              setCredentials={setCredentials}
              setShowJobsWithGptEndpoint={setShowJobsWithGptEndpoint}
              showJobsWithGptEndpoint={showJobsWithGptEndpoint}
            />

            {/* Browser Button */}
            <section className="mb-6">
              <h3 className="font-medium text-surface-800 dark:text-surface-200 mb-3 flex items-center gap-2">
                Browser Button
                <HelpIcon text="Save many job pages into JobSentinel with a browser button you control." />
              </h3>
              <BrowserImportSection />
            </section>

            <SettingsPostingRiskSection
              ghostConfig={ghostConfig}
              ghostConfigLoading={ghostConfigLoading}
              ghostPreset={ghostPreset}
              onGhostConfigChange={setGhostConfig}
              onGhostPresetChange={setGhostPreset}
              onReset={handleResetGhostConfig}
              onSave={handleSaveGhostConfig}
            />

            <SettingsResumeMatchingSection
              config={config}
              onConfigChange={setConfig}
            />

            <SettingsExternalAiSection
              config={config}
              credentialStatus={credentialStatus}
              credentials={credentials}
              onConfigChange={setConfig}
              onCredentialsChange={setCredentials}
            />

            <SettingsHelpStatusSection
              onShowHealthDashboard={() => setShowHealthDashboard(true)}
            />
          </div>
        )}

        <SettingsBackupSection
          onExportConfig={handleExportConfig}
          onImportConfig={handleImportConfig}
        />

        <LocalRecoveryPanel />

        <PortableRecoveryPanel />

        <SettingsSupportSection
          copyingDebugReport={copyingDebugReport}
          onCopyDebugReport={handleCopyDebugReport}
          onOpenFeedback={openFeedbackModal}
          onSaveDebugReport={handleSaveDebugReport}
          savingDebugReport={savingDebugReport}
        />

        {/* Actions */}
        <div className="flex gap-3">
          <Button variant="secondary" onClick={onClose} className="flex-1">
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            loading={saving}
            loadingText="Saving..."
            className="flex-1"
          >
            Save Changes
          </Button>
        </div>
      </Modal>

      {/* Job Sources Modal */}
      {showHealthDashboard && (
        <ScraperHealthDashboard onClose={() => setShowHealthDashboard(false)} />
      )}

      {/* Feedback Modal */}
      <FeedbackModal isOpen={showFeedbackModal} onClose={closeFeedbackModal} />
    </>
  );
}
