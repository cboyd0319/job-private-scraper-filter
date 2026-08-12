import {
  getSuggestedJobSourceOptions,
  type SetupConfig,
  type SetupJobSourceKey,
  type SuggestedJobSourceOption,
} from "./setupWizardPreferences";

export interface SetupWizardSourceReviewOption extends SuggestedJobSourceOption {
  checked: boolean;
}

export function toggleSetupJobSource(
  config: SetupConfig,
  source: SetupJobSourceKey,
  enabled: boolean,
): SetupConfig {
  switch (source) {
    case "remoteok":
      return {
        ...config,
        remoteok: {
          ...config.remoteok,
          enabled,
        },
      };
    case "hn_hiring":
      return {
        ...config,
        hn_hiring: {
          ...config.hn_hiring,
          enabled,
        },
      };
    case "weworkremotely":
      return {
        ...config,
        weworkremotely: {
          ...config.weworkremotely,
          enabled,
        },
      };
  }
}

export function getSetupWizardSourceReviewOptions(
  config: SetupConfig,
): SetupWizardSourceReviewOption[] {
  return getSuggestedJobSourceOptions(config).map((source) => {
    switch (source.key) {
      case "remoteok":
        return { ...source, checked: config.remoteok.enabled };
      case "hn_hiring":
        return { ...source, checked: config.hn_hiring.enabled };
      case "weworkremotely":
        return { ...source, checked: config.weworkremotely.enabled };
    }
  });
}
