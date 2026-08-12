import {
  mockApplications,
  mockConfig,
  mockJobs,
  mockPendingReminders,
  mockUpcomingInterviews,
} from "./data";
import {
  getNextMockCoverLetterTemplateId,
  normalizeMockCoverLetterTemplate,
} from "../features/applications/coverLetterTemplateCommands";
import {
  normalizeInterviewFollowUpState,
  normalizeInterviewPrepState,
} from "../features/applications/interviewProgress";
import {
  getNextMockSavedSearchId,
  normalizeMockSavedSearch,
} from "../features/dashboard/savedSearchCommands";
import {
  getDefaultMockApplicationProfile,
  getDefaultMockScreeningAnswers,
  normalizeMockApplicationProfile,
  normalizeMockScreeningAnswer,
} from "../../features/application-assist/mockProfile";
import { getDefaultMarketAlerts } from "../../features/market/mockHandlers";
import type { MockPendingUrlImport } from "../features/dashboard/jobImportCommands";
import { normalizeResumeDraft } from "../features/resumes/resumeBuilder";
import type { MockPendingMilitaryTransitionReview } from "../features/resumes/resumeCommandTypes";
import { normalizeMockNotificationPreferences } from "../features/settings/notificationCommands";
import {
  cloneApplications,
  getDefaultGhostConfig,
  normalizeApplications,
} from "./handlers/commandHelpers";
import type { MockCredentialUnlockState, MockState } from "./handlers/types";

const MOCK_STATE_KEY = "jobsentinel.mockState.v1";
const defaultCredentialUnlock: MockCredentialUnlockState = {
  mode: "system",
  configured: false,
  unlocked: true,
};

export interface MockExternalAiApproval {
  approvalId: string;
  request: Record<string, unknown>;
  providerId: string;
  model: string;
}

export interface MockExternalAiState {
  pendingExternalAiApproval: MockExternalAiApproval | null;
}

export interface MockRecoveryState {
  stagedRestoreStatus: "none" | "ready";
}

export interface MockRuntimeState
  extends MockState,
    MockExternalAiState,
    MockRecoveryState {
  automationBrowserRunning: boolean;
  nextAutomationAttemptId: number;
  pendingUrlImports: MockPendingUrlImport[];
  pendingMilitaryTransitionReviews: MockPendingMilitaryTransitionReview[];
}

function canUseStorage(): boolean {
  return (
    typeof window !== "undefined" && typeof window.localStorage !== "undefined"
  );
}

function isMockCredentialUnlockState(
  value: unknown,
): value is MockCredentialUnlockState {
  if (!value || typeof value !== "object") return false;

  const state = value as Partial<MockCredentialUnlockState>;
  return (
    (state.mode === "system" || state.mode === "passphrase") &&
    typeof state.configured === "boolean" &&
    typeof state.unlocked === "boolean"
  );
}

function createDefaultState(): MockRuntimeState {
  return {
    jobs: [...mockJobs],
    config: { ...mockConfig },
    interviews: [...mockUpcomingInterviews],
    applications: cloneApplications(mockApplications),
    pendingReminders: [...mockPendingReminders],
    coverLetterTemplates: [],
    savedSearches: [],
    searchHistory: [],
    notificationPreferences: null,
    credentials: {},
    credentialUnlock: { ...defaultCredentialUnlock },
    ghostConfig: getDefaultGhostConfig(),
    bookmarkletConfig: { port: 4321, enabled: false },
    pendingBookmarkletImports: [],
    pendingUrlImports: [],
    resumes: [],
    userSkills: [],
    resumeDrafts: [],
    recentMatches: [],
    savedMatchEvidence: {},
    pendingMilitaryTransitionReviews: [],
    marketAlerts: getDefaultMarketAlerts(),
    applicationProfile: getDefaultMockApplicationProfile(),
    screeningAnswers: getDefaultMockScreeningAnswers(),
    scraperEnabledOverrides: {},
    interviewPrepChecklists: {},
    interviewFollowups: {},
    linkedinWorkbenchReviewed: false,
    pendingExternalAiApproval: null,
    stagedRestoreStatus: "none",
    automationBrowserRunning: false,
    nextAutomationAttemptId: 1,
  };
}

export const mockRuntimeState = createDefaultState();

export function saveMockState(): void {
  if (!canUseStorage()) return;

  const persistedState: Partial<MockRuntimeState> = { ...mockRuntimeState };
  delete persistedState.automationBrowserRunning;
  delete persistedState.nextAutomationAttemptId;
  delete persistedState.pendingUrlImports;
  delete persistedState.pendingExternalAiApproval;
  delete persistedState.pendingMilitaryTransitionReviews;
  window.localStorage.setItem(MOCK_STATE_KEY, JSON.stringify(persistedState));
}

export function loadMockState(): void {
  if (!canUseStorage()) return;

  const rawState = window.localStorage.getItem(MOCK_STATE_KEY);
  if (!rawState) return;

  try {
    const state = JSON.parse(rawState) as Partial<MockRuntimeState>;
    if (Array.isArray(state.jobs)) mockRuntimeState.jobs = state.jobs;
    if (state.config && typeof state.config === "object") {
      mockRuntimeState.config = { ...mockConfig, ...state.config };
    }
    if (Array.isArray(state.interviews))
      mockRuntimeState.interviews = state.interviews;
    if (state.applications && typeof state.applications === "object") {
      mockRuntimeState.applications = normalizeApplications(state.applications);
    }
    if (Array.isArray(state.pendingReminders)) {
      mockRuntimeState.pendingReminders = state.pendingReminders;
    }
    if (Array.isArray(state.coverLetterTemplates)) {
      mockRuntimeState.coverLetterTemplates = state.coverLetterTemplates.map(
        (template) =>
          normalizeMockCoverLetterTemplate(
            template,
            getNextMockCoverLetterTemplateId(
              mockRuntimeState.coverLetterTemplates,
            ),
          ),
      );
    }
    if (Array.isArray(state.savedSearches)) {
      mockRuntimeState.savedSearches = state.savedSearches.map((search) =>
        normalizeMockSavedSearch(
          search,
          getNextMockSavedSearchId(mockRuntimeState.savedSearches),
        ),
      );
    }
    if (Array.isArray(state.searchHistory)) {
      mockRuntimeState.searchHistory = state.searchHistory.filter(
        (query): query is string =>
          typeof query === "string" && query.trim().length >= 2,
      );
    }
    if (
      state.notificationPreferences &&
      typeof state.notificationPreferences === "object"
    ) {
      mockRuntimeState.notificationPreferences =
        normalizeMockNotificationPreferences(state.notificationPreferences);
    }
    if (state.credentials && typeof state.credentials === "object") {
      mockRuntimeState.credentials = state.credentials;
    }
    if (isMockCredentialUnlockState(state.credentialUnlock)) {
      mockRuntimeState.credentialUnlock = state.credentialUnlock;
    }
    if (state.ghostConfig && typeof state.ghostConfig === "object") {
      mockRuntimeState.ghostConfig = {
        ...getDefaultGhostConfig(),
        ...state.ghostConfig,
      };
    }
    if (
      state.bookmarkletConfig &&
      typeof state.bookmarkletConfig === "object"
    ) {
      mockRuntimeState.bookmarkletConfig = {
        ...mockRuntimeState.bookmarkletConfig,
        ...state.bookmarkletConfig,
      };
    }
    if (Array.isArray(state.pendingBookmarkletImports)) {
      mockRuntimeState.pendingBookmarkletImports =
        state.pendingBookmarkletImports;
    }
    if (Array.isArray(state.resumes)) mockRuntimeState.resumes = state.resumes;
    if (Array.isArray(state.userSkills))
      mockRuntimeState.userSkills = state.userSkills;
    if (Array.isArray(state.resumeDrafts)) {
      mockRuntimeState.resumeDrafts = state.resumeDrafts
        .filter((draft) => draft && typeof draft === "object")
        .map((draft) => normalizeResumeDraft(draft));
    }
    if (Array.isArray(state.recentMatches))
      mockRuntimeState.recentMatches = state.recentMatches;
    if (state.savedMatchEvidence && typeof state.savedMatchEvidence === "object") {
      mockRuntimeState.savedMatchEvidence = state.savedMatchEvidence;
    }
    if (Array.isArray(state.marketAlerts))
      mockRuntimeState.marketAlerts = state.marketAlerts;
    if ("applicationProfile" in state) {
      mockRuntimeState.applicationProfile =
        state.applicationProfile && typeof state.applicationProfile === "object"
          ? normalizeMockApplicationProfile(state.applicationProfile)
          : null;
    }
    if (Array.isArray(state.screeningAnswers)) {
      mockRuntimeState.screeningAnswers = state.screeningAnswers
        .filter((answer) => answer && typeof answer === "object")
        .map((answer) => normalizeMockScreeningAnswer(answer));
    }
    if (
      state.scraperEnabledOverrides &&
      typeof state.scraperEnabledOverrides === "object"
    ) {
      mockRuntimeState.scraperEnabledOverrides = state.scraperEnabledOverrides;
    }
    if (
      state.interviewPrepChecklists &&
      typeof state.interviewPrepChecklists === "object"
    ) {
      mockRuntimeState.interviewPrepChecklists = normalizeInterviewPrepState(
        state.interviewPrepChecklists,
      );
    }
    if (
      state.interviewFollowups &&
      typeof state.interviewFollowups === "object"
    ) {
      mockRuntimeState.interviewFollowups = normalizeInterviewFollowUpState(
        state.interviewFollowups,
      );
    }
    if (typeof state.linkedinWorkbenchReviewed === "boolean") {
      mockRuntimeState.linkedinWorkbenchReviewed =
        state.linkedinWorkbenchReviewed;
    }
    if (
      state.stagedRestoreStatus === "none" ||
      state.stagedRestoreStatus === "ready"
    ) {
      mockRuntimeState.stagedRestoreStatus = state.stagedRestoreStatus;
    }
  } catch {
    window.localStorage.removeItem(MOCK_STATE_KEY);
  }
}

export function resetMockState(): void {
  const defaults = createDefaultState();
  Object.assign(mockRuntimeState, {
    jobs: defaults.jobs,
    config: defaults.config,
    interviews: defaults.interviews,
    applications: defaults.applications,
    pendingReminders: defaults.pendingReminders,
    coverLetterTemplates: defaults.coverLetterTemplates,
    savedSearches: defaults.savedSearches,
    searchHistory: defaults.searchHistory,
    notificationPreferences: defaults.notificationPreferences,
    credentials: defaults.credentials,
    credentialUnlock: defaults.credentialUnlock,
    ghostConfig: defaults.ghostConfig,
    bookmarkletConfig: defaults.bookmarkletConfig,
    pendingBookmarkletImports: defaults.pendingBookmarkletImports,
    pendingUrlImports: defaults.pendingUrlImports,
    resumes: defaults.resumes,
    userSkills: defaults.userSkills,
    resumeDrafts: defaults.resumeDrafts,
    recentMatches: defaults.recentMatches,
    savedMatchEvidence: defaults.savedMatchEvidence,
    pendingMilitaryTransitionReviews: defaults.pendingMilitaryTransitionReviews,
    marketAlerts: defaults.marketAlerts,
    applicationProfile: defaults.applicationProfile,
    screeningAnswers: defaults.screeningAnswers,
    linkedinWorkbenchReviewed: defaults.linkedinWorkbenchReviewed,
    pendingExternalAiApproval: defaults.pendingExternalAiApproval,
    stagedRestoreStatus: defaults.stagedRestoreStatus,
  });
  saveMockState();
}

loadMockState();
