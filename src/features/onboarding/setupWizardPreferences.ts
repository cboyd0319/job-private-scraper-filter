import { getSearchSourceDefaults } from "../../shared/jobSourceRecommendations";
export { COMMON_STARTER_JOB_TITLES, COMMON_WORK_TO_AVOID } from "./setupWizardTaxonomy";
import type {
  FreshnessOption,
  FreshnessPreference,
  GhostConfig,
  LocationPreferences,
  ReviewVolumeOption,
  ReviewVolumePreference,
  SetupConfig,
  SetupPayUnit,
  SetupResumeSkill,
  SetupSearchSummary,
  SuggestedJobSourceOption,
} from "./setupWizardTypes";
export type {
  FreshnessPreference,
  ReviewVolumePreference,
  SetupConfig,
  SetupJobSourceKey,
  SetupPayUnit,
  SetupResumeSkill,
  SetupResumeSummary,
  SetupSearchSummary,
  SuggestedJobSourceOption,
} from "./setupWizardTypes";

export const DEFAULT_FRESHNESS_PREFERENCE: FreshnessPreference = "fresh_verified_first";
export const DEFAULT_REVIEW_VOLUME_PREFERENCE: ReviewVolumePreference = "balanced";
export const FULL_TIME_HOURS_PER_YEAR = 2080;

export const FRESHNESS_GHOST_CONFIGS = {
  fresh_verified_first: {
    stale_threshold_days: 30,
    repost_threshold: 2,
    min_description_length: 200,
    penalize_missing_salary: false,
    warning_threshold: 0.2,
    hide_threshold: 0.75,
  },
  balanced: {
    stale_threshold_days: 60,
    repost_threshold: 3,
    min_description_length: 200,
    penalize_missing_salary: false,
    warning_threshold: 0.3,
    hide_threshold: 0.7,
  },
  wide_search: {
    stale_threshold_days: 120,
    repost_threshold: 5,
    min_description_length: 100,
    penalize_missing_salary: false,
    warning_threshold: 0.5,
    hide_threshold: 0.85,
  },
} satisfies Record<FreshnessPreference, GhostConfig>;

export const FRESHNESS_OPTIONS: FreshnessOption[] = [
  {
    id: "fresh_verified_first",
    label: "Fresh and verified first",
    description: "Warn earlier when a posting looks old, reposted, or hard to verify.",
  },
  {
    id: "balanced",
    label: "Balanced",
    description: "Use normal posting-review alerts while keeping the list broad.",
  },
  {
    id: "wide_search",
    label: "Widest search",
    description: "Show more older postings and warn only when risk looks clearer.",
  },
];

export const REVIEW_VOLUME_CONFIGS = {
  focused: {
    immediate_alert_threshold: 0.92,
    remoteok_limit: 25,
    hn_hiring_limit: 50,
    weworkremotely_limit: 25,
    simplyhired_limit: 25,
  },
  balanced: {
    immediate_alert_threshold: 0.9,
    remoteok_limit: 50,
    hn_hiring_limit: 100,
    weworkremotely_limit: 50,
    simplyhired_limit: 50,
  },
  broad: {
    immediate_alert_threshold: 0.85,
    remoteok_limit: 75,
    hn_hiring_limit: 150,
    weworkremotely_limit: 75,
    simplyhired_limit: 75,
  },
} satisfies Record<ReviewVolumePreference, {
  immediate_alert_threshold: number;
  remoteok_limit: number;
  hn_hiring_limit: number;
  weworkremotely_limit: number;
  simplyhired_limit: number;
}>;

export const REVIEW_VOLUME_OPTIONS: ReviewVolumeOption[] = [
  {
    id: "focused",
    label: "Smaller list",
    description: "Show fewer jobs and focus alerts on roles that most clearly fit your search.",
  },
  {
    id: "balanced",
    label: "Balanced list",
    description: "Recommended. Keep a manageable list without hiding useful roles.",
  },
  {
    id: "broad",
    label: "Broad discovery",
    description: "Show more possible roles, including adjacent ones that may still be worth a look.",
  },
];

export function ghostConfigForFreshnessPreference(
  preference: FreshnessPreference
): GhostConfig {
  return { ...FRESHNESS_GHOST_CONFIGS[preference] };
}

export function createDefaultSetupConfig(
  freshnessPreference: FreshnessPreference = DEFAULT_FRESHNESS_PREFERENCE,
  reviewVolumePreference: ReviewVolumePreference = DEFAULT_REVIEW_VOLUME_PREFERENCE
): SetupConfig {
  return {
    title_allowlist: [],
    title_blocklist: [],
    keywords_boost: [],
    keywords_exclude: [],
    location_preferences: {
      allow_remote: true,
      allow_hybrid: true,
      allow_onsite: true,
      cities: [],
    },
    salary_floor_usd: 0,
    alerts: {
      slack: {
        enabled: false,
        webhook_url: "",
      },
      desktop: {
        enabled: false,
        play_sound: false,
        show_when_focused: false,
      },
    },
    immediate_alert_threshold:
      REVIEW_VOLUME_CONFIGS[reviewVolumePreference].immediate_alert_threshold,
    ghost_config: ghostConfigForFreshnessPreference(freshnessPreference),
    remoteok: {
      enabled: false,
      tags: [],
      limit: REVIEW_VOLUME_CONFIGS[reviewVolumePreference].remoteok_limit,
    },
    hn_hiring: {
      enabled: false,
      remote_only: false,
      limit: REVIEW_VOLUME_CONFIGS[reviewVolumePreference].hn_hiring_limit,
    },
    weworkremotely: {
      enabled: false,
      limit: REVIEW_VOLUME_CONFIGS[reviewVolumePreference].weworkremotely_limit,
    },
    simplyhired: {
      enabled: false,
      query: "",
      limit: REVIEW_VOLUME_CONFIGS[reviewVolumePreference].simplyhired_limit,
    },
  };
}

export function freshnessSummary(preference: FreshnessPreference) {
  switch (preference) {
    case "fresh_verified_first":
      return "Fresh and verified first";
    case "balanced":
      return "Balanced";
    case "wide_search":
      return "Widest search";
  }
}

export function reviewVolumeSummary(preference: ReviewVolumePreference) {
  switch (preference) {
    case "focused":
      return "Smaller list";
    case "balanced":
      return "Balanced list";
    case "broad":
      return "Broad discovery";
  }
}

export function formatListSummary(items: string[], emptyText: string) {
  return items.length > 0 ? items.join(", ") : emptyText;
}

export function formatLocationSummary(locationPreferences: LocationPreferences) {
  const workTypes = [
    locationPreferences.allow_remote ? "remote" : null,
    locationPreferences.allow_hybrid ? "hybrid" : null,
    locationPreferences.allow_onsite ? "on-site" : null,
  ].filter(Boolean);

  const workTypeSummary = workTypes.length > 0 ? workTypes.join(", ") : "no work type selected";
  const citySummary =
    locationPreferences.cities.length > 0
      ? ` near ${locationPreferences.cities.join(", ")}`
      : "";

  return `${workTypeSummary}${citySummary}`;
}

export function formatJobSourceSummary(
  config: Pick<SetupConfig, "remoteok" | "hn_hiring" | "weworkremotely">
): string {
  const sources = [
    config.remoteok.enabled ? "Remote OK" : null,
    config.weworkremotely.enabled ? "We Work Remotely" : null,
    config.hn_hiring.enabled ? "Startup and tech hiring posts" : null,
  ].filter((source): source is string => source !== null);

  if (sources.length === 0) {
    return "No outside job sources selected; add reviewed sources in Settings.";
  }

  return `${sources.join(", ")} selected.`;
}

export function normalizeSetupPayFloorUsd(
  value: string,
  unit: SetupPayUnit,
): number {
  const parsed = Number.parseFloat(value);

  if (!Number.isFinite(parsed) || parsed <= 0) {
    return 0;
  }

  return Math.round(
    unit === "hourly" ? parsed * FULL_TIME_HOURS_PER_YEAR : parsed,
  );
}

function formatCurrency(value: number): string {
  return `$${value.toLocaleString(undefined, {
    maximumFractionDigits: 0,
  })}`;
}

function formatHourlyCurrency(value: number): string {
  return `$${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
  })}`;
}

export function formatSetupPayFloorSummary(
  annualPayFloorUsd: number,
  unit: SetupPayUnit = "yearly",
): string {
  if (annualPayFloorUsd <= 0) {
    return "Show jobs even when pay is missing or not listed";
  }

  if (unit === "hourly") {
    const hourlyPay = annualPayFloorUsd / FULL_TIME_HOURS_PER_YEAR;
    return `At least ${formatHourlyCurrency(hourlyPay)}/hour, about ${formatCurrency(annualPayFloorUsd)}/year`;
  }

  return `At least ${formatCurrency(annualPayFloorUsd)}/year`;
}

export function getSuggestedJobSourceOptions(
  config: Pick<SetupConfig, "title_allowlist" | "keywords_boost" | "location_preferences">
): SuggestedJobSourceOption[] {
  const sourceDefaults = getSearchSourceDefaults({
    titles: config.title_allowlist,
    keywords: config.keywords_boost,
    allowRemote: config.location_preferences.allow_remote,
  });

  const sourceOptions: SuggestedJobSourceOption[] = [];
  if (sourceDefaults.remoteokEnabled) {
    sourceOptions.push({
      key: "remoteok",
      label: "Remote OK",
      description: "Remote roles, mostly tech and startup work.",
    });
  }

  if (sourceDefaults.weworkremotelyEnabled) {
    sourceOptions.push({
      key: "weworkremotely",
      label: "We Work Remotely",
      description: "Remote roles across tech, support, product, and marketing.",
    });
  }

  if (sourceDefaults.hnHiringEnabled) {
    sourceOptions.push({
      key: "hn_hiring",
      label: "Startup and tech job posts",
      description: "Public startup and tech hiring posts.",
    });
  }

  return sourceOptions;
}

export function toResumeSkillSuggestions(skills: SetupResumeSkill[]): string[] {
  const seen = new Set<string>();
  const suggestions: string[] = [];

  for (const skill of skills) {
    if (skill.source !== "resume") continue;

    const name = skill.skill_name.trim();
    const key = name.toLocaleLowerCase();

    if (!name || seen.has(key)) continue;

    seen.add(key);
    suggestions.push(name);

    if (suggestions.length >= 6) break;
  }

  return suggestions;
}

export function applyReviewVolumePreference(
  config: SetupConfig,
  preference: ReviewVolumePreference
): SetupConfig {
  const volume = REVIEW_VOLUME_CONFIGS[preference];
  return {
    ...config,
    immediate_alert_threshold: volume.immediate_alert_threshold,
    remoteok: {
      ...config.remoteok,
      limit: volume.remoteok_limit,
    },
    hn_hiring: {
      ...config.hn_hiring,
      limit: volume.hn_hiring_limit,
    },
    weworkremotely: {
      ...config.weworkremotely,
      limit: volume.weworkremotely_limit,
    },
    simplyhired: {
      ...config.simplyhired,
      limit: volume.simplyhired_limit,
    },
  };
}

export function buildSetupSearchSummary(
  config: SetupConfig,
  freshnessPreference: FreshnessPreference,
  reviewVolumePreference: ReviewVolumePreference,
  payUnit: SetupPayUnit = "yearly"
): SetupSearchSummary {
  return {
    titles: config.title_allowlist.join(", "),
    wantedWork: formatListSummary(config.keywords_boost, "No extra work preferences yet"),
    avoidedWork: formatListSummary(config.keywords_exclude, "Nothing selected"),
    location: formatLocationSummary(config.location_preferences),
    freshness: freshnessSummary(freshnessPreference),
    reviewVolume: reviewVolumeSummary(reviewVolumePreference),
    jobSources: formatJobSourceSummary(config),
    alerts: !config.alerts.desktop.enabled
      ? "Desktop alerts off; add alerts later in Settings"
      : config.alerts.desktop.play_sound
        ? "Desktop alerts with sound"
        : "Quiet desktop alerts; no sound",
    pay: formatSetupPayFloorSummary(config.salary_floor_usd, payUnit),
  };
}
