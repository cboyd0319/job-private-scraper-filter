// Dashboard.tsx Type Definitions
// Extracted from Dashboard.tsx to reduce file size

import type { ComponentType, ReactNode } from "react";
import type { RenderCompanyResearch } from "../../shared/companyResearch";

export type SortOption =
  "score-desc" | "score-asc" | "date-desc" | "date-asc" | "company-asc";
export type ScoreFilter = "all" | "high" | "medium" | "low";
export type PostedDateFilter = "all" | "24h" | "7d" | "30d";
export type GhostFilter = "all" | "real" | "ghost";
export type Page =
  "dashboard" | "applications" | "resume" | "automation" | "salary" | "market";

export interface Job {
  id: number;
  hash?: string;
  title: string;
  company: string;
  location: string | null;
  url: string;
  source: string;
  score: number | null;
  score_reasons?: string | null;
  created_at: string;
  description?: string | null;
  salary_min?: number | null;
  salary_max?: number | null;
  remote?: boolean | null;
  bookmarked?: boolean;
  notes?: string | null;
  // Ghost detection fields (v1.4)
  ghost_score?: number | null;
  ghost_reasons?: string | null;
  times_seen?: number;
}

export interface Statistics {
  total_jobs: number;
  high_matches: number;
  average_score: number;
}

export interface ScrapingStatus {
  last_scrape: string | null;
  next_scrape: string | null;
  is_running: boolean;
}

export interface SavedSearch {
  id: string;
  name: string;
  filters: {
    sortBy: SortOption;
    scoreFilter: ScoreFilter;
    sourceFilter: string;
    remoteFilter: string;
    bookmarkFilter: string;
    notesFilter: string;
    postedDateFilter?: PostedDateFilter;
    ghostFilter?: GhostFilter;
    salaryMinFilter?: number | null;
    salaryMaxFilter?: number | null;
  };
  createdAt: string;
}

export interface DuplicateGroup {
  primary_id: number;
  jobs: Job[];
  sources: string[];
}

export interface AutoRefreshConfig {
  enabled: boolean;
  interval_minutes: number;
}

export interface DashboardPreferences {
  autoRefresh: AutoRefreshConfig;
  salaryFloorUsd: number | null;
  anyJobSourceEnabled: boolean;
}

export interface DashboardProps {
  onNavigate?: (page: Page) => void;
  tourAction?: ReactNode;
  renderApplicationAssistAction?: (
    job: Job,
    onOpenApplicationAssist: () => void,
  ) => ReactNode;
  renderCompanyResearch?: RenderCompanyResearch;
  settingsPage?: ComponentType<{
    onClose: () => void;
    initialTab?: "basic" | "advanced";
    linkedinWorkbench?: ReactNode;
  }>;
  settingsInitialTab?: "basic" | "advanced";
  linkedinWorkbench?: ReactNode;
  showSettings?: boolean;
  onShowSettingsChange?: (show: boolean) => void;
  openImportOnMount?: boolean;
  onImportHandled?: () => void;
}

export interface SearchQuery {
  includes: string[];
  excludes: string[];
  isOr: boolean;
}
