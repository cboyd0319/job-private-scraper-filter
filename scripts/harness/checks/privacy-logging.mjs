import * as commands from "./privacy-logging/commands.mjs";
import * as core from "./privacy-logging/core.mjs";
import * as credentials from "./privacy-logging/credentials-feedback-notifications.mjs";
import * as frontend from "./privacy-logging/frontend.mjs";

export * from "./privacy-logging/commands.mjs";
export * from "./privacy-logging/core.mjs";
export * from "./privacy-logging/credentials-feedback-notifications.mjs";
export * from "./privacy-logging/frontend.mjs";

const privacyLoggingViolationChecks = [
  [core.hasRawPrivateQueryLogging, "replace raw private query logging"],
  [core.hasRawUserDataPrivacyLogging, "replace raw user-data privacy logging"],
  [core.hasRawSchedulerJobContentLogging, "sanitize scheduler job content logging"],
  [core.hasRawSchedulerScraperErrorDetails, "sanitize scheduler scraper error details"],
  [core.hasRawScraperUrlOrQueryLogging, "replace raw scraper URL/query logging"],
  [core.hasRawScraperLoopErrorLogging, "sanitize scraper loop error logging"],
  [core.hasUnboundedExternalResponseBodyRead, "replace unbounded external response body read"],
  [core.hasRawLocalPathLogging, "replace raw local path logging"],
  [core.hasRawBackupPathError, "sanitize backup path error display"],
  [core.hasMlRawLocalPathExposure, "remove ML raw local path exposure"],
  [core.hasMlRawErrorDisplay, "sanitize ML error display"],
  [core.hasMlRawLocalPathDoc, "remove ML raw local path doc claim"],
  [core.hasRawJobsWithGptDebug, "sanitize JobsWithGPT debug output"],
  [core.hasRawLinkedInDebug, "sanitize legacy LinkedIn source debug output"],
  [credentials.hasLinkedInLoginCookieReturn, "keep LinkedIn login cookie out of renderer response"],
  [credentials.hasRawEmailTestErrorReturn, "sanitize test email command errors"],
  [credentials.hasRawSlackWebhookValidationErrorReturn, "sanitize Slack webhook validation command errors"],
  [credentials.hasSecretBearingDebugDerive, "sanitize secret-bearing debug derive"],
  [credentials.hasCredentialKeyInputEcho, "avoid echoing credential key input"],
  [credentials.hasRawCredentialStorageErrors, "sanitize credential storage errors"],
  [credentials.hasStaleActiveSecretStorageWording, "replace stale OS-keyring credential storage wording"],
  [credentials.hasMissingLinkedInCredentialStorageDisable, "disable LinkedIn credential storage"],
  [
    credentials.hasMissingWebhookCredentialStorageValidation,
    "validate notification webhook credentials before keyring storage",
  ],
  [credentials.hasRendererCredentialSecretRead, "keep credential values out of renderer IPC"],
  [credentials.hasIncompleteConfigExportRedaction, "redact all credential fields from config export"],
  [credentials.hasRawTelegramBotTokenRequestError, "remove Telegram bot-token URLs from request errors"],
  [credentials.hasRawWebhookTokenRequestError, "remove webhook token URLs from request errors"],
  [credentials.hasRawNotificationProviderErrorBody, "omit notification provider error bodies from errors"],
  [credentials.hasExternalAlertRawScoreReasons, "keep raw match reasons out of external alerts"],
  [credentials.hasRawNotificationServiceErrorDetails, "sanitize notification service error details"],
  [
    frontend.hasFrontendDesktopNotificationPassthrough,
    "keep desktop notification payloads privacy-preserving",
  ],
  [core.hasRawJobsWithGptSmokeEndpointError, "sanitize JobsWithGPT smoke-test endpoint errors"],
  [core.hasRawSourceCheckResultError, "sanitize source-check result errors"],
  [core.hasRawUrlLogging, "replace raw URL logging"],
  [core.hasRawUrlErrorDisplay, "replace raw URL error display"],
  [frontend.hasRawFrontendErrorHelperUserMessage, "sanitize frontend user error messages"],
  [core.hasRawPathOrQueryErrorDisplay, "replace raw path/query error display"],
  [core.hasRawResumeParserPathDisplay, "sanitize resume parser path error display"],
  [commands.hasRawResumeNameLogging, "sanitize resume import name logging"],
  [commands.hasRawResumeCommandErrorDetails, "sanitize resume command error details"],
  [commands.hasRawAtsCommandErrorDetails, "sanitize application tracking command error details"],
  [commands.hasRawAtsTimelinePrivateEventData, "keep private ATS timeline text out of event data"],
  [commands.hasRawAutomationCommandErrorDetails, "sanitize automation command error details"],
  [commands.hasRawSensitiveCommandErrorDetails, "sanitize sensitive command error details"],
  [commands.hasRawUtilityCommandErrorDetails, "sanitize utility command error details"],
  [commands.hasRawResumeCommandDtoExposure, "hide resume file paths from renderer DTOs"],
  [commands.hasRawCommandSetupErrorDisplay, "replace raw command setup error display"],
  [commands.hasRawConfigValidationUrlDisplay, "sanitize config validation URL display"],
  [commands.hasRawImportRedirectDisplay, "replace raw import redirect display"],
  [commands.hasRawJobImportLogging, "replace raw job import logging"],
  [commands.hasRawImportHttpErrorReturn, "sanitize job import HTTP errors"],
  [commands.hasRawImportBookmarkletCommandErrorDetails, "sanitize import and bookmarklet command error details"],
  [commands.hasNonPublicIpErrorEcho, "sanitize non-public IP validation errors"],
  [commands.hasRawAutomationQuestionLogging, "replace raw automation screening question logging"],
  [commands.hasRawScreeningAnswerCommandLogging, "replace raw screening-answer command logging"],
  [commands.hasRawAutomationFormResultData, "sanitize automation form result data"],
  [commands.hasRawAutomationBrowserErrors, "sanitize automation browser errors"],
  [commands.hasRawAutomationDropdownValueLogging, "remove raw automation dropdown value logging"],
  [commands.hasRawNotificationJobTitleLogging, "replace raw notification job title logging"],
  [commands.hasRawBookmarkletImportLogging, "replace raw bookmarklet import metadata logging"],
  [core.hasRawScoringCacheJobHashLogging, "replace raw scoring cache job hash logging"],
  [core.hasRawSchedulerScoringPrivacyLeak, "replace raw scheduler scoring privacy leaks"],
  [core.hasResidualCorePrivacyLeak, "replace residual core privacy leaks"],
  [commands.hasManualBookmarkletJsonErrorResponses, "replace manual bookmarklet JSON error responses"],
  [
    commands.hasUnauthenticatedBookmarkletImports,
    "require structured bookmarklet pairing authorization",
  ],
  [commands.hasNonAtomicBookmarkletPairing, "make bookmarklet pairing atomic and one-use"],
  [
    frontend.hasBookmarkletCodeWithoutPairingBoundary,
    "bind bookmarklet code to structured pairing and origin",
  ],
  [frontend.hasUnsanitizedFrontendErrorReportStorage, "sanitize frontend error report storage"],
  [frontend.hasRawFrontendErrorReporterForwarding, "sanitize frontend error reporter console forwarding"],
  [frontend.hasRawFrontendErrorHelperDebugLogging, "sanitize frontend error helper debug logging"],
  [frontend.hasRawFrontendSharedErrorLogging, "sanitize shared frontend error logging"],
  [frontend.hasRawFrontendToastSupportDetails, "sanitize frontend toast support details"],
  [
    frontend.hasRawFrontendDirectErrorLogging,
    "route frontend direct error logging through sanitized logger",
  ],
  [frontend.hasUnsafeErrorReportStorageParsing, "validate stored error reports before loading"],
  [
    frontend.hasHardcodedFrontendErrorExportVersion,
    "derive frontend error export version from package metadata",
  ],
  [credentials.hasStaleFeedbackWebhookSanitizer, "redact provider webhook URLs in feedback sanitizer"],
  [
    credentials.hasIncompleteFeedbackJobSearchSanitizer,
    "redact sensitive job-search context in feedback sanitizer",
  ],
  [credentials.hasUnsanitizedStructuredDebugLogEvents, "sanitize structured feedback debug events"],
  [credentials.hasUnsanitizedFeedbackFileSave, "sanitize feedback file content before saving"],
  [credentials.hasRawFeedbackOpenErrors, "sanitize feedback support-open errors"],
];

export function collectPrivacyLoggingViolations(root, path) {
  return privacyLoggingViolationChecks
    .filter(([predicate]) => predicate(root, path))
    .map(([, message]) => `${message}: ${path}`);
}
