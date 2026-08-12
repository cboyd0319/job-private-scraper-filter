import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createBackendExternalAiGateway,
  normalizeExternalAiRuntimeConfig,
  providerLabel,
  toGatewaySettings,
} from "./externalAiBackendTransport";
import type { ExternalAiRequest } from "./externalAiTypes";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import("@tauri-apps/api/core");
const mockInvoke = vi.mocked(invoke);

const publicJobRequest: ExternalAiRequest = {
  feature: "job-description-summary",
  sourceJobId: 42,
  labels: ["External AI optional", "Public-data only"],
  dataCategories: ["job_posting", "public_metadata"],
  payload: {
    title: "Operations Manager",
    company: "Example Co",
  },
  redactedPayload: {
    title: "Operations Manager",
    company: "Example Co",
  },
  previewShown: true,
  userApproved: true,
};

function enabledConfig(): ReturnType<typeof normalizeExternalAiRuntimeConfig> {
  return normalizeExternalAiRuntimeConfig({
    enabled: true,
    enabled_providers: ["open_ai"],
    provider: "open_ai",
    log_requests_locally: false,
  });
}

describe("externalAiBackendTransport", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("prepares the exact request before sending its one-time approval", async () => {
    mockInvoke.mockImplementation((command) => {
      if (command === "prepare_external_ai_request") {
        return Promise.resolve({ approvalId: "approval-123" });
      }
      if (command === "send_external_ai_request") {
        return Promise.resolve({ text: "summary" });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    const gateway = createBackendExternalAiGateway(enabledConfig());
    await expect(gateway.send(publicJobRequest)).resolves.toEqual({
      text: "summary",
    });

    const prepareCall = mockInvoke.mock.calls.find(
      ([command]) => command === "prepare_external_ai_request",
    );
    const sendCall = mockInvoke.mock.calls.find(
      ([command]) => command === "send_external_ai_request",
    );
    expect(prepareCall?.[1]).toEqual({
      request: expect.any(Object),
    });
    expect(sendCall?.[1]).toEqual({
      approvalId: "approval-123",
      request: expect.any(Object),
    });
    expect(
      (sendCall?.[1] as { request?: unknown } | undefined)?.request,
    ).toBe((prepareCall?.[1] as { request?: unknown } | undefined)?.request);
  });

  it("rejects an invalid backend approval without sending", async () => {
    mockInvoke.mockResolvedValue({ approvalId: " " });

    const gateway = createBackendExternalAiGateway(enabledConfig());

    await expect(gateway.send(publicJobRequest)).rejects.toThrow(
      "Outside AI approval could not be verified.",
    );
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith(
      "prepare_external_ai_request",
      expect.anything(),
    );
  });

  it("waits for a pending preparation and cancels it before sending", async () => {
    let finishPreparation:
      | ((approval: { approvalId: string }) => void)
      | undefined;
    mockInvoke.mockImplementation((command) => {
      if (command === "prepare_external_ai_request") {
        return new Promise((resolve) => {
          finishPreparation = resolve;
        });
      }
      if (command === "cancel_external_ai_request") {
        return Promise.resolve({ outcome: "cancelled" });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    const gateway = createBackendExternalAiGateway(enabledConfig());
    const send = gateway.send(publicJobRequest);
    let cancellationFinished = false;
    const cancellation = gateway.cancelActive().finally(() => {
      cancellationFinished = true;
    });
    await Promise.resolve();
    expect(cancellationFinished).toBe(false);

    finishPreparation?.({ approvalId: "approval-pending" });
    await expect(cancellation).resolves.toBeUndefined();
    await expect(send).rejects.toThrow("Outside AI request cancelled.");
    expect(mockInvoke).toHaveBeenCalledWith(
      "cancel_external_ai_request",
      { approvalId: "approval-pending" },
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "send_external_ai_request",
      expect.anything(),
    );
  });

  it("waits for durable in-flight cancellation and surfaces ambiguity", async () => {
    let rejectSend: ((error: Error) => void) | undefined;
    mockInvoke.mockImplementation((command) => {
      if (command === "prepare_external_ai_request") {
        return Promise.resolve({ approvalId: "approval-active" });
      }
      if (command === "send_external_ai_request") {
        return new Promise((_resolve, reject) => {
          rejectSend = reject;
        });
      }
      if (command === "cancel_external_ai_request") {
        return Promise.resolve({ outcome: "ambiguous" });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const gateway = createBackendExternalAiGateway(enabledConfig());
    const send = gateway.send(publicJobRequest);
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "send_external_ai_request",
        expect.anything(),
      );
    });

    const cancellation = gateway.cancelActive();
    const sendResult = expect(send).rejects.toThrow(
      "JobSentinel ran into a problem.",
    );
    const cancellationResult = expect(cancellation).rejects.toThrow(
      "Do not retry",
    );
    rejectSend?.(new Error("provider outcome unknown"));

    await sendResult;
    await cancellationResult;
  });

  it("supports older config with a single selected provider", () => {
    const config = normalizeExternalAiRuntimeConfig({
      enabled: true,
      provider: "open_ai",
      enabled_providers: [],
    });

    expect(config).toMatchObject({
      enabled: true,
      provider: "open_ai",
      require_payload_preview: true,
      redaction: { enabled: true },
    });
    expect(toGatewaySettings(config)).toMatchObject({
      enabled: true,
      provider: "open_ai",
      requirePayloadPreview: true,
      redaction: { enabled: true },
    });
  });

  it("uses the first enabled provider as the gateway provider", () => {
    const config = normalizeExternalAiRuntimeConfig({
      enabled: true,
      provider: "open_ai",
      enabled_providers: ["anthropic", "open_ai", "anthropic"],
      require_payload_preview: true,
      allow_sensitive_payloads: false,
      redaction: { enabled: true },
      log_requests_locally: false,
    });

    expect(config.provider).toBe("anthropic");
    expect(providerLabel(config.provider)).toBe("Anthropic");
    expect(toGatewaySettings(config)).toMatchObject({
      enabled: true,
      provider: "anthropic",
      requirePayloadPreview: true,
      allowSensitivePayloads: false,
      logRequestsLocally: false,
    });
  });

  it("treats unsafe review settings as disabled for sending", () => {
    const config = normalizeExternalAiRuntimeConfig({
      enabled: true,
      provider: "custom",
      enabled_providers: ["custom"],
      require_payload_preview: false,
      redaction: { enabled: false },
    });

    expect(toGatewaySettings(config)).toMatchObject({
      enabled: true,
      provider: "custom",
      requirePayloadPreview: false,
      redaction: { enabled: false },
    });
  });
});
