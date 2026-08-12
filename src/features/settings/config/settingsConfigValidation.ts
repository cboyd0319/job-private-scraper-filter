import {
  isExternalAiProvider,
  type ExternalAiSettings,
} from "../external-ai/externalAiProviders";
import type { Config } from "./SettingsConfig";

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasStringArrayField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return (
    Array.isArray(record[field]) &&
    (record[field] as unknown[]).every((item) => typeof item === "string")
  );
}

function hasBooleanField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return typeof record[field] === "boolean";
}

function hasNumberField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return typeof record[field] === "number";
}

function hasStringField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return typeof record[field] === "string";
}

function hasOptionalStringField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return record[field] === undefined || typeof record[field] === "string";
}

function hasOptionalNullableStringField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return (
    record[field] === undefined ||
    record[field] === null ||
    typeof record[field] === "string"
  );
}

function hasOptionalNumberField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return record[field] === undefined || typeof record[field] === "number";
}

function hasOptionalBooleanField(
  record: Record<string, unknown>,
  field: string,
): boolean {
  return record[field] === undefined || typeof record[field] === "boolean";
}

function recordField(
  record: Record<string, unknown>,
  field: string,
): Record<string, unknown> | null {
  const value = record[field];
  return isPlainRecord(value) ? value : null;
}

function isOptionalJobsWithGptPayload(value: unknown): boolean {
  if (value === undefined || value === null) return true;
  if (!isPlainRecord(value)) return false;

  return (
    hasStringField(value, "endpoint") &&
    hasStringArrayField(value, "titles") &&
    hasOptionalNullableStringField(value, "location") &&
    hasBooleanField(value, "remote_only") &&
    hasNumberField(value, "limit")
  );
}

function isExternalAiSettings(value: unknown): value is ExternalAiSettings {
  if (!isPlainRecord(value)) return false;
  const redaction = recordField(value, "redaction");

  return (
    hasBooleanField(value, "enabled") &&
    isExternalAiProvider(value.provider) &&
    hasStringField(value, "model") &&
    hasStringField(value, "custom_endpoint") &&
    (value.provider_order === undefined ||
      hasStringArrayField(value, "provider_order")) &&
    (value.enabled_providers === undefined ||
      hasStringArrayField(value, "enabled_providers")) &&
    (value.provider_models === undefined ||
      isPlainRecord(value.provider_models)) &&
    hasBooleanField(value, "require_payload_preview") &&
    hasBooleanField(value, "allow_sensitive_payloads") &&
    !!redaction &&
    hasBooleanField(redaction, "enabled") &&
    hasBooleanField(value, "log_requests_locally")
  );
}

export function isSettingsBackupConfig(value: unknown): value is Config {
  if (!isPlainRecord(value)) return false;

  const location = recordField(value, "location_preferences");
  const alerts = recordField(value, "alerts");
  const autoRefresh = recordField(value, "auto_refresh");
  const slack = alerts ? recordField(alerts, "slack") : null;
  const email = alerts ? recordField(alerts, "email") : null;
  const discord = alerts ? recordField(alerts, "discord") : null;
  const telegram = alerts ? recordField(alerts, "telegram") : null;
  const teams = alerts ? recordField(alerts, "teams") : null;
  const desktop = alerts ? recordField(alerts, "desktop") : null;
  const linkedin = recordField(value, "linkedin");
  const restrictedSourceAcknowledgements = recordField(
    value,
    "restricted_source_acknowledgements",
  );
  const remoteok = recordField(value, "remoteok");
  const weworkremotely = recordField(value, "weworkremotely");
  const builtin = recordField(value, "builtin");
  const hnHiring = recordField(value, "hn_hiring");
  const dice = recordField(value, "dice");
  const ycStartup = recordField(value, "yc_startup");
  const usajobs = recordField(value, "usajobs");
  const simplyhired = recordField(value, "simplyhired");
  const glassdoor = recordField(value, "glassdoor");
  const jobswithgptApproval = recordField(value, "jobswithgpt_approval");
  const externalAi = recordField(value, "external_ai");

  return (
    hasStringArrayField(value, "title_allowlist") &&
    hasStringArrayField(value, "title_blocklist") &&
    hasStringArrayField(value, "keywords_boost") &&
    hasStringArrayField(value, "keywords_exclude") &&
    hasStringArrayField(value, "preferred_companies") &&
    hasStringArrayField(value, "blocked_companies") &&
    hasNumberField(value, "salary_floor_usd") &&
    hasOptionalNumberField(value, "salary_target_usd") &&
    !!location &&
    hasBooleanField(location, "allow_remote") &&
    hasBooleanField(location, "allow_hybrid") &&
    hasBooleanField(location, "allow_onsite") &&
    hasStringArrayField(location, "cities") &&
    !!autoRefresh &&
    hasBooleanField(autoRefresh, "enabled") &&
    hasNumberField(autoRefresh, "interval_minutes") &&
    !!alerts &&
    !!slack &&
    hasBooleanField(slack, "enabled") &&
    !!email &&
    hasBooleanField(email, "enabled") &&
    hasStringField(email, "smtp_server") &&
    hasNumberField(email, "smtp_port") &&
    hasStringField(email, "smtp_username") &&
    hasStringField(email, "from_email") &&
    hasStringArrayField(email, "to_emails") &&
    hasBooleanField(email, "use_starttls") &&
    !!discord &&
    hasBooleanField(discord, "enabled") &&
    hasOptionalStringField(discord, "user_id_to_mention") &&
    !!telegram &&
    hasBooleanField(telegram, "enabled") &&
    hasOptionalStringField(telegram, "chat_id") &&
    !!teams &&
    hasBooleanField(teams, "enabled") &&
    !!desktop &&
    hasBooleanField(desktop, "enabled") &&
    hasBooleanField(desktop, "show_when_focused") &&
    hasBooleanField(desktop, "play_sound") &&
    !!linkedin &&
    hasBooleanField(linkedin, "enabled") &&
    hasStringField(linkedin, "query") &&
    hasStringField(linkedin, "location") &&
    hasBooleanField(linkedin, "remote_only") &&
    hasNumberField(linkedin, "limit") &&
    (value.restricted_source_acknowledgements === undefined ||
      (!!restrictedSourceAcknowledgements &&
        hasOptionalBooleanField(restrictedSourceAcknowledgements, "builtin") &&
        hasOptionalBooleanField(restrictedSourceAcknowledgements, "dice") &&
        hasOptionalBooleanField(
          restrictedSourceAcknowledgements,
          "simplyhired",
        ) &&
        hasOptionalBooleanField(
          restrictedSourceAcknowledgements,
          "glassdoor",
        ))) &&
    !!remoteok &&
    hasBooleanField(remoteok, "enabled") &&
    hasStringArrayField(remoteok, "tags") &&
    hasNumberField(remoteok, "limit") &&
    !!weworkremotely &&
    hasBooleanField(weworkremotely, "enabled") &&
    hasOptionalStringField(weworkremotely, "category") &&
    hasNumberField(weworkremotely, "limit") &&
    !!builtin &&
    hasBooleanField(builtin, "enabled") &&
    hasBooleanField(builtin, "remote_only") &&
    hasNumberField(builtin, "limit") &&
    !!hnHiring &&
    hasBooleanField(hnHiring, "enabled") &&
    hasBooleanField(hnHiring, "remote_only") &&
    hasNumberField(hnHiring, "limit") &&
    !!dice &&
    hasBooleanField(dice, "enabled") &&
    hasStringField(dice, "query") &&
    hasOptionalStringField(dice, "location") &&
    hasNumberField(dice, "limit") &&
    !!ycStartup &&
    hasBooleanField(ycStartup, "enabled") &&
    hasOptionalStringField(ycStartup, "query") &&
    hasBooleanField(ycStartup, "remote_only") &&
    hasNumberField(ycStartup, "limit") &&
    !!usajobs &&
    hasBooleanField(usajobs, "enabled") &&
    hasStringField(usajobs, "email") &&
    hasOptionalStringField(usajobs, "keywords") &&
    hasOptionalStringField(usajobs, "location") &&
    hasOptionalNumberField(usajobs, "radius") &&
    hasBooleanField(usajobs, "remote_only") &&
    hasOptionalNumberField(usajobs, "pay_grade_min") &&
    hasOptionalNumberField(usajobs, "pay_grade_max") &&
    hasNumberField(usajobs, "date_posted_days") &&
    hasNumberField(usajobs, "limit") &&
    !!simplyhired &&
    hasBooleanField(simplyhired, "enabled") &&
    hasStringField(simplyhired, "query") &&
    hasOptionalStringField(simplyhired, "location") &&
    hasNumberField(simplyhired, "limit") &&
    !!glassdoor &&
    hasBooleanField(glassdoor, "enabled") &&
    hasStringField(glassdoor, "query") &&
    hasOptionalStringField(glassdoor, "location") &&
    hasNumberField(glassdoor, "limit") &&
    hasStringField(value, "jobswithgpt_endpoint") &&
    !!jobswithgptApproval &&
    hasBooleanField(jobswithgptApproval, "enabled") &&
    isOptionalJobsWithGptPayload(jobswithgptApproval.payload) &&
    hasOptionalNullableStringField(jobswithgptApproval, "approved_at") &&
    (value.external_ai === undefined ||
      (!!externalAi && isExternalAiSettings(externalAi))) &&
    hasBooleanField(value, "use_resume_matching")
  );
}
