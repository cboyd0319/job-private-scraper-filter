import { Button } from "../../ui/Button";
import {
  FRESHNESS_OPTIONS,
  REVIEW_VOLUME_OPTIONS,
  type FreshnessPreference,
  type ReviewVolumePreference,
  type SetupConfig,
  type SetupJobSourceKey,
  type SetupSearchSummary,
} from "./setupWizardPreferences";
import { SetupWizardSearchSummary } from "./SetupWizardSearchSummary";
import { SetupWizardSourceReview } from "./SetupWizardSourceReview";
import type { SetupWizardSourceReviewOption } from "./setupWizardSourceReviewState";

interface SetupWizardNotificationsStepProps {
  config: SetupConfig;
  freshnessPreference: FreshnessPreference;
  reviewVolumePreference: ReviewVolumePreference;
  resumeSkillSummary?: {
    resumeName: string;
    skills: string[];
  } | null;
  searchSummary: SetupSearchSummary;
  suggestedJobSources: SetupWizardSourceReviewOption[];
  onBack: () => void;
  onComplete: () => void;
  isSaving: boolean;
  onDesktopAlertsChange: (enabled: boolean) => void;
  onFreshnessPreferenceChange: (preference: FreshnessPreference) => void;
  onQuietAlertModeChange: (enabled: boolean) => void;
  onReviewVolumePreferenceChange: (preference: ReviewVolumePreference) => void;
  onToggleJobSource: (source: SetupJobSourceKey, enabled: boolean) => void;
}

export function SetupWizardNotificationsStep({
  config,
  freshnessPreference,
  reviewVolumePreference,
  resumeSkillSummary,
  searchSummary,
  suggestedJobSources,
  onBack,
  onComplete,
  isSaving,
  onDesktopAlertsChange,
  onFreshnessPreferenceChange,
  onQuietAlertModeChange,
  onReviewVolumePreferenceChange,
  onToggleJobSource,
}: SetupWizardNotificationsStepProps) {
  return (
    <div className="motion-safe:animate-slide-up">
      <fieldset className="mb-6" aria-describedby="setup-freshness-help">
        <legend className="font-semibold text-surface-800 mb-2">
          Fresh and verified jobs
        </legend>
        <p id="setup-freshness-help" className="mb-3 text-sm text-surface-500">
          Choose how JobSentinel handles older or hard-to-verify postings.
        </p>
        <div className="space-y-2">
          {FRESHNESS_OPTIONS.map((option) => {
            const checked = freshnessPreference === option.id;
            return (
              <label
                key={option.id}
                className={`
                  flex items-start gap-3 rounded-lg border-2 p-3 cursor-pointer transition-all duration-150
                  ${checked
                    ? "border-sentinel-500 bg-sentinel-50"
                    : "border-surface-200 hover:border-surface-300"
                  }
                `}
              >
                <input
                  type="radio"
                  name="freshness-preference"
                  value={option.id}
                  checked={checked}
                  onChange={() => onFreshnessPreferenceChange(option.id)}
                  className="mt-1 h-4 w-4 border-surface-300 text-sentinel-500 focus-visible:ring-sentinel-500"
                />
                <span>
                  <span className="block font-medium text-surface-800">
                    {option.label}
                  </span>
                  <span className="block text-sm text-surface-500">
                    {option.description}
                  </span>
                </span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <fieldset className="mb-6" aria-describedby="setup-review-volume-help">
        <legend className="font-semibold text-surface-800 mb-2">
          Jobs to review
        </legend>
        <p id="setup-review-volume-help" className="mb-3 text-sm text-surface-500">
          Choose how broad the first results and alerts should feel.
        </p>
        <div className="space-y-2">
          {REVIEW_VOLUME_OPTIONS.map((option) => {
            const checked = reviewVolumePreference === option.id;
            return (
              <label
                key={option.id}
                className={`
                  flex items-start gap-3 rounded-lg border-2 p-3 cursor-pointer transition-all duration-150
                  ${checked
                    ? "border-sentinel-500 bg-sentinel-50"
                    : "border-surface-200 hover:border-surface-300"
                  }
                `}
              >
                <input
                  type="radio"
                  name="review-volume-preference"
                  value={option.id}
                  checked={checked}
                  onChange={() => onReviewVolumePreferenceChange(option.id)}
                  className="mt-1 h-4 w-4 border-surface-300 text-sentinel-500 focus-visible:ring-sentinel-500"
                />
                <span>
                  <span className="block font-medium text-surface-800">
                    {option.label}
                  </span>
                  <span className="block text-sm text-surface-500">
                    {option.description}
                  </span>
                </span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <div className="mb-6">
        <p className="text-surface-600 mb-4 text-center">
          Desktop alerts are optional. Turn them on only if you want this computer
          to show job-search notifications.
        </p>

        <div className="mb-6 rounded-lg border-2 border-surface-200 p-4">
          <p className="font-medium text-surface-700">Alerts</p>
          <p className="mt-1 text-sm text-surface-500">
            Email or chat alerts can be added later in Settings.
          </p>
          <label
            className="mt-4 flex cursor-pointer items-start gap-3 rounded-lg border border-surface-200 bg-surface-50 p-3"
            htmlFor="desktop-alerts"
          >
            <input
              id="desktop-alerts"
              type="checkbox"
              checked={config.alerts.desktop.enabled}
              onChange={(e) => onDesktopAlertsChange(e.target.checked)}
              className="mt-1 h-4 w-4 rounded border-surface-300 text-sentinel-500 focus-visible:ring-sentinel-500"
            />
            <span>
              <span className="block font-medium text-surface-800">
                Desktop alerts
              </span>
              <span className="block text-sm text-surface-500">
                Show job-search notifications on this computer.
              </span>
            </span>
          </label>
          <p className="mt-3 text-sm text-surface-500">
            {searchSummary.alerts}
          </p>
          <label
            className="mt-4 flex cursor-pointer items-start gap-3 rounded-lg border border-surface-200 bg-surface-50 p-3"
            htmlFor="quiet-alert-mode"
          >
            <input
              id="quiet-alert-mode"
              type="checkbox"
              checked={!config.alerts.desktop.play_sound}
              disabled={!config.alerts.desktop.enabled}
              onChange={(e) => onQuietAlertModeChange(e.target.checked)}
              className="mt-1 h-4 w-4 rounded border-surface-300 text-sentinel-500 focus-visible:ring-sentinel-500"
            />
            <span>
              <span className="block font-medium text-surface-800">
                Quiet job-search mode
              </span>
              <span className="block text-sm text-surface-500">
                No sound. Good for a private or quieter search.
              </span>
            </span>
          </label>
        </div>
      </div>

      <SetupWizardSourceReview
        sources={suggestedJobSources}
        onToggleSource={onToggleJobSource}
      />
      <SetupWizardSearchSummary
        summary={searchSummary}
        resumeSkillSummary={resumeSkillSummary}
      />

      <div className="flex gap-3">
        <Button variant="secondary" onClick={onBack} className="flex-1" size="lg">
          Back
        </Button>
        <Button
          onClick={onComplete}
          variant="success"
          className="flex-1"
          size="lg"
          loading={isSaving}
          loadingText="Saving setup"
        >
          Start Finding Jobs
        </Button>
      </div>
    </div>
  );
}
