import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsExternalAiRequestHistory } from "./SettingsExternalAiRequestHistory";

const mockInvoke = vi.mocked(invoke);

describe("SettingsExternalAiRequestHistory", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("shows durable ambiguous activity without backend review secrets", async () => {
    mockInvoke.mockResolvedValue([
      {
        providerId: "open_ai",
        destination: "https://provider.example/outside-ai",
        status: "ambiguous",
        createdAt: "2026-07-19T01:00:00Z",
        completedAt: "2026-07-19T01:00:01Z",
        approvalId: "outside-ai-approval:secret",
        requestSha256: "secret-digest",
        rawError: "private error",
      },
    ]);

    render(<SettingsExternalAiRequestHistory />);

    expect(
      await screen.findByText("Outcome unknown. Do not retry."),
    ).toBeInTheDocument();
    expect(screen.getByText("open_ai")).toBeInTheDocument();
    expect(
      screen.getByText("https://provider.example/outside-ai"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/outside-ai-approval/)).not.toBeInTheDocument();
    expect(screen.queryByText(/secret-digest/)).not.toBeInTheDocument();
    expect(screen.queryByText(/private error/)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /clear history/i }),
    ).not.toBeInTheDocument();
  });
});
