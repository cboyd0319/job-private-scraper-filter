import { describe, expect, it } from "vitest";
import { DEFAULT_EXTERNAL_AI_CONFIG, type Config } from "./SettingsConfig";
import {
  getCredentialValidationError,
  type CredentialKey,
  type CredentialStatusMap,
  type CredentialStatusState,
  type Credentials,
} from "../credentials/SettingsCredentials";
import { makeConfig as makeBaseConfig } from "../SettingsPage.testFixtures";

function makeConfig(): Config {
  const config = makeBaseConfig();
  config.keywords_boost = [];
  config.salary_floor_usd = 0;
  return config;
}

function makeCredentials(overrides: Partial<Credentials> = {}): Credentials {
  return {
    slack_webhook: "",
    smtp_password: "",
    discord_webhook: "",
    teams_webhook: "",
    telegram_bot_token: "",
    usajobs_api_key: "",
    external_ai_openai_api_key: "",
    external_ai_anthropic_api_key: "",
    external_ai_google_api_key: "",
    external_ai_github_copilot_api_key: "",
    external_ai_custom_api_key: "",
    ...overrides,
  };
}

function statusValue(state: CredentialStatusState) {
  return {
    exists: state === "saved" || state === "needs_attention",
    available: state !== "expected" && state !== "needs_attention",
    state,
  };
}

function makeCredentialStatus(
  overrides: Partial<Record<CredentialKey, CredentialStatusState>> = {},
): CredentialStatusMap {
  return {
    slack_webhook: statusValue(overrides.slack_webhook ?? "empty"),
    smtp_password: statusValue(overrides.smtp_password ?? "empty"),
    discord_webhook: statusValue(overrides.discord_webhook ?? "empty"),
    teams_webhook: statusValue(overrides.teams_webhook ?? "empty"),
    telegram_bot_token: statusValue(overrides.telegram_bot_token ?? "empty"),
    usajobs_api_key: statusValue(overrides.usajobs_api_key ?? "empty"),
    external_ai_openai_api_key: statusValue(
      overrides.external_ai_openai_api_key ?? "empty",
    ),
    external_ai_anthropic_api_key: statusValue(
      overrides.external_ai_anthropic_api_key ?? "empty",
    ),
    external_ai_google_api_key: statusValue(
      overrides.external_ai_google_api_key ?? "empty",
    ),
    external_ai_github_copilot_api_key: statusValue(
      overrides.external_ai_github_copilot_api_key ?? "empty",
    ),
    external_ai_custom_api_key: statusValue(
      overrides.external_ai_custom_api_key ?? "empty",
    ),
  };
}

describe("getCredentialValidationError", () => {
  it("does not treat expected Telegram details as confirmed saved secrets", () => {
    const config = makeConfig();
    config.alerts.telegram = { enabled: true, chat_id: "12345" };

    expect(
      getCredentialValidationError(
        makeCredentials(),
        config,
        makeCredentialStatus({ telegram_bot_token: "expected" }),
      ),
    ).toEqual({
      title: "Finish Telegram alerts",
      message:
        "Add the Telegram details shown below, or turn Telegram alerts off.",
    });
  });

  it("does not treat expected USAJobs details as confirmed saved secrets", () => {
    const config = makeConfig();
    config.usajobs = {
      ...config.usajobs,
      enabled: true,
      email: "person@example.com",
    };

    expect(
      getCredentialValidationError(
        makeCredentials(),
        config,
        makeCredentialStatus({ usajobs_api_key: "expected" }),
      ),
    ).toEqual({
      title: "Finish USAJobs scheduled checks",
      message:
        "Add the USAJobs email and access code shown below, or turn USAJobs scheduled checks off.",
    });
  });

  it("accepts confirmed saved or newly entered required secrets", () => {
    const config = makeConfig();
    config.alerts.telegram = { enabled: true, chat_id: "12345" };
    config.usajobs = {
      ...config.usajobs,
      enabled: true,
      email: "person@example.com",
    };

    expect(
      getCredentialValidationError(
        makeCredentials(),
        config,
        makeCredentialStatus({
          telegram_bot_token: "saved",
          usajobs_api_key: "saved",
        }),
      ),
    ).toBeNull();

    expect(
      getCredentialValidationError(
        makeCredentials({
          telegram_bot_token: "new-telegram-token",
          usajobs_api_key: "new-usajobs-key",
        }),
        config,
        makeCredentialStatus({
          telegram_bot_token: "expected",
          usajobs_api_key: "expected",
        }),
      ),
    ).toBeNull();
  });

  it("ignores retired scheduled source flags from legacy config", () => {
    const config = makeConfig();
    config.builtin.enabled = true;
    config.dice.enabled = true;
    config.simplyhired.enabled = true;
    config.glassdoor.enabled = true;

    expect(
      getCredentialValidationError(
        makeCredentials(),
        config,
        makeCredentialStatus(),
      ),
    ).toBeNull();
  });

  it("requires a saved or newly entered key for each enabled outside AI provider", () => {
    const config = makeConfig();
    config.external_ai = {
      ...DEFAULT_EXTERNAL_AI_CONFIG,
      enabled: true,
      provider: "open_ai",
      enabled_providers: ["open_ai", "anthropic"],
      provider_order: [
        "open_ai",
        "anthropic",
        "google_gemini",
        "github_copilot",
        "custom",
      ],
    };

    expect(
      getCredentialValidationError(
        makeCredentials(),
        config,
        makeCredentialStatus({ external_ai_openai_api_key: "saved" }),
      ),
    ).toEqual({
      title: "Add Anthropic key",
      message:
        "Paste the provider key, or turn that provider off. The key is stored in the local secure vault.",
    });

    expect(
      getCredentialValidationError(
        makeCredentials({ external_ai_anthropic_api_key: "new-anthropic-key" }),
        config,
        makeCredentialStatus({ external_ai_openai_api_key: "saved" }),
      ),
    ).toBeNull();
  });
});
