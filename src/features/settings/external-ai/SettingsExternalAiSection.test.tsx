import { useState } from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { SettingsExternalAiSection } from "./SettingsExternalAiSection";
import type { Config } from "../config/SettingsConfig";
import type {
  CredentialKey,
  CredentialStatusMap,
  Credentials,
} from "../credentials/SettingsCredentials";
import { makeConfig } from "../SettingsPage.testFixtures";

function makeCredentials(): Credentials {
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
  };
}

function makeCredentialStatus(): CredentialStatusMap {
  const status = {} as CredentialStatusMap;
  (Object.keys(makeCredentials()) as CredentialKey[]).forEach((key) => {
    status[key] = {
      key,
      exists: false,
      available: true,
      state: "empty",
    };
  });
  return status;
}

function ExternalAiHarness() {
  const [config, setConfig] = useState<Config>(makeConfig);
  const [credentials, setCredentials] = useState<Credentials>(makeCredentials);

  return (
    <>
      <SettingsExternalAiSection
        config={config}
        credentialStatus={makeCredentialStatus()}
        credentials={credentials}
        onConfigChange={setConfig}
        onCredentialsChange={setCredentials}
      />
      <output data-testid="external-ai-state">
        {JSON.stringify(config.external_ai)}
      </output>
      <output data-testid="external-ai-credentials">
        {JSON.stringify(credentials)}
      </output>
    </>
  );
}

describe("SettingsExternalAiSection", () => {
  it("configures multiple outside AI providers without storing keys in config", async () => {
    const user = userEvent.setup();
    render(<ExternalAiHarness />);

    expect(screen.getByText("OpenAI")).toBeInTheDocument();
    expect(screen.getByText("Anthropic")).toBeInTheDocument();
    expect(screen.getByText("Google Gemini")).toBeInTheDocument();
    expect(screen.getByText("GitHub Copilot")).toBeInTheDocument();
    expect(screen.getByText("Durable outside AI activity")).toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", { name: "Save request history" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("checkbox", { name: /OpenAI/ }));
    await user.click(screen.getByRole("checkbox", { name: /Anthropic/ }));

    const anthropicProvider = screen.getByRole("group", {
      name: "Anthropic outside AI provider",
    });
    await user.type(
      within(anthropicProvider).getByPlaceholderText("Paste your provider key"),
      "test-anthropic-provider-value",
    );
    await user.click(
      within(anthropicProvider).getByRole("button", { name: "Up" }),
    );

    const externalAi = JSON.parse(
      screen.getByTestId("external-ai-state").textContent ?? "{}",
    );
    const credentials = JSON.parse(
      screen.getByTestId("external-ai-credentials").textContent ?? "{}",
    );

    expect(externalAi).toMatchObject({
      enabled: true,
      provider: "anthropic",
      enabled_providers: ["anthropic", "open_ai"],
      require_payload_preview: true,
      redaction: { enabled: true },
    });
    expect(externalAi.provider_models).toEqual({});
    expect(JSON.stringify(externalAi)).not.toContain(
      "test-anthropic-provider-value",
    );
    expect(credentials.external_ai_anthropic_api_key).toBe(
      "test-anthropic-provider-value",
    );
  });
});
