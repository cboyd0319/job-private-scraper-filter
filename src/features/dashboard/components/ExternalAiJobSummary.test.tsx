import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../../app/providers/ToastProvider";
import { ExternalAiJobSummary } from "./ExternalAiJobSummary";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import("@tauri-apps/api/core");
const mockInvoke = vi.mocked(invoke);

const job = {
  id: 42,
  hash: "job-hash",
  title: "Customer Support Lead",
  company: "CareBridge Services",
  location: "Chicago, IL",
  url: "https://example.com/job/42",
  source: "greenhouse",
  created_at: "2026-06-18T12:00:00.000Z",
  description: "Guide care teams and improve customer support workflows.",
  salary_min: 55_000,
  salary_max: 72_000,
  remote: false,
  score: 0.92,
  notes: "Private local note",
};

function renderWithToast(ui: React.ReactElement) {
  return render(<ToastProvider>{ui}</ToastProvider>);
}

function enabledExternalAiConfig() {
  return {
    enabled: true,
    provider: "open_ai",
    enabled_providers: ["open_ai"],
    require_payload_preview: true,
    allow_sensitive_payloads: false,
    redaction: { enabled: true },
    log_requests_locally: false,
  };
}

describe("ExternalAiJobSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockReset();
    window.localStorage.clear();
  });

  it("guides the user to settings when outside AI is not configured", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue({ external_ai: { enabled: false } });

    renderWithToast(<ExternalAiJobSummary job={job} />);
    await user.click(
      screen.getByRole("button", {
        name: "Summarize posting with Outside AI",
      }),
    );

    expect(await screen.findByText("Outside AI is not ready")).toBeInTheDocument();
    expect(
      screen.getByText(/Turn on Outside AI in Settings/i),
    ).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("get_config", {});
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "send_external_ai_request",
      expect.anything(),
    );
  });

  it("sends only reviewed public job details through the backend command", async () => {
    const user = userEvent.setup();
    let preparedArgs: unknown;
    let sentArgs: unknown;

    mockInvoke.mockImplementation((cmd, args) => {
      if (cmd === "get_config") {
        return Promise.resolve({ external_ai: enabledExternalAiConfig() });
      }
      if (cmd === "prepare_external_ai_request") {
        preparedArgs = args;
        return Promise.resolve({ approvalId: "approval-123" });
      }
      if (cmd === "send_external_ai_request") {
        sentArgs = args;
        return Promise.resolve({
          text: "Summary\nThis is a public posting summary.",
        });
      }
      return Promise.resolve(null);
    });

    renderWithToast(<ExternalAiJobSummary job={job} />);
    await user.click(
      screen.getByRole("button", {
        name: "Summarize posting with Outside AI",
      }),
    );

    expect(
      await screen.findByRole("heading", { name: "Review Outside AI Details" }),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Send Reviewed Details" }),
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "prepare_external_ai_request",
        expect.anything(),
      );
      expect(mockInvoke).toHaveBeenCalledWith(
        "send_external_ai_request",
        expect.anything(),
      );
    });
    expect(sentArgs).toMatchObject({
      approvalId: "approval-123",
      request: {
        feature: "job-description-summary",
        sourceJobId: 42,
        provider: "open_ai",
        labels: ["External AI optional", "Public-data only"],
        dataCategories: ["job_posting", "public_metadata"],
        previewShown: true,
        userApproved: true,
        payload: {
          title: "Customer Support Lead",
          company: "CareBridge Services",
          location: "Chicago, IL",
          description: "Guide care teams and improve customer support workflows.",
        },
      },
    });
    expect((sentArgs as { request: unknown }).request).toBe(
      (preparedArgs as { request: unknown }).request,
    );
    expect(JSON.stringify(sentArgs)).not.toContain("Private local note");
    expect(JSON.stringify(sentArgs)).not.toContain("0.92");
    expect(
      await screen.findByRole("heading", { name: "Posting Summary" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/public posting summary/i)).toBeInTheDocument();
  });

  it("cancels the active backend approval while sending", async () => {
    const user = userEvent.setup();
    let rejectSend: ((error: Error) => void) | undefined;

    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "get_config") {
        return Promise.resolve({ external_ai: enabledExternalAiConfig() });
      }
      if (cmd === "prepare_external_ai_request") {
        return Promise.resolve({ approvalId: "approval-cancel" });
      }
      if (cmd === "send_external_ai_request") {
        return new Promise((_resolve, reject) => {
          rejectSend = reject;
        });
      }
      if (cmd === "cancel_external_ai_request") {
        return Promise.resolve({ outcome: "ambiguous" });
      }
      return Promise.reject(new Error(`Unexpected command: ${cmd}`));
    });

    renderWithToast(<ExternalAiJobSummary job={job} />);
    await user.click(
      screen.getByRole("button", {
        name: "Summarize posting with Outside AI",
      }),
    );
    await user.click(
      await screen.findByRole("button", { name: "Send Reviewed Details" }),
    );
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "send_external_ai_request",
        expect.anything(),
      );
    });

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "cancel_external_ai_request",
        { approvalId: "approval-cancel" },
      );
    });
    expect(
      screen.getByRole("button", {
        name: "Summarize posting with Outside AI",
      }),
    ).toBeDisabled();
    rejectSend?.(new Error("Outside AI request cancelled."));
    expect(
      await screen.findByText(/Settings > Outside AI.*durable activity/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Summarize posting with Outside AI",
      }),
    ).toBeDisabled();
  });
});
