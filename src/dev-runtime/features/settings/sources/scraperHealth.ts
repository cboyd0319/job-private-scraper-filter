import { isPolicyBlockedScheduledSourceId } from "../../../../shared/restrictedSourceTaxonomy";

export type MockScraperEnabledOverrides = Record<string, boolean>;

type MockScraperType = "api" | "html" | "rss" | "graphql" | "hybrid";
type MockHealthStatus = "healthy" | "degraded" | "down" | "disabled" | "unknown";
type MockSelectorHealth = "healthy" | "degraded" | "broken" | "unknown";
type MockScraperRunStatus = "running" | "success" | "error" | "rate_limited";
type MockSourceRequestOutcome = "started" | "success" | "failure" | "timeout";

interface MockScraperDefinition {
  scraper_name: string;
  display_name: string;
  requires_auth: boolean;
  scraper_type: MockScraperType;
  rate_limit_per_hour: number;
}

interface MockScraperHealthMetrics extends MockScraperDefinition {
  is_enabled: boolean;
  health_status: MockHealthStatus;
  selector_health: MockSelectorHealth;
  success_rate_24h: number;
  avg_duration_ms: number | null;
  last_success: string | null;
  last_error: string | null;
  total_runs_24h: number;
  jobs_found_24h: number;
}

interface MockScraperRun {
  id: number;
  scraper_name: string;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
  status: MockScraperRunStatus;
  jobs_found: number;
  jobs_new: number;
  error_message: string | null;
  error_code: string | null;
  retry_attempt: number;
}

interface MockSmokeTestResult {
  scraper_name: string;
  test_type: "connectivity" | "selector" | "auth" | "rate_limit";
  passed: boolean;
  duration_ms: number;
  details: Record<string, unknown> | null;
  error: string | null;
}

interface MockSourceRequestSummary {
  id: number;
  source: string;
  sentAt: string;
  endpointHost: string | null;
  titleCount: number;
  hasLocation: boolean;
  remoteOnly: boolean;
  resultLimit: number;
  outcome: MockSourceRequestOutcome;
}

const MOCK_SCRAPERS: readonly MockScraperDefinition[] = [
  {
    scraper_name: "greenhouse",
    display_name: "Greenhouse",
    requires_auth: false,
    scraper_type: "api",
    rate_limit_per_hour: 60,
  },
  {
    scraper_name: "lever",
    display_name: "Lever",
    requires_auth: false,
    scraper_type: "api",
    rate_limit_per_hour: 60,
  },
  {
    scraper_name: "remoteok",
    display_name: "Remote OK",
    requires_auth: false,
    scraper_type: "api",
    rate_limit_per_hour: 120,
  },
  {
    scraper_name: "hn_hiring",
    display_name: "Startup and tech job posts",
    requires_auth: false,
    scraper_type: "html",
    rate_limit_per_hour: 30,
  },
  {
    scraper_name: "weworkremotely",
    display_name: "We Work Remotely",
    requires_auth: false,
    scraper_type: "rss",
    rate_limit_per_hour: 60,
  },
  {
    scraper_name: "indeed",
    display_name: "Indeed",
    requires_auth: false,
    scraper_type: "html",
    rate_limit_per_hour: 60,
  },
  {
    scraper_name: "wellfound",
    display_name: "Wellfound",
    requires_auth: true,
    scraper_type: "html",
    rate_limit_per_hour: 45,
  },
  {
    scraper_name: "jobswithgpt",
    display_name: "JobsWithGPT",
    requires_auth: false,
    scraper_type: "api",
    rate_limit_per_hour: 60,
  },
  {
    scraper_name: "ziprecruiter",
    display_name: "ZipRecruiter",
    requires_auth: false,
    scraper_type: "rss",
    rate_limit_per_hour: 45,
  },
  {
    scraper_name: "usajobs",
    display_name: "USAJOBS",
    requires_auth: true,
    scraper_type: "api",
    rate_limit_per_hour: 120,
  },
] as const;
const RESTRICTED_SMOKE_TEST_SOURCES = new Set([
  "indeed",
  "wellfound",
  "builtin",
  "dice",
  "ziprecruiter",
  "simplyhired",
  "glassdoor",
]);
const JOBSWITHGPT_PROVIDER_REVIEW_REASON =
  "JobsWithGPT provider endpoint and usage policy require review";

export function hasEnabledMockScraperSource(configRecord: Record<string, unknown>): boolean {
  return MOCK_SCRAPERS.some((scraper) =>
    hasEnabledMockSource(configRecord, scraper.scraper_name),
  );
}

export function hasConfiguredJobsWithGpt(configRecord: Record<string, unknown>): boolean {
  const endpoint = configRecord.jobswithgpt_endpoint;
  const titles = configRecord.title_allowlist;
  const approval = configRecord.jobswithgpt_approval;
  if (
    typeof endpoint !== "string" ||
    endpoint.trim().length === 0 ||
    !Array.isArray(titles)
  ) {
    return false;
  }

  const payloadTitles = titles
    .filter((title): title is string => typeof title === "string")
    .map((title) => title.trim())
    .filter((title) => title.length > 0);
  if (payloadTitles.length === 0 || !isRecord(approval)) return false;

  const payload = approval.payload;
  if (approval.enabled !== true || !isRecord(payload)) return false;
  const locationPreferences = configRecord.location_preferences;
  const remoteOnly =
    isRecord(locationPreferences) &&
    locationPreferences.allow_remote === true &&
    locationPreferences.allow_onsite !== true;

  return (
    payload.endpoint === endpoint.trim() &&
    Array.isArray(payload.titles) &&
    payload.titles.length === payloadTitles.length &&
    payload.titles.every((title, index) => title === payloadTitles[index]) &&
    (payload.location ?? null) === null &&
    payload.remote_only === remoteOnly &&
    payload.limit === 100
  );
}

export function getMockScraperHealth(
  scraperEnabledOverrides: MockScraperEnabledOverrides,
): MockScraperHealthMetrics[] {
  return MOCK_SCRAPERS.map((scraper, index) => {
    const isEnabled =
      scraper.scraper_name !== "jobswithgpt" &&
      (scraperEnabledOverrides[scraper.scraper_name] ?? true);
    const status: MockHealthStatus = isEnabled
      ? index % 7 === 0
        ? "degraded"
        : "healthy"
      : "disabled";
    return {
      ...scraper,
      is_enabled: isEnabled,
      health_status: status,
      selector_health:
        status === "disabled" ? "unknown" : status === "degraded" ? "degraded" : "healthy",
      success_rate_24h: status === "healthy" ? 96 : status === "degraded" ? 82 : 0,
      avg_duration_ms: isEnabled ? 850 + index * 75 : null,
      last_success: isEnabled
        ? new Date(Date.now() - (index + 1) * 600000).toISOString()
        : null,
      last_error: status === "degraded" ? "Selector fallback used" : null,
      total_runs_24h: isEnabled ? 12 : 0,
      jobs_found_24h: isEnabled ? 4 + index : 0,
    };
  });
}

export function getMockHealthSummary(
  scraperEnabledOverrides: MockScraperEnabledOverrides,
) {
  const health = getMockScraperHealth(scraperEnabledOverrides);
  return {
    total_scrapers: health.length,
    healthy: health.filter((scraper) => scraper.health_status === "healthy").length,
    degraded: health.filter((scraper) => scraper.health_status === "degraded").length,
    down: health.filter((scraper) => scraper.health_status === "down").length,
    disabled: health.filter((scraper) => scraper.health_status === "disabled").length,
    total_jobs_24h: health.reduce((sum, scraper) => sum + scraper.jobs_found_24h, 0),
  };
}

export function getMockScraperRuns(args?: Record<string, unknown>): MockScraperRun[] {
  const scraperName =
    getStringArg(args, "scraperName") ?? getStringArg(args, "scraper_name") ?? "greenhouse";
  const limit = getNumericArg(args, "limit") ?? 20;
  return Array.from({ length: limit }, (_, index) => {
    const startedAt = new Date(Date.now() - (index + 1) * 3600000).toISOString();
    return {
      id: index + 1,
      scraper_name: scraperName,
      started_at: startedAt,
      finished_at: new Date(Date.now() - (index + 1) * 3600000 + 900).toISOString(),
      duration_ms: 900 + index * 25,
      status: "success",
      jobs_found: 5 + index,
      jobs_new: 2,
      error_message: null,
      error_code: null,
      retry_attempt: 0,
    };
  });
}

export function getMockLatestSourceRequest(
  args: Record<string, unknown> | undefined,
  configRecord: Record<string, unknown>,
): MockSourceRequestSummary | null {
  const source = getStringArg(args, "source") ?? "jobswithgpt";
  if (source !== "jobswithgpt" || !hasConfiguredJobsWithGpt(configRecord)) {
    return null;
  }

  const titles = Array.isArray(configRecord.title_allowlist)
    ? configRecord.title_allowlist
    : [];
  const locationPreferences = isRecord(configRecord.location_preferences)
    ? configRecord.location_preferences
    : {};

  return {
    id: 1,
    source,
    sentAt: new Date(Date.now() - 3600000).toISOString(),
    endpointHost: "api.jobswithgpt.example",
    titleCount: titles.filter((title) => typeof title === "string" && title.trim().length > 0)
      .length,
    hasLocation: false,
    remoteOnly: Boolean(
      locationPreferences.allow_remote && !locationPreferences.allow_onsite,
    ),
    resultLimit: 100,
    outcome: "success",
  };
}

export function getMockSmokeTestResultForArgs(
  args: Record<string, unknown> | undefined,
  scraperEnabledOverrides: MockScraperEnabledOverrides,
): MockSmokeTestResult {
  const scraperName =
    getStringArg(args, "scraperName") ?? getStringArg(args, "scraper_name") ?? "greenhouse";
  return getMockSmokeTestResult(scraperName, scraperEnabledOverrides);
}

export function getAllMockSmokeTestResults(
  scraperEnabledOverrides: MockScraperEnabledOverrides,
): MockSmokeTestResult[] {
  return MOCK_SCRAPERS.map((scraper) =>
    getMockSmokeTestResult(scraper.scraper_name, scraperEnabledOverrides),
  );
}

export function updateMockScraperEnabled(
  args: Record<string, unknown> | undefined,
  scraperEnabledOverrides: MockScraperEnabledOverrides,
): MockScraperEnabledOverrides {
  const scraperName = getStringArg(args, "scraperName") ?? getStringArg(args, "scraper_name");
  if (!scraperName || isPolicyBlockedScheduledSourceId(scraperName)) {
    return scraperEnabledOverrides;
  }

  return {
    ...scraperEnabledOverrides,
    [scraperName]: booleanValue(getArg(args, "enabled"), true),
  };
}

function getMockSmokeTestResult(
  scraperName: string,
  scraperEnabledOverrides: MockScraperEnabledOverrides,
): MockSmokeTestResult {
  if (scraperName === "jobswithgpt") {
    return {
      scraper_name: scraperName,
      test_type: "connectivity",
      passed: true,
      duration_ms: 0,
      details: {
        status: "skipped",
        reason: JOBSWITHGPT_PROVIDER_REVIEW_REASON,
      },
      error: null,
    };
  }
  if (RESTRICTED_SMOKE_TEST_SOURCES.has(scraperName)) {
    return {
      scraper_name: scraperName,
      test_type: "connectivity",
      passed: true,
      duration_ms: 0,
      details: {
        status: "skipped",
        reason:
          "Automated access is unavailable after provider policy review. Use a user-opened search link, Browser Import, or manual entry.",
      },
      error: null,
    };
  }
  return {
    scraper_name: scraperName,
    test_type: "connectivity",
    passed: scraperEnabledOverrides[scraperName] !== false,
    duration_ms: 700,
    details: null,
    error: scraperEnabledOverrides[scraperName] === false ? "Scraper disabled" : null,
  };
}

function hasEnabledMockSource(configRecord: Record<string, unknown>, key: string): boolean {
  const value = configRecord[key];
  return (
    typeof value === "object" &&
    value !== null &&
    "enabled" in value &&
    (value as { enabled?: unknown }).enabled === true
  );
}

function getStringArg(
  args: Record<string, unknown> | undefined,
  key: string,
): string | undefined {
  const value = getArg(args, key);
  return typeof value === "string" ? value : undefined;
}

function getNumericArg(
  args: Record<string, unknown> | undefined,
  key: string,
): number | undefined {
  const value = getArg(args, key);
  if (typeof value === "number") return value;
  if (typeof value === "string") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : undefined;
  }
  return undefined;
}

function getArg(
  args: Record<string, unknown> | undefined,
  key: string,
): unknown {
  const nestedArgs = args?.payload as Record<string, unknown> | undefined;
  return args?.[key] ?? nestedArgs?.[key];
}

function booleanValue(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object";
}
