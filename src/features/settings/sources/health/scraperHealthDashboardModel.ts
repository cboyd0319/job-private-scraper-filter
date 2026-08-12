import { getUserFriendlyError } from "../../../../shared/errorReporting/messages";

export interface ScraperHealthMetrics {
  scraper_name: string;
  display_name: string;
  is_enabled: boolean;
  requires_auth: boolean;
  scraper_type: "api" | "html" | "rss" | "graphql" | "hybrid";
  health_status: "healthy" | "degraded" | "down" | "disabled" | "unknown";
  selector_health: "healthy" | "degraded" | "broken" | "unknown";
  success_rate_24h: number;
  avg_duration_ms: number | null;
  last_success: string | null;
  last_error: string | null;
  total_runs_24h: number;
  jobs_found_24h: number;
  rate_limit_per_hour: number;
}

export interface HealthSummary {
  total_scrapers: number;
  healthy: number;
  degraded: number;
  down: number;
  disabled: number;
  total_jobs_24h: number;
}

export interface ScraperRun {
  id: number;
  scraper_name: string;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
  status: "running" | "success" | "error" | "rate_limited";
  jobs_found: number;
  jobs_new: number;
  error_message: string | null;
  error_code: string | null;
  retry_attempt: number;
}

export interface SmokeTestResult {
  scraper_name: string;
  test_type: "connectivity" | "selector" | "auth" | "rate_limit";
  passed: boolean;
  duration_ms: number;
  details: Record<string, unknown> | null;
  error: string | null;
}

export const healthStatusConfig = {
  healthy: { variant: "success" as const, label: "Working", icon: "check" },
  degraded: { variant: "alert" as const, label: "Having trouble", icon: "warning" },
  down: { variant: "danger" as const, label: "Not working", icon: "x" },
  disabled: { variant: "surface" as const, label: "Off", icon: "minus" },
  unknown: { variant: "surface" as const, label: "Not checked", icon: "question" },
};

export const selectorHealthConfig = {
  healthy: { variant: "success" as const, label: "Yes" },
  degraded: { variant: "alert" as const, label: "Having trouble" },
  broken: { variant: "danger" as const, label: "Cannot read jobs" },
  unknown: { variant: "surface" as const, label: "No action needed" },
};

export function formatDuration(ms: number | null): string {
  if (ms === null) return "Not checked yet";
  if (ms < 1000) return "under 1s";
  return `${(ms / 1000).toFixed(1)}s`;
}

export function formatRelativeTime(dateStr: string | null): string {
  if (!dateStr) return "Never";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

export function formatSourceType(type: ScraperHealthMetrics["scraper_type"]): string {
  switch (type) {
    case "api":
    case "graphql":
      return "Official source";
    case "rss":
      return "Public job list";
    case "html":
    case "hybrid":
      return "Website page";
  }
}

export function getRecentStatus(scraper: ScraperHealthMetrics) {
  if (!scraper.is_enabled) {
    return { label: "Off", className: "text-surface-500" };
  }

  if (scraper.total_runs_24h === 0) {
    return { label: "Not checked yet", className: "text-surface-500" };
  }

  if (scraper.success_rate_24h >= 90) {
    return { label: "Mostly working", className: "text-green-600 dark:text-green-400" };
  }

  if (scraper.success_rate_24h >= 70) {
    return { label: "Some trouble", className: "text-yellow-600 dark:text-yellow-400" };
  }

  return { label: "Needs attention", className: "text-red-600 dark:text-red-400" };
}

export function formatSourceNextStep(scraper: ScraperHealthMetrics): string {
  if (scraper.scraper_name === "jobswithgpt") {
    return "Provider endpoint and usage policy review pending.";
  }

  if (!scraper.is_enabled) {
    return "Off. Turn on if useful.";
  }

  if (scraper.requires_auth && scraper.health_status !== "healthy") {
    return "Update connection in Settings if this keeps happening.";
  }

  if (scraper.health_status === "down") {
    return "Try again later or turn this source off.";
  }

  if (scraper.health_status === "degraded" || scraper.success_rate_24h < 70) {
    return "Try again later. Use search links if urgent.";
  }

  if (
    scraper.jobs_found_24h === 0 &&
    scraper.total_runs_24h > 0 &&
    scraper.health_status === "healthy"
  ) {
    return "Adjust search words or use search links.";
  }

  return "Working. No action needed.";
}

export function formatRunStatus(status: ScraperRun["status"], retryAttempt: number): string {
  const labels: Record<ScraperRun["status"], string> = {
    error: "Problem found",
    rate_limited: "Waiting",
    running: "Checking",
    success: "Worked",
  };
  const label = labels[status];
  return retryAttempt > 0 ? `${label} after another try` : label;
}

export function formatSafeIssue(message: string | null): string {
  if (!message) {
    return "-";
  }

  const friendly = getUserFriendlyError(message);
  return friendly.action ?? friendly.message;
}
