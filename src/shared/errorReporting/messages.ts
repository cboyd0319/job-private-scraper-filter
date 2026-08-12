/**
 * Error message utility for translating technical errors into user-friendly messages.
 *
 * This module provides human-readable error messages for non-technical users,
 * helping them understand what went wrong and what they can do about it.
 */

import { sanitizeTextForStorage } from './errorReporter';

export interface UserFriendlyError {
  title: string;
  message: string;
  action?: string;
  technical?: string;
}

/**
 * Network and connection error patterns
 */
const NETWORK_ERRORS = [
  { pattern: /network|fetch|connection|timeout|ECONNREFUSED/i, title: 'Connection Problem', message: 'We couldn\'t connect to the internet or the job board.', action: 'Check your internet connection and try again. If this keeps happening, copy a safe support report.' },
  { pattern: /ENOTFOUND|DNS|domain/i, title: 'Website Not Found', message: 'The job board website couldn\'t be reached.', action: 'The website might be down temporarily. Try again in a few minutes. If this keeps happening, copy a safe support report.' },
  { pattern: /certificate|ssl|tls/i, title: 'Security Certificate Problem', message: 'There\'s a problem with the website\'s security certificate.', action: 'This is usually temporary. Try again later, check your computer date and time, or copy a safe support report if this keeps happening.' },
  { pattern: /429|rate.?limit/i, title: 'Job Board Asked JobSentinel to Slow Down', message: 'The job board asked JobSentinel to wait before checking again.', action: 'Wait a few minutes, then try again. If this keeps happening, check this site less often or copy a safe support report.' },
  { pattern: /503|502|504|service.?unavailable/i, title: 'Service Temporarily Down', message: 'The job board\'s servers are temporarily unavailable.', action: 'This is on their end. Try again in 10 to 15 minutes. If this keeps happening, copy a safe support report.' },
  { pattern: /401|unauthorized|authentication/i, title: 'Sign-In Details Not Working', message: 'Your saved sign-in or access details aren\'t working.', action: 'Open Settings, reconnect the job source, or paste updated access details. If this keeps happening, copy a safe support report.' },
  { pattern: /403|forbidden/i, title: 'Access Denied', message: 'The job board would not let JobSentinel check it.', action: 'Check whether this job board needs a signed-in or premium account, then try again. If this keeps happening, copy a safe support report.' },
  { pattern: /404|not.?found/i, title: 'Page Not Found', message: 'The job listing or page no longer exists.', action: 'The posting may have been removed. Check the job list again, or copy a safe support report if this keeps happening.' },
];

/**
 * Database error patterns
 */
const DATABASE_ERRORS = [
  { pattern: /database.*locked|SQLITE_BUSY/i, title: 'Local Data Busy', message: 'JobSentinel is already saving or reading your local data.', action: 'Wait a moment and try again. If this keeps happening, copy a safe support report before closing and reopening JobSentinel.' },
  { pattern: /constraint|unique|duplicate/i, title: 'Duplicate Entry', message: 'This item is already saved.', action: 'Check your existing entries before adding it again.' },
  { pattern: /foreign.?key|reference/i, title: 'Related Information Missing', message: 'This action needs another saved item before it can continue.', action: 'Check that the related job, application, or setting still exists, then try again. If this keeps happening, copy a safe support report.' },
  { pattern: /corrupt|malformed|integrity/i, title: 'Local Data Problem', message: 'JobSentinel\'s local data file may be damaged.', action: 'Copy a safe support report and restore from a backup if you have one. Don\'t delete app data yet.' },
  { pattern: /disk|storage|space/i, title: 'Storage Full', message: 'Your computer is running out of disk space.', action: 'Free up some storage space on your hard drive and try again. If this keeps happening, copy a safe support report.' },
];

/**
 * Validation error patterns
 */
const VALIDATION_ERRORS = [
  { pattern: /required|missing|empty/i, title: 'Add missing details', message: 'Some details need to be added before this can continue.', action: 'Add the highlighted details, then try again.' },
  { pattern: /invalid.*email/i, title: 'Email Not Recognized', message: 'The email address does not look complete.', action: 'Check the email address and make sure it looks like: name@example.com' },
  { pattern: /invalid.*url|invalid.*webhook/i, title: 'Web Address Not Recognized', message: 'The web address is not in a format JobSentinel can use.', action: 'Copy the full address again. It should usually start with https://.' },
  { pattern: /invalid.*json/i, title: 'Data Not Recognized', message: 'The selected data is not in a format JobSentinel can use.', action: 'Try exporting it again from the original app, or copy a safe support report if you need help.' },
  { pattern: /password.*weak|password.*short/i, title: 'Weak Password', message: 'The password doesn\'t meet security requirements.', action: 'Use a longer password with a mix of letters, numbers, and special characters.' },
  { pattern: /date|time.*invalid/i, title: 'Check date or time', message: 'The date or time format is not recognized.', action: 'Use a date like MM/DD/YYYY, or check your computer date and time settings.' },
];

/**
 * Scraper and job source error patterns
 */
const SCRAPER_ERRORS = [
  { pattern: /parse|selector|element.*not.*found/i, title: 'Job Website Changed', message: 'The job board changed how its page is organized, so JobSentinel can\'t read it yet.', action: 'Check for app updates, or copy a safe support report if you want help.' },
  { pattern: /no.*jobs.*found|empty.*results/i, title: 'No Jobs Found', message: 'No job listings matched your search criteria.', action: 'Try broadening your search filters or check different job boards.' },
  { pattern: /scraper.*disabled|source.*unavailable/i, title: 'Job Source Turned Off', message: 'This job board is turned off in your settings.', action: 'Open Settings and turn this job source on if you still want JobSentinel to check it.' },
  { pattern: /api.*key|api.*quota|api.*limit/i, title: 'Daily Job Board Check Limit Reached', message: 'This job board has stopped accepting more checks today.', action: 'Wait until tomorrow, or set JobSentinel to check this job source less often.' },
  { pattern: /captcha|bot.*detection|cloudflare/i, title: 'Site Asked for a Human Check', message: 'This site asked for an extra human check before showing jobs.', action: 'Open the site yourself, complete any check there, or try again later. If this keeps happening, copy a safe support report.' },
];

/**
 * Configuration error patterns
 */
const CONFIG_ERRORS = [
  { pattern: /config.*not.*found|config.*missing/i, title: 'Saved Settings Need Attention', message: 'JobSentinel could not find your saved settings.', action: 'Open Settings and save them again. If this keeps happening, copy a safe support report before resetting anything.' },
  { pattern: /config.*invalid|config.*corrupt/i, title: 'Saved Settings Need Attention', message: 'JobSentinel could not read your saved settings.', action: 'Open Settings and save them again. If this keeps happening, copy a safe support report before resetting anything.' },
  { pattern: /permission|access.*denied|EACCES/i, title: 'Permission Needed', message: 'JobSentinel needs permission to open that file or folder.', action: 'Check your system privacy settings, then try again. If this keeps happening, copy a safe support report.' },
  { pattern: /file.*not.*found|ENOENT/i, title: 'File unavailable', message: 'JobSentinel could not find the file it needs.', action: 'Choose the file again, restore it from backup, or reinstall JobSentinel if this keeps happening.' },
];

/**
 * Notification and webhook error patterns
 */
const NOTIFICATION_ERRORS = [
  { pattern: /webhook.*failed|webhook.*invalid/i, title: 'Could not set up notifications', message: 'We couldn\'t send notifications to your saved alert channel.', action: 'Open Settings, choose that alert channel, then paste a fresh connection link.' },
  { pattern: /slack.*error/i, title: 'Could not send Slack notification', message: 'Couldn\'t send a notification to Slack.', action: 'Paste a fresh Slack connection link in Settings and make sure the channel still exists.' },
  { pattern: /discord.*error/i, title: 'Could not send Discord notification', message: 'Couldn\'t send a notification to Discord.', action: 'Paste a fresh Discord connection link in Settings and make sure the channel still exists.' },
  { pattern: /teams.*error/i, title: 'Could not send Teams notification', message: 'Couldn\'t send a notification to Microsoft Teams.', action: 'Paste a fresh Teams connection link in Settings and make sure the connector is still active.' },
  { pattern: /email.*error|smtp/i, title: 'Could not send email notification', message: 'Couldn\'t send an email notification.', action: 'Check your email sending settings and saved password. If this keeps happening, copy a safe support report.' },
];

/**
 * Application tracking error patterns
 */
const ATS_ERRORS = [
  { pattern: /application.*not.*found/i, title: 'Application Not Found', message: 'The job application you\'re looking for doesn\'t exist.', action: 'It may have been deleted. Check your applications list.' },
  { pattern: /interview.*conflict/i, title: 'Interview Time Conflict', message: 'This interview time overlaps with another one.', action: 'Choose a different time or reschedule one of the interviews.' },
  { pattern: /reminder.*failed/i, title: 'Could not create reminder', message: 'Couldn\'t create a reminder for this event.', action: 'Check your alert settings and try again. If this keeps happening, copy a safe support report.' },
];

/**
 * Resume review and optional outside-tool error patterns
 */
const RESUME_REVIEW_ERRORS = [
  { pattern: /resume.*not.*found|resume.*missing/i, title: 'Resume Not Found', message: 'No resume has been added yet.', action: 'Add your resume in the Resume section before using this feature.' },
  { pattern: /parsing.*failed|extract.*failed/i, title: 'Resume Could Not Be Read', message: 'JobSentinel could not read the information from your resume.', action: 'Use a PDF, DOCX, TXT, or Markdown resume, or choose a different file.' },
  { pattern: /ai.*error|model.*error|openai/i, title: 'Resume Review Problem', message: 'The resume review had a problem.', action: 'Try again in a moment. If this keeps happening, check any review tool you connected in Settings or copy a safe support report.' },
  { pattern: /token.*limit|context.*length/i, title: 'Resume or Job Post Too Long', message: 'The resume or job post is too long for this review.', action: 'Try a shorter resume or job post. Remove repeated sections first.' },
];

const OUTSIDE_AI_ERRORS = [
  {
    pattern: /outside AI.*(?:outcome is unknown|provider may have received)|do not retry.*outside AI/i,
    title: 'Outside AI Outcome Unknown',
    message: 'The provider may have received this request. Do not retry it.',
    action: 'Open Settings > Outside AI and check durable activity before taking any next step.',
  },
];

/**
 * All error pattern groups combined
 */
const ALL_ERROR_PATTERNS = [
  ...OUTSIDE_AI_ERRORS,
  ...RESUME_REVIEW_ERRORS,
  ...NETWORK_ERRORS,
  ...DATABASE_ERRORS,
  ...VALIDATION_ERRORS,
  ...SCRAPER_ERRORS,
  ...CONFIG_ERRORS,
  ...NOTIFICATION_ERRORS,
  ...ATS_ERRORS,
];

/**
 * Generic fallback error
 */
const GENERIC_ERROR: UserFriendlyError = {
  title: 'JobSentinel needs attention',
  message: 'JobSentinel ran into a problem.',
  action: 'Try again in a moment. If the problem continues, copy a safe support report and share it only if you want help.',
};

/**
 * Extract error message from various error types
 */
function extractErrorMessage(error: unknown): string {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (error && typeof error === 'object') {
    // Check for common error properties
    if ('message' in error && typeof error.message === 'string') {
      return error.message;
    }
    if ('error' in error && typeof error.error === 'string') {
      return error.error;
    }
    if ('msg' in error && typeof error.msg === 'string') {
      return error.msg;
    }

    return 'Unknown object error';
  }

  return String(error);
}

/**
 * Convert any error into a user-friendly error object
 *
 * @param error - The error to convert (can be string, Error, or unknown)
 * @returns A UserFriendlyError object with plain English explanation
 *
 * @example
 * ```typescript
 * try {
 *   await fetchJobs();
 * } catch (error) {
 *   const friendlyError = getUserFriendlyError(error);
 *   showNotification(friendlyError.title, friendlyError.message);
 * }
 * ```
 */
export function getUserFriendlyError(error: unknown): UserFriendlyError {
  const rawTechnicalMessage = extractErrorMessage(error);
  const safeTechnicalMessage = sanitizeTextForStorage(rawTechnicalMessage);

  // Try to match against known error patterns
  for (const pattern of ALL_ERROR_PATTERNS) {
    if (pattern.pattern.test(rawTechnicalMessage)) {
      return {
        title: pattern.title,
        message: pattern.message,
        action: pattern.action,
        technical: safeTechnicalMessage,
      };
    }
  }

  // No match found, return generic error
  return {
    ...GENERIC_ERROR,
    technical: safeTechnicalMessage,
  };
}

/**
 * Format a user-friendly error for display
 *
 * @param error - The UserFriendlyError to format
 * @param includeAction - Whether to include the action in the output (default: true)
 * @returns A formatted string for display
 *
 * @example
 * ```typescript
 * const error = getUserFriendlyError(err);
 * console.error(formatErrorForDisplay(error));
 * ```
 */
export function formatErrorForDisplay(error: UserFriendlyError, includeAction = true): string {
  let result = `${error.title}\n${error.message}`;

  if (includeAction && error.action) {
    result += `\n\nWhat to do: ${error.action}`;
  }

  return result;
}

/**
 * Check if an error is network-related
 */
export function isNetworkError(error: unknown): boolean {
  const message = extractErrorMessage(error);
  return NETWORK_ERRORS.some(pattern => pattern.pattern.test(message));
}

/**
 * Check if an error is database-related
 */
export function isDatabaseError(error: unknown): boolean {
  const message = extractErrorMessage(error);
  return DATABASE_ERRORS.some(pattern => pattern.pattern.test(message));
}

/**
 * Check if an error is validation-related
 */
export function isValidationError(error: unknown): boolean {
  const message = extractErrorMessage(error);
  return VALIDATION_ERRORS.some(pattern => pattern.pattern.test(message));
}

/**
 * Get a short error summary (title only)
 */
export function getErrorSummary(error: unknown): string {
  return getUserFriendlyError(error).title;
}

/**
 * Create a user-friendly error from a custom message
 *
 * Useful for creating consistent error objects in application code
 */
export function createUserError(
  title: string,
  message: string,
  action?: string,
  technical?: string
): UserFriendlyError {
  return {
    title,
    message,
    action,
    technical: technical ? sanitizeTextForStorage(technical) : undefined,
  };
}
