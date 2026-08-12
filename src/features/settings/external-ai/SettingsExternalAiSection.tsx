import type { Dispatch, SetStateAction } from "react";
import { HelpIcon } from "../../../ui/HelpIcon";
import {
  EXTERNAL_AI_PROVIDER_CREDENTIAL_KEYS,
  EXTERNAL_AI_PROVIDER_DESCRIPTIONS,
  EXTERNAL_AI_PROVIDER_LABELS,
  normalizeEnabledExternalAiProviders,
  normalizeExternalAiProviderOrder,
  type EnabledExternalAiProvider,
} from "./externalAiProviders";
import type { Config } from "../config/SettingsConfig";
import {
  credentialExists,
  type CredentialStatusMap,
  type Credentials,
} from "../credentials/SettingsCredentials";
import { SettingsExternalAiRequestHistory } from "./SettingsExternalAiRequestHistory";

interface SettingsExternalAiSectionProps {
  config: Config;
  credentialStatus: CredentialStatusMap;
  credentials: Credentials;
  onConfigChange: Dispatch<SetStateAction<Config | null>>;
  onCredentialsChange: Dispatch<SetStateAction<Credentials>>;
}

function providerLabel(provider: EnabledExternalAiProvider): string {
  return EXTERNAL_AI_PROVIDER_LABELS[provider];
}

function moveProvider(
  providers: EnabledExternalAiProvider[],
  provider: EnabledExternalAiProvider,
  direction: -1 | 1,
): EnabledExternalAiProvider[] {
  const index = providers.indexOf(provider);
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= providers.length) {
    return providers;
  }

  const next = [...providers];
  const currentProvider = next[index];
  const targetProvider = next[nextIndex];
  if (!currentProvider || !targetProvider) return providers;

  next[index] = targetProvider;
  next[nextIndex] = currentProvider;
  return next;
}

function primaryProvider(
  enabledProviders: EnabledExternalAiProvider[],
): Config["external_ai"]["provider"] {
  return enabledProviders[0] ?? "none";
}

export function SettingsExternalAiSection({
  config,
  credentialStatus,
  credentials,
  onConfigChange,
  onCredentialsChange,
}: SettingsExternalAiSectionProps) {
  const providerOrder = normalizeExternalAiProviderOrder(
    config.external_ai.provider_order,
  );
  const enabledProviders = normalizeEnabledExternalAiProviders(
    config.external_ai.enabled_providers,
  );

  const updateExternalAi = (updates: Partial<Config["external_ai"]>) => {
    onConfigChange((previous) =>
      previous
        ? {
            ...previous,
            external_ai: {
              ...previous.external_ai,
              ...updates,
            },
          }
        : previous,
    );
  };

  const updateProviderEnabled = (
    provider: EnabledExternalAiProvider,
    enabled: boolean,
  ) => {
    const enabledSet = new Set(enabledProviders);
    if (enabled) {
      enabledSet.add(provider);
    } else {
      enabledSet.delete(provider);
    }

    const nextEnabledProviders = providerOrder.filter((item) =>
      enabledSet.has(item),
    );

    updateExternalAi({
      enabled: nextEnabledProviders.length > 0,
      provider: primaryProvider(nextEnabledProviders),
      enabled_providers: nextEnabledProviders,
      provider_order: providerOrder,
      require_payload_preview: true,
      redaction: { enabled: true },
    });
  };

  const updateProviderOrder = (
    provider: EnabledExternalAiProvider,
    direction: -1 | 1,
  ) => {
    const nextProviderOrder = moveProvider(providerOrder, provider, direction);
    const nextEnabledProviders = nextProviderOrder.filter((item) =>
      enabledProviders.includes(item),
    );

    updateExternalAi({
      provider_order: nextProviderOrder,
      enabled_providers: nextEnabledProviders,
      provider: primaryProvider(nextEnabledProviders),
    });
  };

  const updateProviderModel = (
    provider: EnabledExternalAiProvider,
    model: string,
  ) => {
    const providerModels = {
      ...config.external_ai.provider_models,
      [provider]: model,
    };

    updateExternalAi({
      model:
        provider === config.external_ai.provider
          ? model
          : config.external_ai.model,
      provider_models: providerModels,
    });
  };

  return (
    <section className="mb-6" aria-labelledby="external-ai-settings-heading">
      <h3
        id="external-ai-settings-heading"
        className="mb-3 flex items-center gap-2 font-medium text-surface-800 dark:text-surface-200"
      >
        Outside AI
        <HelpIcon text="JobSentinel works without outside AI. If you turn it on, every request still shows what would be sent before anything leaves this device." />
      </h3>

      <div className="space-y-4 rounded-lg border border-surface-200 bg-surface-50 p-4 dark:border-surface-700 dark:bg-surface-800">
        <div>
          <p className="text-sm font-medium text-surface-800 dark:text-surface-100">
            Optional help for drafts and explanations
          </p>
          <p className="mt-1 text-xs text-surface-600 dark:text-surface-300">
            Local tools stay available. Outside AI is only used after you review
            and approve the exact details.
          </p>
        </div>

        <fieldset className="space-y-3">
          <legend className="text-sm font-medium text-surface-700 dark:text-surface-200">
            Providers and order
          </legend>

          {providerOrder.map((provider, index) => {
            const credentialKey =
              EXTERNAL_AI_PROVIDER_CREDENTIAL_KEYS[provider];
            const isEnabled = enabledProviders.includes(provider);
            const hasSavedProviderKey = credentialExists(
              credentialStatus,
              credentialKey,
            );

            return (
              <div
                key={provider}
                role="group"
                aria-label={`${providerLabel(provider)} outside AI provider`}
                className="rounded-md border border-surface-200 bg-white p-3 dark:border-surface-700 dark:bg-surface-900"
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <label className="flex min-w-0 items-start gap-3 text-sm text-surface-700 dark:text-surface-200">
                    <input
                      type="checkbox"
                      checked={isEnabled}
                      onChange={(event) =>
                        updateProviderEnabled(provider, event.target.checked)
                      }
                      className="mt-1 rounded border-surface-300 text-sentinel-600 focus:ring-sentinel-500"
                    />
                    <span>
                      <span className="block font-medium text-surface-900 dark:text-surface-100">
                        {providerLabel(provider)}
                      </span>
                      <span className="block text-xs text-surface-600 dark:text-surface-300">
                        {EXTERNAL_AI_PROVIDER_DESCRIPTIONS[provider]}
                      </span>
                    </span>
                  </label>

                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={() => updateProviderOrder(provider, -1)}
                      disabled={index === 0}
                      className="rounded border border-surface-300 px-2 py-1 text-xs text-surface-700 disabled:opacity-50 dark:border-surface-600 dark:text-surface-200"
                    >
                      Up
                    </button>
                    <button
                      type="button"
                      onClick={() => updateProviderOrder(provider, 1)}
                      disabled={index === providerOrder.length - 1}
                      className="rounded border border-surface-300 px-2 py-1 text-xs text-surface-700 disabled:opacity-50 dark:border-surface-600 dark:text-surface-200"
                    >
                      Down
                    </button>
                  </div>
                </div>

                {isEnabled && (
                  <div className="mt-3 grid gap-3 md:grid-cols-2">
                    <label className="block text-sm">
                      <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
                        Provider key
                      </span>
                      <input
                        value={credentials[credentialKey]}
                        onChange={(event) =>
                          onCredentialsChange((previous) => ({
                            ...previous,
                            [credentialKey]: event.target.value,
                          }))
                        }
                        placeholder={
                          hasSavedProviderKey
                            ? "Saved locally. Paste a new key only to replace it."
                            : "Paste your provider key"
                        }
                        type="password"
                        autoComplete="off"
                        className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus:ring-1 focus:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-900 dark:text-surface-100"
                      />
                      <span className="mt-1 block text-xs text-surface-500 dark:text-surface-400">
                        {hasSavedProviderKey
                          ? "A key is saved in the local secure vault."
                          : "Keys are stored locally in the secure vault, not in settings backups."}
                      </span>
                    </label>

                    <label className="block text-sm">
                      <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
                        Model name
                      </span>
                      <input
                        value={
                          config.external_ai.provider_models[provider] ?? ""
                        }
                        onChange={(event) =>
                          updateProviderModel(provider, event.target.value)
                        }
                        placeholder="Optional provider model name"
                        className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus:ring-1 focus:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-900 dark:text-surface-100"
                      />
                    </label>
                  </div>
                )}
              </div>
            );
          })}
        </fieldset>

        {enabledProviders.includes("custom") && (
          <label className="block text-sm">
            <span className="mb-1 block font-medium text-surface-700 dark:text-surface-200">
              Custom provider endpoint
            </span>
            <input
              value={config.external_ai.custom_endpoint}
              onChange={(event) =>
                updateExternalAi({ custom_endpoint: event.target.value })
              }
              placeholder="https://provider.example/secure-ai"
              className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 text-sm text-surface-900 focus:border-sentinel-500 focus:ring-1 focus:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-900 dark:text-surface-100"
            />
          </label>
        )}

        <div className="flex flex-col gap-3 text-sm text-surface-700 dark:text-surface-200 sm:flex-row">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={config.external_ai.require_payload_preview}
              readOnly
              className="rounded border-surface-300 text-sentinel-600"
            />
            Preview before sending
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={config.external_ai.redaction.enabled}
              readOnly
              className="rounded border-surface-300 text-sentinel-600"
            />
            Redaction on
          </label>
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={config.external_ai.allow_sensitive_payloads}
              onChange={(event) =>
                updateExternalAi({
                  allow_sensitive_payloads: event.target.checked,
                })
              }
              className="rounded border-surface-300 text-sentinel-600 focus:ring-sentinel-500"
            />
            Private details after review
          </label>
        </div>

        <SettingsExternalAiRequestHistory />
      </div>
    </section>
  );
}
