import { isSafeExternalHttpsUrl } from "../../mocks/externalUrlSafety";
import {
  getArg,
  getStringArg,
  normalizeProfileInput,
} from "../../mocks/handlers/commandHelpers";
import type {
  MockApplicationProfile,
  MockFillResultWithAttempt,
  MockScreeningAnswer,
} from "../../mocks/handlers/types";
import {
  buildMockApplicationProfileFromInput,
  getMockApplicationProfileEdit,
  getMockApplicationProfilePreview,
  getMockSuggestedAnswers,
  upsertMockScreeningAnswer,
} from "../../../features/application-assist/mockProfile";
import { getMockAtsPlatform, getMockAtsPlatformDetection } from "./atsPlatform";
import { requiresUserAnswer } from "../../../shared/applicationScreeningTaxonomy";

export interface MockApplicationAssistState {
  applicationProfile: MockApplicationProfile | null;
  screeningAnswers: MockScreeningAnswer[];
  automationBrowserRunning: boolean;
  nextAutomationAttemptId: number;
}

interface MockApplicationAssistCommandResult {
  handled: boolean;
  value: unknown;
  state: MockApplicationAssistState;
  shouldSave: boolean;
}

function fillMockApplicationForm(
  args: Record<string, unknown> | undefined,
  state: MockApplicationAssistState,
): { value: MockFillResultWithAttempt; state: MockApplicationAssistState } {
  const jobUrl = getStringArg(args, "jobUrl") ?? getStringArg(args, "job_url") ?? "";
  if (!isSafeExternalHttpsUrl(jobUrl)) {
    throw new Error("This application link is not safe to open");
  }

  const platform = getMockAtsPlatform(jobUrl);
  const hasJobHash = Boolean(getStringArg(args, "jobHash") ?? getStringArg(args, "job_hash"));
  const attemptId = hasJobHash ? state.nextAutomationAttemptId : null;
  const detectedQuestions = getArg(args, "detectedQuestions");
  const requiresManualReview = Array.isArray(detectedQuestions) && detectedQuestions.some(
    (question) => typeof question === "string" && requiresUserAnswer(question),
  );
  const screeningFields = state.screeningAnswers
    .filter((answer) => !requiresUserAnswer(answer.questionPattern))
    .slice(0, 2)
    .map((_, index) => `screening:savedAnswer${index + 1}`);

  return {
    value: {
      filledFields: ["firstName", "lastName", "email", "phone", "resume", ...screeningFields],
      unfilledFields: platform === "unknown" ? ["customQuestion"] : [],
      captchaDetected: false,
      readyForReview: true,
      errorMessage: null,
      manualReviewTopics: requiresManualReview
        ? ["voluntary or sensitive personal questions"]
        : [],
      attemptId,
      durationMs: 1250,
      atsPlatform: platform,
    },
    state: {
      ...state,
      automationBrowserRunning: true,
      nextAutomationAttemptId: hasJobHash
        ? state.nextAutomationAttemptId + 1
        : state.nextAutomationAttemptId,
    },
  };
}

export function handleMockApplicationAssistCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockApplicationAssistState,
): MockApplicationAssistCommandResult {
  switch (command) {
    case "has_application_profile":
      return result(Boolean(state.applicationProfile), state);
    case "get_application_profile_preview":
      return result(getMockApplicationProfilePreview(state.applicationProfile), state);
    case "get_application_profile":
      return result(getMockApplicationProfileEdit(state.applicationProfile), state);
    case "select_application_resume_file":
      return result(
        {
          token: "7d9d16a1-2e5d-4b32-9eb2-bfbffb4ee871--mock-resume.pdf",
          fileName: "mock-resume.pdf",
        },
        state,
      );
    case "upsert_application_profile": {
      const input = normalizeProfileInput(getArg(args, "input"));
      const applicationProfile = buildMockApplicationProfileFromInput(
        input,
        state.applicationProfile,
      );
      return result(applicationProfile.id, { ...state, applicationProfile }, true);
    }
    case "get_screening_answers":
      return result(state.screeningAnswers, state);
    case "get_application_screening_answer_previews":
      return result(
        state.screeningAnswers
          .filter((answer) => !requiresUserAnswer(answer.questionPattern))
          .map(({ questionPattern, answer }) => ({ questionPattern, answer })),
        state,
      );
    case "upsert_screening_answer":
      return result(
        undefined,
        {
          ...state,
          screeningAnswers: upsertMockScreeningAnswer(args, state.screeningAnswers),
        },
        true,
      );
    case "get_automation_stats":
      return result(
        {
          totalAttempts: 42,
          submitted: 38,
          failed: 0,
          pending: 4,
          successRate: 90.476,
        },
        state,
      );
    case "detect_ats_platform":
      return result(getMockAtsPlatformDetection(getStringArg(args, "url") ?? ""), state);
    case "fill_application_form": {
      const fill = fillMockApplicationForm(args, state);
      return result(fill.value, fill.state);
    }
    case "is_browser_running":
      return result(state.automationBrowserRunning, state);
    case "close_automation_browser":
      return result(undefined, { ...state, automationBrowserRunning: false });
    case "mark_attempt_submitted":
      return result(undefined, state);
    case "get_suggested_answers":
      return result(getMockSuggestedAnswers(args, state.screeningAnswers), state);
    default:
      return { handled: false, value: undefined, state, shouldSave: false };
  }
}

function result(
  value: unknown,
  state: MockApplicationAssistState,
  shouldSave = false,
): MockApplicationAssistCommandResult {
  return { handled: true, value, state, shouldSave };
}
