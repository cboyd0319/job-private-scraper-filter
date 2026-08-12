/** Detects privacy, authorization, and error-redaction regressions in command surfaces. */

import { existsSync } from "node:fs";
import {
  readFileSync,
  join,
  rawResumeNameLoggingPaths,
  resumeCommandDtoPrivacyPaths,
  resumeCommandErrorPrivacyPaths,
  atsCommandErrorPrivacyPaths,
  atsTimelineEventPrivacyPaths,
  automationCommandErrorPrivacyPaths,
  sensitiveCommandErrorPrivacyPaths,
  utilityCommandErrorPrivacyPaths,
  rawCommandSetupErrorDisplayPaths,
  configValidationPrivacyPaths,
  rawJobImportLoggingPaths,
  importCommandPrivacyPaths,
  rawImportRedirectDisplayPaths,
  urlSecurityPrivacyPaths,
  importBookmarkletCommandPrivacyPaths,
  rawBookmarkletLoggingPaths,
  rawAutomationQuestionLoggingPaths,
  automationFormPrivacyPaths,
  automationBrowserErrorPrivacyPaths,
  screeningAnswerCommandLoggingPaths,
  rawNotificationJobTitleLoggingPaths,
  stripRustTestModules,
} from "./shared.mjs";

export function hasRawAutomationDropdownValueLogging(root, path) {
  if (path !== "crates/jobsentinel-assistance/src/automation/browser/page.rs") {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /tracing::debug!\(\s*"Selected option[^"]*"[\s\S]{0,120},\s*value\b/.test(
    productionText,
  );
}

export function hasRawResumeNameLogging(root, path) {
  if (!rawResumeNameLoggingPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /tracing::(?:debug|info|warn|error)!\([^;]*import_json_resume[^;]*(?:\bname\s*[:=]\s*\{\}|\bname\s*=\s*%?name\b)/.test(
    productionText,
  );
}

export function hasRawResumeCommandErrorDetails(root, path) {
  if (!resumeCommandErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /map_err\(\|e\|\s*format!\(\s*"(?:Failed to|Export failed)[^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /tracing::info!\([^;]*(?:job:\s*\{\}|skill:\s*\{\})[^;]*\)/.test(productionText) ||
    /tracing::info!\([^;]*(?:\bjob_hash\b|skill\.skill_name)[^;]*\)/.test(productionText)
  );
}

export function hasRawAtsCommandErrorDetails(root, path) {
  if (!atsCommandErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /map_err\(\|e\|\s*format!\(\s*"(?:Failed to|Invalid status)[^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /tracing::info!\([^;]*(?:job_hash:\s*\{\}|status:\s*\{\}|type:\s*\{\}|at:\s*\{\}|outcome:\s*\{\})[^;]*\)/.test(
      productionText,
    ) ||
    /tracing::info!\([^;]*(?:\bjob_hash\b|\bstatus\b|\binterview_type\b|\bscheduled_at\b|\boutcome\b)[^;]*\)/.test(
      productionText,
    )
  );
}

export function hasRawAtsTimelinePrivateEventData(root, path) {
  if (!atsTimelineEventPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /serde_json::json!\(\s*\{[\s\S]{0,220}"notes"\s*:\s*notes/.test(productionText) ||
    /serde_json::json!\(\s*\{[\s\S]{0,260}"message"\s*:\s*message/.test(productionText)
  );
}

export function hasRawAutomationCommandErrorDetails(root, path) {
  if (!automationCommandErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /map_err\(\|e\|\s*format!\(\s*"Failed to [^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /Err\(e\)\s*=>\s*Err\(format!\(\s*"Failed to [^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /tracing::(?:info|warn)!\([^;]*(?:job:\s*\{\}|hash:\s*\{\})[^;]*\)/.test(
      productionText,
    ) ||
    /tracing::(?:info|warn)!\([^;]*(?:\bjob_hash\b\s*,|\bjob_hash\s*=\s*[%?]?\s*job_hash\b)[^;]*\)/.test(
      productionText,
    ) ||
    /tracing::warn!\(\s*"Failed to create automation attempt:\s*\{\}"\s*,\s*e\s*\)/.test(
      productionText,
    )
  );
}

export function hasRawSensitiveCommandErrorDetails(root, path) {
  if (!sensitiveCommandErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /map_err\(\|e\|\s*format!\(\s*"Failed to [^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /Err\(e\)\s*=>\s*Err\(format!\(\s*"Failed to [^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /serde_json::to_value\([^)]*\)\.map_err\(\|e\|\s*format!\(\s*"Failed to [^"]*:\s*\{\}"\s*,\s*e\s*\)\)/.test(
      productionText,
    ) ||
    /tracing::info!\([^;]*(?:job:\s*\{\}|scenario:\s*\{\})[^;]*\)/.test(productionText) ||
    /tracing::info!\([^;]*(?:\bjob_hash\b\s*,|\bscenario\b\s*,|\bjob_hash\s*=\s*[%?]?\s*job_hash\b|\bscenario\s*=\s*[%?]?\s*scenario\b)[^;]*\)/.test(
      productionText,
    )
  );
}

export function hasRawUtilityCommandErrorDetails(root, path) {
  if (!utilityCommandErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /format!\(\s*"(?:Scraping failed|Database error|Failed to [^"]*|Invalid (?:configuration|ghost config)):\s*\{\}"\s*,\s*e\s*\)/.test(
      productionText,
    ) ||
    /format!\(\s*"Failed to [^"]*\{\}:\s*\{\}"\s*,\s*[^,]+,\s*e\s*\)/.test(
      productionText,
    ) ||
    /tracing::error!\(\s*"[^"]*:\s*\{\}"\s*,\s*e\s*\)/.test(productionText) ||
    /tracing::error!\(\s*"Failed to serialize job \{\}:\s*\{\}"\s*,\s*job\.id\s*,\s*e\s*\)/.test(
      productionText,
    ) ||
    /tracing::error!\([^;]*error\s*=\s*%e/.test(productionText) ||
    /DeepLinkOpenedEvent\s*\{\s*url:\s*url\.clone\(\)\s*\}/.test(productionText)
  );
}

function resumeSummaryStructMissingOrPrivate(text) {
  const match = text.match(
    /pub(?:\([^)]*\))?\s+struct\s+ResumeSummary\s*\{([^}]*)\}/,
  );
  return !match || /\b(?:file_path|parsed_text)\b/.test(match[1]);
}

export function hasRawResumeCommandDtoExposure(root, path) {
  if (!resumeCommandDtoPrivacyPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");

  if (path === "src-tauri/src/ipc/resume.rs") {
    const productionText = stripRustTestModules(text);
    return (
      /Result\s*<\s*Option\s*<\s*Resume\s*>\s*,\s*String\s*>/.test(productionText) ||
      /Result\s*<\s*Vec\s*<\s*Resume\s*>\s*,\s*String\s*>/.test(productionText) ||
      resumeSummaryStructMissingOrPrivate(productionText)
    );
  }

  if (path === "src/features/resumes/library/ResumeLibraryPage.tsx") {
    return /interface\s+ResumeData\s*\{[\s\S]{0,320}\b(?:file_path|parsed_text)\b/.test(text);
  }

  if (path === "src/features/resumes/builder/ResumeBuilderPage.tsx") {
    return /interface\s+Resume\s*\{[\s\S]{0,320}\b(?:file_path|parsed_text)\b/.test(text);
  }

  if (path === "src/dev-runtime/mocks/handlers.ts") {
    if (existsSync(join(root, "src/dev-runtime/features/resumes/resumeCommands.ts"))) {
      return false;
    }
    return (
      !/(?:toMockResumeSummary|handleMockResumeCommand)/.test(text) ||
      /case\s+["']get_active_resume["']:[\s\S]{0,180}return\s+getActiveResume\(\)\s+as\s+T/.test(
        text,
      ) ||
      /case\s+["']list_all_resumes["']:[\s\S]{0,120}return\s+resumes\s+as\s+T/.test(text)
    );
  }

  if (path === "src/dev-runtime/features/resumes/resumeCommands.ts") {
    return (
      !/toMockResumeSummary/.test(text) ||
      /case\s+["']list_all_resumes["']:[\s\S]{0,160}return\s+withoutSave\(\s*state\s*,\s*state\.resumes\s*\)/.test(
        text,
      )
    );
  }

  return (
    /invoke<Resume>\(["']get_active_resume["']\)/.test(text) ||
    resumeSummaryStructMissingOrPrivate(text)
  );
}

export function hasRawCommandSetupErrorDisplay(root, path) {
  if (!rawCommandSetupErrorDisplayPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /map_err\(\s*\|e\|\s*format!\(\s*"Failed to (?:load config|save config|create config directory|connect to database|migrate database|run migrations): \{\}"\s*,\s*e\s*\)\s*\)/.test(
      productionText,
    ) ||
    /format!\(\s*"Configuration error: \{\}"\s*,\s*e\s*\)/.test(productionText) ||
    /tracing::error!\(\s*"Failed to load config: \{\}"\s*,\s*e\s*\)/.test(productionText) ||
    /tracing::error!\([\s\S]{0,240}error\s*=\s*%e[\s\S]{0,240}"Failed to [^"]*(?:config|configuration|database)"/.test(
      productionText,
    )
  );
}

export function hasRawConfigValidationUrlDisplay(root, path) {
  if (!configValidationPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /Got:\s*\{\}"[\s\S]{0,120},\s*url\b/.test(productionText);
}

export function hasRawImportRedirectDisplay(root, path) {
  if (!rawImportRedirectDisplayPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /(?:Redirect blocked while fetching URL: \{location\}|URL validation failed: \{0\}|Invalid JSON-LD format: \{0\}|HTML parsing failed: \{0\}|Database error: \{0\}|HTTP request failed: \{0\})/.test(
    productionText,
  );
}

export function hasRawJobImportLogging(root, path) {
  if (!rawJobImportLoggingPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return (
    /#\[tracing::instrument\([^\]]*fields\(url\)/.test(text) ||
    /tracing::info!\([^;]*(?:title|company)\s*=\s*%(?:preview\.)?(?:title|company)/.test(text)
  );
}

export function hasRawImportHttpErrorReturn(root, path) {
  if (!importCommandPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /Failed to fetch the page:\s*\{\}[\s\S]{0,80},\s*e\b/.test(productionText);
}

export function hasNonPublicIpErrorEcho(root, path) {
  if (!urlSecurityPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return /Blocked non-public IP address ['"]?\{[^}]*}/.test(productionText);
}

export function hasRawImportBookmarkletCommandErrorDetails(root, path) {
  if (!importBookmarkletCommandPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /format!\(\s*"(?:Failed to serialize job|Invalid URL|Failed to read the job page response|Failed to parse the page|Invalid Schema\.org data format|Database error):\s*\{\}"/.test(
      productionText,
    ) ||
    /format!\(\s*"Invalid category:\s*\{\}"/.test(productionText) ||
    /tracing::(?:error|warn)!\(\s*"[^"]*(?:scoring config|bookmarklet server|Connection error|Accept error|job data|Database error)[^"]*:\s*\{\}"\s*,\s*e\s*\)/.test(
      productionText,
    ) ||
    /tracing::error!\([^;]*error\s*=\s*%e/.test(productionText) ||
    /json_error_response\(\s*format!\(\s*"[^"]*\{e\}[^"]*"\s*\)\s*\)/.test(
      productionText,
    ) ||
    /json_error_response\(\s*format!\(\s*r#"\{\{"error":"[^"]*\{\}[^"]*"\}\}"#,\s*e\s*\)\s*\)/.test(
      productionText,
    )
  );
}

export function hasRawBookmarkletImportLogging(root, path) {
  if (!rawBookmarkletLoggingPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /tracing::info!\([^;]*title\s*=\s*%title[^;]*company\s*=\s*%company/s.test(
      productionText,
    ) ||
    /tracing::info!\([^;]*(?:\bjob_hash\b\s*,|\bjob_hash\s*=\s*%job_hash\b)/.test(
      productionText,
    )
  );
}

export function hasManualBookmarkletJsonErrorResponses(root, path) {
  if (!rawBookmarkletLoggingPaths.has(path)) {
    return false;
  }

  return /format!\(r#"\{\{"error":"[^"]*\{\}[^"]*"\}\}"#,\s*e\)/.test(
    readFileSync(join(root, path), "utf8"),
  );
}

export function hasUnauthenticatedBookmarkletImports(root, path) {
  if (!rawBookmarkletLoggingPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  if (path.endsWith("/server.rs")) {
    return (
      /handle_import_request\(&request,\s*database\)\.await/.test(productionText) ||
      (/handle_import_request\(&request/.test(productionText) &&
        !/handle_import_request\(\s*&request,\s*&active_pairing/.test(productionText))
    );
  }
  if (path.endsWith("/imports.rs") && /fn\s+handle_import_request/.test(productionText)) {
    return (
      !/#\[serde\(deny_unknown_fields\)\]/.test(productionText) ||
      !/consume_active_pairing\(/.test(productionText) ||
      !/authorize_browser_action\(/.test(productionText)
    );
  }
  return false;
}

export function hasNonAtomicBookmarkletPairing(root, path) {
  if (!rawBookmarkletLoggingPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  if (path.endsWith("/pairing_state.rs") && /fn\s+consume_active_pairing/.test(productionText)) {
    return (
      !/active\s*\.as_mut\(\)[\s\S]{0,180}\.authorize\(/.test(productionText) ||
      !/active\.take\(\)/.test(productionText) ||
      !/pairing\.revoke\(\)/.test(productionText)
    );
  }
  return /body_has_valid_bookmarklet_token|has_valid_bookmarklet_token|auth_token_expires_at/.test(
    productionText,
  );
}

export function hasRawAutomationQuestionLogging(root, path) {
  if (!rawAutomationQuestionLoggingPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return (
    /tracing::debug!\([^;]*(?:screening question|screening answer)[^;]*'\{\}'[^;]*question_text/.test(
      text,
    ) ||
    /tracing::debug!\([^;]*Matched pattern[\s\S]*answer\.question_pattern[\s\S]*question/.test(
      text,
    )
  );
}

export function hasRawAutomationFormResultData(root, path) {
  if (!automationFormPrivacyPaths.has(path)) {
    return false;
  }

  const text = path.endsWith(".rs")
    ? stripRustTestModules(readFileSync(join(root, path), "utf8"))
    : readFileSync(join(root, path), "utf8");

  if (path === "crates/jobsentinel-assistance/src/automation/form_filler.rs") {
    return (
      /format!\(\s*"screening:\{\}"\s*,\s*(?:field_name|question_text)/.test(text) ||
      /truncate_question\(&question_text/.test(text) ||
      /Failed to (?:execute|parse) question finder (?:script|result):\s*\{\}/.test(text)
    );
  }

  return /`screening:\$\{answer\.questionPattern\}`/.test(text);
}

export function hasRawAutomationBrowserErrors(root, path) {
  if (!automationBrowserErrorPrivacyPaths.has(path)) {
    return false;
  }

  const productionText = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  return (
    /Failed to build browser config:\s*\{\}/.test(productionText) ||
    /File does not exist:\s*\{:\?\}/.test(productionText) ||
    /Invalid file path encoding/.test(productionText) ||
    /Failed to build file upload params:\s*\{\}/.test(productionText)
  );
}

export function hasRawScreeningAnswerCommandLogging(root, path) {
  if (!screeningAnswerCommandLoggingPaths.has(path)) {
    return false;
  }

  const text = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  const traceCalls = text.match(/tracing::(?:debug|info|warn|error)!\([\s\S]*?\);/g) ?? [];

  return traceCalls.some((call) => {
    if (/\b(?:question_text|answer_filled|modified_to)\b/.test(call)) {
      return true;
    }

    if (/\bquestion_pattern\b/.test(call) && !/\bquestion_pattern_chars\b/.test(call)) {
      return true;
    }

    if (
      /(?:\bquestion\s*=\s*[%?]?|\bquestion\s*,|\?question\b)/.test(call) &&
      !/\bquestion_chars\b/.test(call)
    ) {
      return true;
    }

    return (
      /(?:\bpattern\s*=\s*[%?]?|\bpattern\s*,|\?pattern\b)/.test(call) &&
      !/\b(?:pattern_chars|has_pattern)\b/.test(call)
    );
  });
}

export function hasRawNotificationJobTitleLogging(root, path) {
  if (!rawNotificationJobTitleLoggingPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return /tracing::info!\([^;]*notification\.job\.title/.test(text);
}
