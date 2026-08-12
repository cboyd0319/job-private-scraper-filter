import type { ReactNode } from "react";
import type { RestrictedSourceAcknowledgements } from "../../../shared/restrictedSourceTaxonomy";
import {
  DEFAULT_EXTERNAL_AI_CONFIG,
  type ExternalAiProvider,
  type ExternalAiSettings,
} from "../external-ai/externalAiProviders";

// Ghost detection configuration interface
export interface GhostConfig {
  stale_threshold_days: number;
  repost_threshold: number;
  min_description_length: number;
  penalize_missing_salary: boolean;
  warning_threshold: number;
  hide_threshold: number;
}

export interface SettingsProps {
  onClose: () => void;
  initialTab?: "basic" | "advanced";
  linkedinWorkbench?: ReactNode;
}

export interface JobsWithGptPayload {
  endpoint: string;
  titles: string[];
  location?: string | null;
  remote_only: boolean;
  limit: number;
}

export interface JobsWithGptApproval {
  enabled: boolean;
  payload?: JobsWithGptPayload | null;
  approved_at?: string | null;
}

export type SourceRequestOutcome =
  "started" | "success" | "failure" | "timeout";

export interface SourceRequestSummary {
  id: number;
  source: string;
  sentAt: string;
  endpointHost?: string | null;
  titleCount: number;
  hasLocation: boolean;
  remoteOnly: boolean;
  resultLimit: number;
  outcome: SourceRequestOutcome;
}

// Config interface without sensitive credential fields (stored through secure storage)
export interface Config {
  title_allowlist: string[];
  title_blocklist: string[];
  keywords_boost: string[];
  keywords_exclude: string[];
  location_preferences: {
    allow_remote: boolean;
    allow_hybrid: boolean;
    allow_onsite: boolean;
    cities: string[];
  };
  salary_floor_usd: number;
  salary_target_usd?: number;
  preferred_companies: string[];
  blocked_companies: string[];
  auto_refresh: {
    enabled: boolean;
    interval_minutes: number;
  };
  bookmarklet_port?: number;
  alerts: {
    slack: {
      enabled: boolean;
      // webhook_url stored securely
    };
    email: {
      enabled: boolean;
      smtp_server: string;
      smtp_port: number;
      smtp_username: string;
      // smtp_password stored securely
      from_email: string;
      to_emails: string[];
      use_starttls: boolean;
    };
    discord: {
      enabled: boolean;
      // webhook_url stored securely
      user_id_to_mention?: string;
    };
    telegram: {
      enabled: boolean;
      // bot_token stored securely
      chat_id?: string;
    };
    teams: {
      enabled: boolean;
      // webhook_url stored securely
    };
    desktop: {
      enabled: boolean;
      show_when_focused: boolean;
      play_sound: boolean;
    };
  };
  linkedin: {
    enabled: boolean;
    // LinkedIn stays user-opened; no session cookie is stored.
    query: string;
    location: string;
    remote_only: boolean;
    limit: number;
  };
  restricted_source_acknowledgements: RestrictedSourceAcknowledgements;
  remoteok: {
    enabled: boolean;
    tags: string[];
    limit: number;
  };
  weworkremotely: {
    enabled: boolean;
    category?: string;
    limit: number;
  };
  builtin: {
    enabled: boolean;
    remote_only: boolean;
    limit: number;
  };
  hn_hiring: {
    enabled: boolean;
    remote_only: boolean;
    limit: number;
  };
  dice: {
    enabled: boolean;
    query: string;
    location?: string;
    limit: number;
  };
  yc_startup: {
    enabled: boolean;
    query?: string;
    remote_only: boolean;
    limit: number;
  };
  usajobs: {
    enabled: boolean;
    // api_key stored securely
    email: string;
    keywords?: string;
    location?: string;
    radius?: number;
    remote_only: boolean;
    pay_grade_min?: number;
    pay_grade_max?: number;
    date_posted_days: number;
    limit: number;
  };
  simplyhired: {
    enabled: boolean;
    query: string;
    location?: string;
    limit: number;
  };
  glassdoor: {
    enabled: boolean;
    query: string;
    location?: string;
    limit: number;
  };
  jobswithgpt_endpoint: string;
  jobswithgpt_approval: JobsWithGptApproval;
  external_ai: ExternalAiSettings;
  use_resume_matching: boolean;
}

export { DEFAULT_EXTERNAL_AI_CONFIG };
export type { ExternalAiProvider, ExternalAiSettings };

export type GhostPreset = "lenient" | "balanced" | "strict";
export type GhostPresetSelection = GhostPreset | "custom";

export const GHOST_PRESETS: GhostPreset[] = ["lenient", "balanced", "strict"];

export const GHOST_PRESET_LABELS: Record<GhostPreset, string> = {
  lenient: "Widest search",
  balanced: "Balanced",
  strict: "Fresh and verified first",
};

export const GHOST_PRESET_DESCRIPTIONS: Record<GhostPresetSelection, string> = {
  lenient:
    "Shows the broadest list and warns only on stronger stale-posting signals.",
  balanced:
    "Recommended. Keeps jobs visible while warning sooner when posting evidence is weak.",
  strict:
    "Warns sooner about old, reposted, missing-pay, or thin postings. Some legitimate jobs may need review.",
  custom:
    "Use detailed controls if you want to tune how early warnings appear.",
};

export function formatPostingRiskWarningLabel(value: number): string {
  if (value <= 0.2) return "Very early";
  if (value <= 0.35) return "Early";
  if (value <= 0.5) return "Balanced";
  return "Later";
}

export function formatPostingRiskHideLabel(value: number): string {
  if (value <= 0.55) return "Hide more flagged jobs";
  if (value <= 0.75) return "Balanced";
  return "Keep more visible";
}

export function buildJobsWithGptPayload(
  config: Config,
): JobsWithGptPayload | null {
  const endpoint = config.jobswithgpt_endpoint.trim();
  const titles = config.title_allowlist
    .map((title) => title.trim())
    .filter((title) => title.length > 0);

  if (!endpoint || titles.length === 0) {
    return null;
  }

  return {
    endpoint,
    titles,
    location: null,
    remote_only:
      config.location_preferences.allow_remote &&
      !config.location_preferences.allow_onsite,
    limit: 100,
  };
}

export function formatJobSourceSite(endpoint: string): string {
  try {
    return new URL(endpoint).host;
  } catch {
    return "Hidden until the link is valid";
  }
}

function sameStringArray(left: string[], right: string[]): boolean {
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function sameJobsWithGptPayload(
  left?: JobsWithGptPayload | null,
  right?: JobsWithGptPayload | null,
): boolean {
  if (!left || !right) return false;

  return (
    left.endpoint === right.endpoint &&
    sameStringArray(left.titles, right.titles) &&
    (left.location ?? null) === (right.location ?? null) &&
    left.remote_only === right.remote_only &&
    left.limit === right.limit
  );
}

export function isCurrentJobsWithGptPayloadApproved(
  config: Config,
  payload: JobsWithGptPayload | null,
): boolean {
  return (
    config.jobswithgpt_approval.enabled &&
    sameJobsWithGptPayload(config.jobswithgpt_approval.payload, payload)
  );
}

export const isValidEmail = (email: string): boolean => {
  if (!email) return true;
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
};
