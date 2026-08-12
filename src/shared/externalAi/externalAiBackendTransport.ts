import { safeInvoke } from "../../platform/tauri";
import { createExternalAiGateway } from "./internal/aiGateway";
import type {
  ExternalAiGateway,
  ExternalAiProvider,
  ExternalAiResponse,
  ExternalAiSettings,
  ExternalAiTransport,
  PreparedExternalAiRequest,
} from "./externalAiTypes";

const ENABLED_PROVIDERS = [
  "open_ai",
  "anthropic",
  "google_gemini",
  "github_copilot",
  "custom",
] as const;

type EnabledExternalAiProvider = (typeof ENABLED_PROVIDERS)[number];

export interface ExternalAiRuntimeConfig {
  enabled: boolean;
  provider: ExternalAiProvider;
  require_payload_preview: boolean;
  allow_sensitive_payloads: boolean;
  redaction: {
    enabled: boolean;
  };
  log_requests_locally: boolean;
}

export interface BackendExternalAiGateway extends ExternalAiGateway {
  cancelActive(): Promise<void>;
}

interface ConfigEnvelope {
  external_ai?: unknown;
}

interface RawExternalAiConfig {
  enabled?: unknown;
  provider?: unknown;
  enabled_providers?: unknown;
  require_payload_preview?: unknown;
  allow_sensitive_payloads?: unknown;
  redaction?: unknown;
  log_requests_locally?: unknown;
}

const PROVIDER_LABELS: Record<EnabledExternalAiProvider, string> = {
  open_ai: "OpenAI",
  anthropic: "Anthropic",
  google_gemini: "Google Gemini",
  github_copilot: "GitHub Copilot",
  custom: "Custom provider",
};

function isEnabledProvider(value: unknown): value is EnabledExternalAiProvider {
  return ENABLED_PROVIDERS.includes(value as EnabledExternalAiProvider);
}

function normalizeProvider(value: unknown): ExternalAiProvider {
  if (value === "none") return "none";
  return isEnabledProvider(value) ? value : "none";
}

function normalizeEnabledProviders(value: unknown): EnabledExternalAiProvider[] {
  if (!Array.isArray(value)) return [];

  const seen = new Set<EnabledExternalAiProvider>();
  return value.filter((provider): provider is EnabledExternalAiProvider => {
    if (!isEnabledProvider(provider) || seen.has(provider)) return false;
    seen.add(provider);
    return true;
  });
}

function readObject(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

export function providerLabel(provider: ExternalAiProvider): string {
  return provider === "none" ? "Outside AI" : PROVIDER_LABELS[provider];
}

export function normalizeExternalAiRuntimeConfig(
  rawConfig: unknown,
): ExternalAiRuntimeConfig {
  const raw = readObject(rawConfig) as RawExternalAiConfig;
  const configuredProvider = normalizeProvider(raw.provider);
  const enabledProviders = normalizeEnabledProviders(
    raw.enabled_providers,
  );
  const legacyEnabledProviders =
    enabledProviders.length > 0
      ? enabledProviders
      : raw.enabled === true && configuredProvider !== "none"
        ? [configuredProvider]
        : [];
  const provider = legacyEnabledProviders[0] ?? "none";
  const redaction = readObject(raw.redaction);

  return {
    enabled: raw.enabled === true && provider !== "none",
    provider,
    require_payload_preview: raw.require_payload_preview !== false,
    allow_sensitive_payloads: raw.allow_sensitive_payloads === true,
    redaction: {
      enabled: redaction.enabled !== false,
    },
    log_requests_locally: raw.log_requests_locally !== false,
  };
}

export async function loadExternalAiRuntimeConfig(): Promise<ExternalAiRuntimeConfig> {
  const config = await safeInvoke<ConfigEnvelope>(
    "get_config",
    {},
    { logContext: "Load outside AI settings", silent: true },
  );
  return normalizeExternalAiRuntimeConfig(config.external_ai);
}

export function toGatewaySettings(
  config: ExternalAiRuntimeConfig,
): ExternalAiSettings {
  return {
    enabled: config.enabled,
    provider: config.provider,
    requirePayloadPreview: config.require_payload_preview,
    allowSensitivePayloads: config.allow_sensitive_payloads,
    redaction: {
      enabled: config.redaction.enabled,
    },
    logRequestsLocally: config.log_requests_locally,
  };
}

function readApprovalId(value: unknown): string {
  const approvalId =
    value && typeof value === "object" && "approvalId" in value
      ? (value as { approvalId?: unknown }).approvalId
      : null;
  if (typeof approvalId !== "string" || !approvalId.trim()) {
    throw new Error("Outside AI approval could not be verified.");
  }
  return approvalId;
}

type CancelOutcome = "cancelled" | "ambiguous" | "already_completed";

function readCancelOutcome(value: unknown): CancelOutcome {
  const outcome =
    value && typeof value === "object" && "outcome" in value
      ? (value as { outcome?: unknown }).outcome
      : null;
  if (
    outcome !== "cancelled" &&
    outcome !== "ambiguous" &&
    outcome !== "already_completed"
  ) {
    throw new Error("Outside AI cancellation could not be verified.");
  }
  return outcome;
}

function createBackendTransport(): {
  transport: ExternalAiTransport;
  cancelActive: () => Promise<void>;
} {
  let approvalId: string | null = null;
  let cancelRequested = false;
  let cancelPromise: Promise<CancelOutcome> | null = null;
  let preparation: Promise<void> | null = null;
  let activeSend: Promise<ExternalAiResponse> | null = null;

  const cancelApproval = (id: string) => {
    cancelPromise ??= safeInvoke<unknown>(
      "cancel_external_ai_request",
      { approvalId: id },
      { logContext: "Cancel reviewed outside AI request" },
    ).then(readCancelOutcome);
    return cancelPromise;
  };

  return {
    cancelActive: async () => {
      cancelRequested = true;
      await preparation?.catch(() => undefined);
      if (!approvalId) return;
      const outcome = await cancelApproval(approvalId);
      const send = activeSend;
      if (send) await send.catch(() => undefined);
      if (outcome === "ambiguous") {
        throw new Error(
          "Outside AI cancellation is recorded, but the provider may have received the request. Do not retry.",
        );
      }
      if (outcome === "already_completed") {
        throw new Error(
          "The Outside AI request completed before cancellation. Do not retry.",
        );
      }
    },
    transport: {
      async send(
        request: PreparedExternalAiRequest,
      ): Promise<ExternalAiResponse> {
        preparation = safeInvoke<unknown>(
          "prepare_external_ai_request",
          { request },
          { logContext: "Prepare reviewed outside AI request" },
        ).then((result) => {
          approvalId = readApprovalId(result);
        });
        try {
          await preparation;
          const preparedApprovalId = approvalId;
          if (!preparedApprovalId) {
            throw new Error("Outside AI approval could not be verified.");
          }
          if (cancelRequested) {
            await cancelApproval(preparedApprovalId);
            throw new Error("Outside AI request cancelled.");
          }

          const send = safeInvoke<ExternalAiResponse>(
            "send_external_ai_request",
            { approvalId: preparedApprovalId, request },
            { logContext: "Send reviewed outside AI request" },
          );
          activeSend = send;
          try {
            return await send;
          } finally {
            if (activeSend === send) activeSend = null;
          }
        } finally {
          preparation = null;
        }
      },
    },
  };
}

export function createBackendExternalAiGateway(
  config: ExternalAiRuntimeConfig,
): BackendExternalAiGateway {
  const { transport, cancelActive } = createBackendTransport();
  const gateway = createExternalAiGateway(
    toGatewaySettings(config),
    transport,
  );

  return {
    send: (request) => gateway.send(request),
    cancelActive,
  };
}
