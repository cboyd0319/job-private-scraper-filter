import { invoke } from "../../../platform/tauri";
import {
  validateDiscordWebhook,
  validateSlackWebhook,
  validateTeamsWebhook,
} from "./notificationConnectionValidation";
import {
  EXTERNAL_AI_PROVIDER_CREDENTIAL_KEYS,
  EXTERNAL_AI_PROVIDER_LABELS,
  type ExternalAiCredentialKey,
} from "../external-ai/externalAiProviders";
import type { Config } from "../config/SettingsConfig";

// Credentials are stored through the backend CredentialService, not config.
export interface Credentials {
  slack_webhook: string;
  smtp_password: string;
  discord_webhook: string;
  teams_webhook: string;
  telegram_bot_token: string;
  usajobs_api_key: string;
  external_ai_openai_api_key: string;
  external_ai_anthropic_api_key: string;
  external_ai_google_api_key: string;
  external_ai_github_copilot_api_key: string;
  external_ai_custom_api_key: string;
}

// Credential key names must match the backend CredentialKey enum.
export type CredentialKey =
  | "slack_webhook"
  | "smtp_password"
  | "discord_webhook"
  | "teams_webhook"
  | "telegram_bot_token"
  | "usajobs_api_key"
  | ExternalAiCredentialKey;

export interface CredentialStatusEntry {
  key: CredentialKey;
  exists: boolean;
  available?: boolean;
}

export type CredentialUnlockMode = "system" | "passphrase";

export interface CredentialUnlockStatus {
  mode: CredentialUnlockMode;
  configured: boolean;
  unlocked: boolean;
}

export interface CredentialStatusValue {
  exists: boolean;
  available: boolean;
  state: CredentialStatusState;
}

export type CredentialStatusState =
  "empty" | "expected" | "saved" | "needs_attention";

export type CredentialStatusMap = Record<CredentialKey, CredentialStatusValue>;

export interface CredentialValidationError {
  title: string;
  message: string;
}

export async function storeCredential(
  key: CredentialKey,
  value: string,
): Promise<void> {
  await invoke("store_credential", { key, value });
}

export async function hasCredential(key: CredentialKey): Promise<boolean> {
  return await invoke<boolean>("has_credential", { key });
}

export async function getCredentialStatusEntries(): Promise<
  CredentialStatusEntry[]
> {
  return await invoke<CredentialStatusEntry[]>("get_credential_status");
}

export async function getCredentialUnlockStatus(): Promise<CredentialUnlockStatus> {
  const status = await invoke<unknown>("get_credential_unlock_status");
  if (!isCredentialUnlockStatus(status)) {
    throw new Error("Credential lock status is unavailable");
  }
  return status;
}

export async function enableCredentialPassphrase(
  passphrase: string,
): Promise<void> {
  await invoke("enable_credential_passphrase", { passphrase });
}

export async function unlockCredentialVault(passphrase: string): Promise<void> {
  await invoke("unlock_credential_vault", { passphrase });
}

export async function disableCredentialPassphrase(
  passphrase: string,
): Promise<void> {
  await invoke("disable_credential_passphrase", { passphrase });
}

export function credentialExists(
  credentialStatus: CredentialStatusMap,
  key: CredentialKey,
): boolean {
  const status = credentialStatus[key];
  return status.state === "saved" || (status.available && status.exists);
}

export function credentialIsExpected(
  credentialStatus: CredentialStatusMap,
  key: CredentialKey,
): boolean {
  return credentialStatus[key].state === "expected";
}

export function credentialNeedsAttention(
  credentialStatus: CredentialStatusMap,
  key: CredentialKey,
): boolean {
  return credentialStatus[key].state === "needs_attention";
}

export const isValidSlackWebhook = (url: string): boolean =>
  validateSlackWebhook(url) === undefined;

export const isValidDiscordWebhook = (url: string): boolean =>
  validateDiscordWebhook(url) === undefined;

export const isValidTeamsWebhook = (url: string): boolean =>
  validateTeamsWebhook(url) === undefined;

export function getCredentialValidationError(
  credentials: Credentials,
  config?: Config,
  credentialStatus?: CredentialStatusMap,
): CredentialValidationError | null {
  if (validateSlackWebhook(credentials.slack_webhook)) {
    return {
      title: "Check Slack connection link",
      message:
        "Paste the full Slack connection link copied from Slack. If you are not sure, leave it blank and set it up later.",
    };
  }

  if (validateDiscordWebhook(credentials.discord_webhook)) {
    return {
      title: "Check Discord connection link",
      message:
        "Paste the full Discord connection link copied from Discord. If you are not sure, leave it blank and set it up later.",
    };
  }

  if (validateTeamsWebhook(credentials.teams_webhook)) {
    return {
      title: "Check Teams connection link",
      message:
        "Paste the full Teams connection link copied from Teams. If you are not sure, leave it blank and set it up later.",
    };
  }

  if (config?.alerts.slack?.enabled) {
    const hasSlackConnection =
      Boolean(
        credentialStatus && credentialExists(credentialStatus, "slack_webhook"),
      ) || Boolean(credentials.slack_webhook.trim());

    if (!hasSlackConnection) {
      return {
        title: "Finish Slack alerts",
        message:
          "Paste the Slack connection link again, or turn Slack alerts off.",
      };
    }
  }

  if (config?.alerts.email?.enabled) {
    const hasEmailPassword =
      Boolean(
        credentialStatus && credentialExists(credentialStatus, "smtp_password"),
      ) || Boolean(credentials.smtp_password.trim());

    if (!hasEmailPassword) {
      return {
        title: "Finish email alerts",
        message: "Add the email app password, or turn email alerts off.",
      };
    }
  }

  if (config?.alerts.discord?.enabled) {
    const hasDiscordConnection =
      Boolean(
        credentialStatus &&
        credentialExists(credentialStatus, "discord_webhook"),
      ) || Boolean(credentials.discord_webhook.trim());

    if (!hasDiscordConnection) {
      return {
        title: "Finish Discord alerts",
        message:
          "Paste the Discord connection link again, or turn Discord alerts off.",
      };
    }
  }

  if (config?.alerts.teams?.enabled) {
    const hasTeamsConnection =
      Boolean(
        credentialStatus && credentialExists(credentialStatus, "teams_webhook"),
      ) || Boolean(credentials.teams_webhook.trim());

    if (!hasTeamsConnection) {
      return {
        title: "Finish Teams alerts",
        message:
          "Paste the Teams connection link again, or turn Teams alerts off.",
      };
    }
  }

  if (config?.alerts.telegram?.enabled) {
    const hasAlertCode =
      Boolean(
        credentialStatus &&
        credentialExists(credentialStatus, "telegram_bot_token"),
      ) || Boolean(credentials.telegram_bot_token.trim());
    const hasDestination = Boolean(config.alerts.telegram.chat_id?.trim());

    if (!hasAlertCode || !hasDestination) {
      return {
        title: "Finish Telegram alerts",
        message:
          "Add the Telegram details shown below, or turn Telegram alerts off.",
      };
    }
  }

  if (config?.usajobs?.enabled) {
    const hasAccessCode =
      Boolean(
        credentialStatus &&
        credentialExists(credentialStatus, "usajobs_api_key"),
      ) || Boolean(credentials.usajobs_api_key.trim());
    const hasEmail = Boolean(config.usajobs.email?.trim());

    if (!hasEmail || !hasAccessCode) {
      return {
        title: "Finish USAJobs scheduled checks",
        message:
          "Add the USAJobs email and access code shown below, or turn USAJobs scheduled checks off.",
      };
    }
  }

  if (config?.external_ai?.enabled) {
    if (config.external_ai.provider === "none") {
      return {
        title: "Choose an outside AI provider",
        message:
          "Choose at least one outside AI provider, or turn outside AI off.",
      };
    }

    if (!config.external_ai.require_payload_preview) {
      return {
        title: "Keep outside AI review on",
        message:
          "JobSentinel must show what would be sent before any outside AI request.",
      };
    }

    if (!config.external_ai.redaction.enabled) {
      return {
        title: "Keep outside AI redaction on",
        message:
          "JobSentinel must remove private details before outside AI is used.",
      };
    }

    if (config.external_ai.enabled_providers.includes("custom")) {
      try {
        const endpoint = new URL(config.external_ai.custom_endpoint);
        if (
          endpoint.protocol !== "https:" ||
          endpoint.username ||
          endpoint.password
        ) {
          throw new Error("invalid custom endpoint");
        }
      } catch {
        return {
          title: "Check outside AI endpoint",
          message:
            "Use a full HTTPS endpoint from your provider, without a username or password in the link.",
        };
      }
    }

    for (const provider of config.external_ai.enabled_providers) {
      const credentialKey = EXTERNAL_AI_PROVIDER_CREDENTIAL_KEYS[provider];
      const hasProviderKey =
        Boolean(
          credentialStatus && credentialExists(credentialStatus, credentialKey),
        ) || Boolean(credentials[credentialKey].trim());

      if (!hasProviderKey) {
        return {
          title: `Add ${EXTERNAL_AI_PROVIDER_LABELS[provider]} key`,
          message:
            "Paste the provider key, or turn that provider off. The key is stored in the local secure vault.",
        };
      }
    }
  }

  return null;
}

function isCredentialUnlockStatus(
  value: unknown,
): value is CredentialUnlockStatus {
  if (!value || typeof value !== "object") {
    return false;
  }

  const status = value as Partial<CredentialUnlockStatus>;
  return (
    (status.mode === "system" || status.mode === "passphrase") &&
    typeof status.configured === "boolean" &&
    typeof status.unlocked === "boolean"
  );
}
