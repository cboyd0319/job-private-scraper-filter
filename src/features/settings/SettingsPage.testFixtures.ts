import { DEFAULT_EXTERNAL_AI_CONFIG } from "./config/SettingsConfig";

export function makeConfig() {
  return {
    title_allowlist: [],
    title_blocklist: [],
    keywords_boost: ["rust"],
    keywords_exclude: [],
    location_preferences: {
      allow_remote: true,
      allow_hybrid: false,
      allow_onsite: false,
      cities: [],
    },
    salary_floor_usd: 100000,
    preferred_companies: [],
    blocked_companies: [],
    auto_refresh: { enabled: false, interval_minutes: 30 },
    alerts: {
      slack: { enabled: false },
      email: {
        enabled: false,
        smtp_server: "",
        smtp_port: 587,
        smtp_username: "",
        from_email: "",
        to_emails: [],
        use_starttls: true,
      },
      discord: { enabled: false },
      telegram: { enabled: false },
      teams: { enabled: false },
      desktop: {
        enabled: false,
        show_when_focused: false,
        play_sound: false,
      },
    },
    linkedin: {
      enabled: false,
      query: "",
      location: "",
      remote_only: false,
      limit: 25,
    },
    remoteok: { enabled: false, tags: [], limit: 25 },
    weworkremotely: { enabled: false, limit: 25 },
    builtin: { enabled: false, remote_only: false, limit: 25 },
    hn_hiring: { enabled: false, remote_only: false, limit: 25 },
    dice: {
      enabled: false,
      query: "",
      location: undefined as string | undefined,
      limit: 25,
    },
    yc_startup: { enabled: false, remote_only: false, limit: 25 },
    usajobs: {
      enabled: false,
      email: "",
      remote_only: false,
      date_posted_days: 7,
      limit: 25,
    },
    simplyhired: {
      enabled: false,
      query: "",
      location: undefined as string | undefined,
      limit: 25,
    },
    glassdoor: {
      enabled: false,
      query: "",
      location: undefined as string | undefined,
      limit: 25,
    },
    restricted_source_acknowledgements: {
      builtin: false,
      dice: false,
      simplyhired: false,
      glassdoor: false,
    },
    jobswithgpt_endpoint: "",
    jobswithgpt_approval: {
      enabled: false,
      payload: null,
      approved_at: null,
    },
    external_ai: DEFAULT_EXTERNAL_AI_CONFIG,
    use_resume_matching: false,
  };
}

export function makeGhostConfig() {
  return {
    stale_threshold_days: 60,
    repost_threshold: 3,
    min_description_length: 200,
    penalize_missing_salary: false,
    warning_threshold: 0.3,
    hide_threshold: 0.7,
  };
}

export function makeSavedSearch() {
  return {
    id: "search-1",
    name: "Remote coordinator",
    sortBy: "score",
    scoreFilter: "all",
    sourceFilter: "all",
    remoteFilter: "all",
    bookmarkFilter: "all",
    notesFilter: "all",
    postedDateFilter: null,
    salaryMinFilter: 50000,
    salaryMaxFilter: null,
    ghostFilter: null,
    textSearch: "coordinator",
    createdAt: "2026-06-19T12:00:00Z",
    lastUsedAt: null,
  };
}
