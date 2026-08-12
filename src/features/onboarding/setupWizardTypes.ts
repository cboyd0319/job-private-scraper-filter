export interface LocationPreferences {
  allow_remote: boolean;
  allow_hybrid: boolean;
  allow_onsite: boolean;
  cities: string[];
}

export interface GhostConfig {
  stale_threshold_days: number;
  repost_threshold: number;
  min_description_length: number;
  penalize_missing_salary: boolean;
  warning_threshold: number;
  hide_threshold: number;
}

export interface SetupQuerySourceConfig {
  enabled: boolean;
  query: string;
  location?: string;
  limit: number;
}

export type FreshnessPreference =
  | "fresh_verified_first"
  | "balanced"
  | "wide_search";

export interface FreshnessOption {
  id: FreshnessPreference;
  label: string;
  description: string;
}

export type ReviewVolumePreference =
  | "focused"
  | "balanced"
  | "broad";

export type SetupPayUnit = "yearly" | "hourly";

export interface ReviewVolumeOption {
  id: ReviewVolumePreference;
  label: string;
  description: string;
}

export interface SetupResumeSummary {
  id: number;
  name: string;
}

export interface SetupResumeSkill {
  skill_name: string;
  source?: string | null;
}

export interface SetupConfig {
  title_allowlist: string[];
  title_blocklist: string[];
  keywords_boost: string[];
  keywords_exclude: string[];
  location_preferences: LocationPreferences;
  salary_floor_usd: number;
  alerts: {
    slack: {
      enabled: boolean;
      webhook_url: string;
    };
    desktop: {
      enabled: boolean;
      play_sound: boolean;
      show_when_focused: boolean;
    };
  };
  immediate_alert_threshold?: number;
  ghost_config: GhostConfig;
  remoteok: {
    enabled: boolean;
    tags: string[];
    limit: number;
  };
  hn_hiring: {
    enabled: boolean;
    remote_only: boolean;
    limit: number;
  };
  weworkremotely: {
    enabled: boolean;
    limit: number;
  };
  simplyhired: SetupQuerySourceConfig;
}

export interface SetupSearchSummary {
  titles: string;
  wantedWork: string;
  avoidedWork: string;
  location: string;
  freshness: string;
  reviewVolume: string;
  jobSources: string;
  alerts: string;
  pay: string;
}

export type SetupJobSourceKey =
  | "remoteok"
  | "weworkremotely"
  | "hn_hiring";

export interface SuggestedJobSourceOption {
  key: SetupJobSourceKey;
  label: string;
  description: string;
}
