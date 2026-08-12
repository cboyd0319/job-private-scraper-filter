/** Defines browser-development command state and fixture types. */

import type {
  mockApplications,
  mockConfig,
  mockJobs,
  mockPendingReminders,
} from "../data";
import type { NotificationPreferences } from "../../../shared/notificationPreferences";
import type {
  MockApplicationProfile,
  MockScreeningAnswer,
} from "../../../features/application-assist/mockProfile";
import type {
  PostedDateFilter,
  ScoreFilter,
  SortOption,
} from "../../../features/dashboard/types";
import type {
  MockInterviewFollowUpState,
  MockInterviewPrepState,
} from "../../features/applications/interviewProgress";
import type { MockMarketAlert } from "../../../features/market/mockHandlers";
import type { MockBuilderSkill, MockResumeDraft } from "../../features/resumes/resumeBuilder";
import type { MockScraperEnabledOverrides } from "../../features/settings/sources/scraperHealth";

export type MockJob = typeof mockJobs[number];
export type MockConfig = typeof mockConfig;
export type MockApplicationStatus = keyof typeof mockApplications;

export interface MockApplication {
  id: number;
  job_hash: string;
  job_title: string;
  company: string;
  status: string;
  applied_at: string | null;
  notes: string | null;
  last_contact: string | null;
}

export type MockApplications = Record<MockApplicationStatus, MockApplication[]>;
export type MockPendingReminder = typeof mockPendingReminders[number];

export type MockTemplateCategory =
  | "general"
  | "tech"
  | "creative"
  | "finance"
  | "healthcare"
  | "sales"
  | "custom"
  | "thankyou"
  | "followup"
  | "withdrawal";

export interface MockCoverLetterTemplate {
  id: string;
  name: string;
  content: string;
  category: MockTemplateCategory;
  createdAt: string;
  updatedAt: string;
}

export interface MockSavedSearch {
  id: string;
  name: string;
  sortBy: SortOption;
  scoreFilter: ScoreFilter;
  sourceFilter: string;
  remoteFilter: string;
  bookmarkFilter: string;
  notesFilter: string;
  postedDateFilter: PostedDateFilter | null;
  salaryMinFilter: number | null;
  salaryMaxFilter: number | null;
  ghostFilter: string | null;
  textSearch: string | null;
  createdAt: string;
  lastUsedAt: string | null;
}

export type MockCredentialKey =
  | "slack_webhook"
  | "smtp_password"
  | "discord_webhook"
  | "teams_webhook"
  | "telegram_bot_token"
  | "usajobs_api_key"
  | "external_ai_openai_api_key"
  | "external_ai_anthropic_api_key"
  | "external_ai_google_api_key"
  | "external_ai_github_copilot_api_key"
  | "external_ai_custom_api_key";

export interface MockCredentialUnlockState {
  mode: "system" | "passphrase";
  configured: boolean;
  unlocked: boolean;
}

export interface MockGhostConfig {
  stale_threshold_days: number;
  repost_threshold: number;
  min_description_length: number;
  penalize_missing_salary: boolean;
  warning_threshold: number;
  hide_threshold: number;
}

export interface MockBookmarkletConfig {
  port: number;
  enabled: boolean;
}

export interface MockPendingBookmarkletImport {
  id: string;
  title: string;
  company: string;
  url: string;
  location: string | null;
  description_preview: string | null;
  remote: boolean;
  received_at: string;
}

export interface MockResumeData {
  id: number;
  name: string;
  file_path: string;
  parsed_text: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export type MockResumeSummary = Omit<
  MockResumeData,
  "file_path" | "parsed_text"
> & {
  format_label: string;
  has_readable_text: boolean;
  readable_text_chars: number;
};

export interface MockResumeTextPreview {
  resume_id: number;
  name: string;
  has_text: boolean;
  text_preview: string;
  text_chars: number;
  is_truncated: boolean;
}

export interface MockUserSkill {
  id: number;
  resume_id: number;
  skill_name: string;
  skill_category: string | null;
  confidence_score: number;
  years_experience: number | null;
  proficiency_level: string | null;
  source: string;
}

export interface MockSkillInput {
  skill_name?: unknown;
  skill_category?: unknown;
  proficiency_level?: unknown;
  years_experience?: unknown;
}

export interface MockMatchResult {
  id: number;
  resume_id: number;
  job_hash: string;
  job_title: string;
  company: string;
  overall_match_score: number;
  skills_match_score: number | null;
  experience_match_score: number | null;
  education_match_score: number | null;
  matching_skills: string[];
  missing_skills: string[];
  gap_analysis: string | null;
  feedback: {
    match_id: number;
    label: "useful" | "not_relevant";
    recorded_at: string;
  } | null;
  created_at: string;
}

export interface MockInterview {
  id: number;
  application_id: number;
  job_hash: string;
  interview_type: string;
  scheduled_at: string;
  duration_minutes: number;
  location: string | null;
  interviewer_name: string | null;
  interviewer_title: string | null;
  notes: string | null;
  completed: boolean;
  outcome: string | null;
  post_interview_notes: string | null;
  job_title: string;
  company: string;
}

export interface MockDashboardPreferences {
  autoRefresh: MockConfig["auto_refresh"];
  salaryFloorUsd: number;
  anyJobSourceEnabled: boolean;
}

export interface MockFillResultWithAttempt {
  filledFields: string[];
  unfilledFields: string[];
  manualReviewTopics?: string[];
  captchaDetected: boolean;
  readyForReview: boolean;
  errorMessage: string | null;
  attemptId: number | null;
  durationMs: number;
  atsPlatform: string;
}

export interface MockState {
  jobs: MockJob[];
  config: MockConfig;
  interviews: MockInterview[];
  applications: MockApplications;
  pendingReminders: MockPendingReminder[];
  coverLetterTemplates: MockCoverLetterTemplate[];
  savedSearches: MockSavedSearch[];
  searchHistory: string[];
  notificationPreferences: NotificationPreferences | null;
  credentials: Partial<Record<MockCredentialKey, string>>;
  credentialUnlock: MockCredentialUnlockState;
  ghostConfig: MockGhostConfig;
  bookmarkletConfig: MockBookmarkletConfig;
  pendingBookmarkletImports: MockPendingBookmarkletImport[];
  resumes: MockResumeData[];
  userSkills: MockUserSkill[];
  resumeDrafts: MockResumeDraft[];
  recentMatches: MockMatchResult[];
  savedMatchEvidence: Record<string, MockSavedMatchEvidenceState>;
  marketAlerts: MockMarketAlert[];
  applicationProfile: MockApplicationProfile | null;
  screeningAnswers: MockScreeningAnswer[];
  scraperEnabledOverrides: MockScraperEnabledOverrides;
  interviewPrepChecklists: MockInterviewPrepState;
  interviewFollowups: MockInterviewFollowUpState;
  linkedinWorkbenchReviewed: boolean;
}

export interface MockSavedMatchEvidenceState {
  confirmedEvidenceIds: string[];
  confirmedMilitaryEvidenceKinds: Array<
    "military_service" | "current_clearance"
  >;
  packetClaims: Array<{
    claim_id: string;
    reviewed_text: string;
    evidence_ids: string[];
    boundaries: [
      "clearance_currentness_unverified",
      "military_civilian_equivalence_unverified",
    ];
  }>;
}

export type {
  MockApplicationProfile,
  MockBuilderSkill,
  MockInterviewFollowUpState,
  MockInterviewPrepState,
  MockMarketAlert,
  MockResumeDraft,
  MockScreeningAnswer,
  MockScraperEnabledOverrides,
  NotificationPreferences,
};
