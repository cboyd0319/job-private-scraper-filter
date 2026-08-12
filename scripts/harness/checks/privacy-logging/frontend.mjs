import {
  existsSync,
  readFileSync,
  join,
  frontendErrorReportingPaths,
  frontendErrorLoggerPaths,
  frontendToastSupportDetailPaths,
  frontendDirectErrorLoggingPaths,
  frontendDesktopNotificationPrivacyPaths,
  bookmarkletGeneratorPaths,
  stripRustTestModules,
  stripTypeScriptComments,
} from "./shared.mjs";

export function hasRawFrontendErrorReporterForwarding(root, path) {
  if (!frontendErrorReportingPaths.has(path)) {
    return false;
  }

  const text = stripTypeScriptComments(readFileSync(join(root, path), "utf8"));
  return (
    /originalConsoleError\.apply\(\s*console\s*,\s*args\s*\)/.test(text) ||
    /window\.onerror\s*=\s*\([\s\S]{0,640}return\s+false\s*;/.test(text) ||
    /window\.onunhandledrejection\s*=\s*\([\s\S]{0,720}if\s*\(\s*!import\.meta\.env\.DEV\s*\)\s*\{[\s\S]{0,120}event\.preventDefault\(\)/.test(
      text,
    )
  );
}

export function hasUnsanitizedFrontendErrorReportStorage(root, path) {
  if (!frontendErrorReportingPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return (
    /this\.errors\.unshift\(report\)/.test(text) ||
    /\berrors:\s*this\.errors\b/.test(text) ||
    (/localStorage\.setItem\(STORAGE_KEY,\s*JSON\.stringify\(this\.errors\)\)/.test(text) &&
      !/sanitizeStoredReport/.test(text)) ||
    /logError\(`\[ErrorReporter\]\[\$\{type\}\]`,\s*error\.message/.test(text) ||
    /\boriginalError:\s*error\b/.test(text) ||
    /logError\(`\[ErrorReporter\]\[\$\{type\}\]`[\s\S]{0,160}\breport,\s*$/m.test(text) ||
    /console\.warn\(\s*["']\[ErrorReporter\][^;]*,\s*(?:e|error)\s*\)/.test(text) ||
    !/token\(\?:\\s\+\|=\)/.test(text) ||
    text.includes("hooks\\.slack\\.com\\/services") ||
    !text.includes("discord(?:app)?\\.com\\/api\\/webhooks") ||
    !text.includes("outlook\\.office(?:365)?\\.com\\/webhook") ||
    !/salary/.test(text) ||
    !text.includes("screening[_-]?(?:question|answer)") ||
    !text.includes("private[_-]?notes?") ||
    !text.includes("SENSITIVE_LABELED_TEXT_PATTERNS")
  );
}

export function hasRawFrontendErrorHelperDebugLogging(root, path) {
  const fullPath = join(root, path);
  if (!path.startsWith("src/") || !/\.tsx?$/.test(path) || !existsSync(fullPath)) {
    return false;
  }

  const text = readFileSync(fullPath, "utf8");
  const definesErrorDetailLogger = /function\s+logErrorDetails\b/.test(text);
  const hasRawDetailLogging =
    /console\.error\(\s*["']Error:["']\s*,\s*error\s*\)/.test(text) ||
    /console\.log\(\s*["']Context:["']\s*,\s*context\s*\)/.test(text) ||
    /console\.log\(\s*["']Stack:["']\s*,\s*error\.stack\s*\)/.test(text);

  if (hasRawDetailLogging) {
    return true;
  }

  return definesErrorDetailLogger && (
    !text.includes("sanitizeDebugValue") ||
    !text.includes("sanitizeTextForStorage") ||
    !text.includes("sanitizeContext")
  );
}

export function hasRawFrontendErrorHelperUserMessage(root, path) {
  const fullPath = join(root, path);
  if (!path.startsWith("src/") || !/\.tsx?$/.test(path) || !existsSync(fullPath)) {
    return false;
  }

  const text = readFileSync(fullPath, "utf8");
  return /function\s+getUserMessage[\s\S]*?\breturn\s+error\.message\s*;/.test(text);
}

export function hasRawFrontendSharedErrorLogging(root, path) {
  if (!frontendErrorLoggerPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  const getErrorMessageStart = text.indexOf("export function getErrorMessage");
  const getErrorMessageEnd =
    getErrorMessageStart === -1
      ? -1
      : text.indexOf("\n/**", getErrorMessageStart + 1);
  const getErrorMessageBody =
    getErrorMessageStart === -1
      ? ""
      : text.slice(
          getErrorMessageStart,
          getErrorMessageEnd === -1 ? undefined : getErrorMessageEnd,
        );
  return (
    /console\.error\(\s*message\s*,\s*error\s*\)/.test(text) ||
    /\breturn\s+(?:error\.message|error|String\(\s*\([^)]*message|String\(\s*error)/.test(
      getErrorMessageBody,
    ) ||
    !text.includes("sanitizeLoggedError") ||
    !text.includes("sanitizeTextForStorage") ||
    !text.includes("sanitizeContext")
  );
}

export function hasRawFrontendToastSupportDetails(root, path) {
  if (!frontendToastSupportDetailPaths.has(path)) {
    return false;
  }

  const text = stripTypeScriptComments(readFileSync(join(root, path), "utf8"));
  const fullMessageStart = text.indexOf("const fullMessage");
  const fullMessageBody =
    fullMessageStart === -1 ? "" : text.slice(fullMessageStart, fullMessageStart + 800);

  return (
    /(?:Technical|Support details):\s*\$\{\s*enhancedError\.message\s*\}/.test(text) ||
    /showTechnical[\s\S]{0,360}enhancedError\.message/.test(fullMessageBody) ||
    /invokeArgs\s*:\s*args/.test(text) ||
    /error\s+instanceof\s+Error\s*\?\s*error\s*:\s*new\s+Error\(String\(error\)\)/.test(text)
  );
}

export function hasRawFrontendDirectErrorLogging(root, path) {
  if (!frontendDirectErrorLoggingPaths.has(path)) {
    return false;
  }

  return /console\.error\(/.test(readFileSync(join(root, path), "utf8"));
}

export function hasUnsafeErrorReportStorageParsing(root, path) {
  if (!frontendErrorReportingPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return (
    /JSON\.parse\(stored\)\.map/.test(text) ||
    !/function\s+isErrorReport/.test(text) ||
    !/parseStoredErrorReports/.test(text)
  );
}

export function hasHardcodedFrontendErrorExportVersion(root, path) {
  if (!frontendErrorReportingPaths.has(path)) {
    return false;
  }

  return /app_version:\s*["']\d+\.\d+(?:\.\d+)?["']/.test(
    readFileSync(join(root, path), "utf8"),
  );
}

export function hasBookmarkletCodeWithoutPairingBoundary(root, path) {
  if (!bookmarkletGeneratorPaths.has(path)) {
    return false;
  }

  const text = stripRustTestModules(readFileSync(join(root, path), "utf8"));
  const originCheck = text.indexOf("location.origin!==__ORIGIN__");
  const pageAccess = text.indexOf("document.createElement");
  return (
    /api\/bookmarklet\/import/.test(text) &&
    (!/payload=\{pairing:__PAIRING__,job:job\}/.test(text) ||
      originCheck === -1 ||
      pageAccess === -1 ||
      originCheck > pageAccess)
  );
}

export function hasFrontendDesktopNotificationPassthrough(root, path) {
  if (!frontendDesktopNotificationPrivacyPaths.has(path)) {
    return false;
  }

  const text = readFileSync(join(root, path), "utf8");
  return (
    /export\s+async\s+function\s+notify\s*\(\s*title\s*:\s*string\s*,\s*body\s*:\s*string/.test(
      text,
    ) ||
    /sendNotification\(\s*\{\s*title\s*,\s*body\s*\}/.test(text) ||
    /body:\s*`[^`]*(?:\$\{\s*(?:title|company|jobTitle|message|score|Math\.round\(score)[^}]*\})/.test(
      text,
    ) ||
    /title:\s*`[^`]*(?:\$\{\s*(?:title|company|jobTitle|message)[^}]*\})/.test(text)
  );
}
