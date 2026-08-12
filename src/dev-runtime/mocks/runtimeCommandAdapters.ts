/** Adapts desktop command names to deterministic browser-development responses. */

import { handleMockApplicationAssistCommand } from "../features/application-assist/commands";
import { handleMockApplicationsCommand } from "../features/applications/commands";
import { handleMockCoverLetterTemplateCommand } from "../features/applications/coverLetterTemplateCommands";
import { handleMockInterviewCommand } from "../features/applications/interviewCommands";
import { handleMockDashboardCommand } from "../features/dashboard/commands";
import { handleMockJobImportCommand } from "../features/dashboard/jobImportCommands";
import { handleMockSavedSearchCommand } from "../features/dashboard/savedSearchCommands";
import { handleMockLinkedInWorkbenchCommand } from "../features/linkedin-workbench/commands";
import { handleMockMarketCommand } from "../features/market/commands";
import { handleMockOpportunityCaseCommand } from "../features/opportunity-case/commands";
import { handleMockOnboardingCommand } from "../features/onboarding/commands";
import {
  getMockActiveResume,
  handleMockResumeCommand,
} from "../features/resumes/resumeCommands";
import { handleMockSalaryCommand } from "../features/salary/commands";
import { handleMockSearchLinksCommand } from "../features/search-links/commands";
import { handleMockSettingsCommand } from "../features/settings/commands";
import { handleMockExternalAiCommand } from "../features/settings/externalAiCommands";
import { handleMockNotificationCommand } from "../features/settings/notificationCommands";
import { handleMockPackCommand } from "../features/settings/packCommands";
import { handleMockSourceHealthCommand } from "../features/settings/sources/commands";
import { handleMockSupportCommand } from "../features/settings/support/commands";
import { handleMockRecoveryCommand } from "../features/settings/support/recoveryCommands";
import { mockRuntimeState, saveMockState } from "./runtimeState";

export type MockCommandAdapter = (
  command: string,
  args: Record<string, unknown> | undefined,
) => unknown;

export const applyMockDashboardCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockDashboardCommand(command, args, {
    jobs: mockRuntimeState.jobs,
  });
  if (!result.handled) return undefined;
  mockRuntimeState.jobs = result.state.jobs;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockOnboardingCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockOnboardingCommand(command, args, {
    config: mockRuntimeState.config,
  });
  if (!result.handled) return undefined;
  mockRuntimeState.config = result.state.config;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockSettingsCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockSettingsCommand(command, args, {
    config: mockRuntimeState.config,
    credentials: mockRuntimeState.credentials,
    credentialUnlock: mockRuntimeState.credentialUnlock,
    ghostConfig: mockRuntimeState.ghostConfig,
    bookmarkletConfig: mockRuntimeState.bookmarkletConfig,
    pendingBookmarkletImports: mockRuntimeState.pendingBookmarkletImports,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockPackCommand: MockCommandAdapter = (command, args) => {
  return handleMockPackCommand(command, args);
};

export const applyMockUnavailableNativeFileDropCommand: MockCommandAdapter = () => {
  throw new Error("Native file drop is unavailable in browser development.");
};

export const applyMockSupportCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockSupportCommand(
    command,
    args,
    mockRuntimeState.config,
    Boolean(getMockActiveResume(mockRuntimeState.resumes)),
  );
  return result.handled ? result.value : undefined;
};

export const applyMockExternalAiCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockExternalAiCommand(command, args, mockRuntimeState);
  return result.handled ? result.value : undefined;
};

export const applyMockRecoveryCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockRecoveryCommand(
    command,
    args,
    mockRuntimeState.pendingUrlImports.length,
    mockRuntimeState,
  );
  if (!result.handled) return undefined;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockSearchLinksCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockSearchLinksCommand(command, args);
  return result.handled ? result.value : undefined;
};

export const applyMockJobImportCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockJobImportCommand(command, args, {
    jobs: mockRuntimeState.jobs,
    pendingUrlImports: mockRuntimeState.pendingUrlImports,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockLinkedInCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockLinkedInWorkbenchCommand(command, args, {
    jobs: mockRuntimeState.jobs,
    applications: mockRuntimeState.applications,
    pendingReminders: mockRuntimeState.pendingReminders,
    reviewed: mockRuntimeState.linkedinWorkbenchReviewed,
  });
  mockRuntimeState.jobs = result.state.jobs;
  mockRuntimeState.applications = result.state.applications;
  mockRuntimeState.pendingReminders = result.state.pendingReminders;
  mockRuntimeState.linkedinWorkbenchReviewed = result.state.reviewed;
  saveMockState();
  return result.value;
};

export const applyMockApplicationsCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockApplicationsCommand(command, args, {
    jobs: mockRuntimeState.jobs,
    applications: mockRuntimeState.applications,
    pendingReminders: mockRuntimeState.pendingReminders,
    interviews: mockRuntimeState.interviews,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockOpportunityCaseCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockOpportunityCaseCommand(command, args, {
    jobs: mockRuntimeState.jobs,
    applications: mockRuntimeState.applications,
  });
  return result.handled ? result.value : undefined;
};

export const applyMockResumeCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockResumeCommand(command, args, {
    jobs: mockRuntimeState.jobs,
    resumes: mockRuntimeState.resumes,
    userSkills: mockRuntimeState.userSkills,
    resumeDrafts: mockRuntimeState.resumeDrafts,
    recentMatches: mockRuntimeState.recentMatches,
    savedMatchEvidence: mockRuntimeState.savedMatchEvidence,
    pendingMilitaryTransitionReviews:
      mockRuntimeState.pendingMilitaryTransitionReviews,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockSalaryCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockSalaryCommand(command, args);
  return result.handled ? result.value : undefined;
};

export const applyMockMarketCommand: MockCommandAdapter = (command, args) => {
  const result = handleMockMarketCommand(command, args, {
    marketAlerts: mockRuntimeState.marketAlerts,
  });
  if (!result.handled) return undefined;
  mockRuntimeState.marketAlerts = result.state.marketAlerts;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockApplicationAssistCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockApplicationAssistCommand(command, args, {
    applicationProfile: mockRuntimeState.applicationProfile,
    screeningAnswers: mockRuntimeState.screeningAnswers,
    automationBrowserRunning: mockRuntimeState.automationBrowserRunning,
    nextAutomationAttemptId: mockRuntimeState.nextAutomationAttemptId,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockSourceHealthCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockSourceHealthCommand(command, args, {
    config: mockRuntimeState.config,
    scraperEnabledOverrides: mockRuntimeState.scraperEnabledOverrides,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockInterviewCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockInterviewCommand(command, args, {
    interviewPrepChecklists: mockRuntimeState.interviewPrepChecklists,
    interviewFollowups: mockRuntimeState.interviewFollowups,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockCoverLetterTemplateCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockCoverLetterTemplateCommand(command, args, {
    coverLetterTemplates: mockRuntimeState.coverLetterTemplates,
  });
  if (!result.handled) return undefined;
  mockRuntimeState.coverLetterTemplates = result.state.coverLetterTemplates;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockNotificationCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockNotificationCommand(command, args, {
    notificationPreferences: mockRuntimeState.notificationPreferences,
  });
  if (!result.handled) return undefined;
  mockRuntimeState.notificationPreferences =
    result.state.notificationPreferences;
  if (result.shouldSave) saveMockState();
  return result.value;
};

export const applyMockSavedSearchCommand: MockCommandAdapter = (
  command,
  args,
) => {
  const result = handleMockSavedSearchCommand(command, args, {
    savedSearches: mockRuntimeState.savedSearches,
    searchHistory: mockRuntimeState.searchHistory,
  });
  if (!result.handled) return undefined;
  Object.assign(mockRuntimeState, result.state);
  if (result.shouldSave) saveMockState();
  return result.value;
};
