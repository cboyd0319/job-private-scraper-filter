/** Verifies Settings save flows, source boundaries, and user-visible state. */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  makeConfig,
  makeGhostConfig,
  mockInvoke,
  mockToast,
  setupHappyPath,
} from "./SettingsPage.testSupport";
import Settings from "./SettingsPage";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe("Settings — handleSave flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("moves settings tabs with arrow keys and keeps one tab stop", async () => {
    setupHappyPath();
    render(<Settings onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    const searchTab = screen.getByRole("tab", { name: "Search Preferences" });
    const sourcesTab = screen.getByRole("tab", { name: "Sources & Alerts" });

    expect(searchTab).toHaveAttribute("tabIndex", "0");
    expect(sourcesTab).toHaveAttribute("tabIndex", "-1");

    searchTab.focus();
    fireEvent.keyDown(searchTab, { key: "ArrowRight" });

    await waitFor(() => {
      expect(sourcesTab).toHaveAttribute("aria-selected", "true");
      expect(sourcesTab).toHaveFocus();
    });
    expect(searchTab).toHaveAttribute("tabIndex", "-1");
    expect(sourcesTab).toHaveAttribute("tabIndex", "0");

    fireEvent.keyDown(sourcesTab, { key: "Home" });

    await waitFor(() => {
      expect(searchTab).toHaveAttribute("aria-selected", "true");
      expect(searchTab).toHaveFocus();
    });
  });

  it("opens Sources and Alerts when requested by another workflow", async () => {
    setupHappyPath();
    render(<Settings initialTab="advanced" onClose={vi.fn()} />);

    expect(
      await screen.findByRole("tab", { name: "Sources & Alerts" }),
    ).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("tab", { name: "Search Preferences" }),
    ).toHaveAttribute("aria-selected", "false");
  });

  it("enables the saved-details passphrase lock from Settings", async () => {
    const user = userEvent.setup();
    let unlockStatus = { mode: "system", configured: false, unlocked: true };

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return makeConfig();
      if (cmd === "get_credential_status") return [];
      if (cmd === "get_credential_unlock_status") return unlockStatus;
      if (cmd === "enable_credential_passphrase") {
        unlockStatus = { mode: "passphrase", configured: true, unlocked: true };
        return null;
      }
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      return null;
    });

    render(<Settings onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    expect(await screen.findByText("Saved Details Lock")).toBeInTheDocument();
    expect(screen.getByText("System lock is active")).toBeInTheDocument();

    await user.type(
      screen.getByLabelText("New passphrase"),
      "correct battery staple",
    );
    await user.type(
      screen.getByLabelText("Confirm passphrase"),
      "correct battery staple",
    );
    await user.click(
      screen.getByRole("button", { name: "Use Passphrase Lock" }),
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("enable_credential_passphrase", {
        passphrase: "correct battery staple",
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith(
      "Passphrase lock enabled",
      "Saved details will need this passphrase after app start.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "has_credential",
      expect.anything(),
    );
  });

  it("unlocks saved details without checking credential existence", async () => {
    const user = userEvent.setup();
    let unlockStatus = {
      mode: "passphrase",
      configured: true,
      unlocked: false,
    };

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return makeConfig();
      if (cmd === "get_credential_status") return [];
      if (cmd === "get_credential_unlock_status") return unlockStatus;
      if (cmd === "unlock_credential_vault") {
        unlockStatus = { mode: "passphrase", configured: true, unlocked: true };
        return null;
      }
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      return null;
    });

    render(<Settings onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    expect(
      await screen.findByText("Passphrase lock is on"),
    ).toBeInTheDocument();

    await user.type(
      screen.getByLabelText("Current passphrase"),
      "correct battery staple",
    );
    await user.click(
      screen.getByRole("button", { name: "Unlock Saved Details" }),
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("unlock_credential_vault", {
        passphrase: "correct battery staple",
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith(
      "Saved details unlocked",
      "Saved details can be used during this app session.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "has_credential",
      expect.anything(),
    );
  });

  it("shows success toast and closes on successful save", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();

    setupHappyPath();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return makeConfig();
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      if (cmd === "save_config") return null;
      return null;
    });

    render(<Settings onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    // Find and click save button
    const saveButton = screen.getByRole("button", { name: /save changes/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockToast.success).toHaveBeenCalledWith(
        "Settings saved",
        "Your job-search preferences were saved.",
      );
    });

    expect(onClose).toHaveBeenCalled();
  });

  it("keeps the connected source disabled while provider review is pending", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const loadedConfig = {
      ...makeConfig(),
      title_allowlist: ["Case Manager"],
    };
    let savedConfig: ReturnType<typeof makeConfig> | null = null;

    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === "get_config") return loadedConfig;
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      if (cmd === "save_config") {
        savedConfig = (args as { config: ReturnType<typeof makeConfig> })
          .config;
        return null;
      }
      return null;
    });

    render(<Settings onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.click(screen.getByText("More Job Boards"));
    await user.type(
      screen.getByLabelText("Optional job-source link"),
      "https://api.jobswithgpt.example/mcp",
    );

    expect(
      screen.getByPlaceholderText(
        "Leave blank unless you intentionally use an outside job feed",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Optional source address"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Approve these exact details before JobSentinel contacts this source",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("api.jobswithgpt.example")).toBeInTheDocument();
    expect(
      screen.queryByText("https://api.jobswithgpt.example/mcp"),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Show full link" }));

    expect(
      screen.getByText("https://api.jobswithgpt.example/mcp"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Hide full link" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Case Manager")).toBeInTheDocument();
    const approvalSummary = screen
      .getByText(
        "Approve these exact details before JobSentinel contacts this source",
      )
      .closest("div");
    expect(approvalSummary).not.toBeNull();
    expect(within(approvalSummary!).getByText("Location")).toBeInTheDocument();
    expect(within(approvalSummary!).getByText("Not sent")).toBeInTheDocument();
    expect(
      within(approvalSummary!).getByText("Remote-only filter"),
    ).toBeInTheDocument();
    expect(within(approvalSummary!).getByText("Yes")).toBeInTheDocument();
    expect(
      within(approvalSummary!).queryByText(
        "Uses your saved work-location choices",
      ),
    ).not.toBeInTheDocument();

    expect(
      screen.getByText(
        "Scheduled contact is disabled while JobSentinel verifies the provider endpoint and usage policy.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Provider review pending" }),
    ).toBeDisabled();
    expect(
      screen.queryByRole("button", { name: "Approve these exact details" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /save changes/i }));
    await waitFor(() => {
      expect(mockToast.success).toHaveBeenCalledWith(
        "Settings saved",
        "Your job-search preferences were saved.",
      );
    });

    expect(savedConfig?.jobswithgpt_endpoint).toBe(
      "https://api.jobswithgpt.example/mcp",
    );
    expect(savedConfig?.jobswithgpt_approval.enabled).toBe(false);
    expect(savedConfig?.jobswithgpt_approval.payload).toBeNull();
  });

  it("shows connected source contact history as minimized metadata", async () => {
    const user = userEvent.setup();
    const approvedPayload = {
      endpoint: "https://api.jobswithgpt.example/mcp",
      titles: ["Case Manager"],
      location: null,
      remote_only: true,
      limit: 100,
    };
    const loadedConfig = {
      ...makeConfig(),
      title_allowlist: ["Case Manager"],
      jobswithgpt_endpoint: approvedPayload.endpoint,
      jobswithgpt_approval: {
        enabled: true,
        payload: approvedPayload,
        approved_at: "2026-06-01T12:00:00Z",
      },
    };

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return loadedConfig;
      if (cmd === "get_latest_source_request") {
        return {
          id: 42,
          source: "jobswithgpt",
          sentAt: "2026-06-01T12:30:00Z",
          endpointHost: "api.jobswithgpt.example",
          titleCount: 1,
          hasLocation: false,
          remoteOnly: true,
          resultLimit: 100,
          outcome: "started",
        };
      }
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      return null;
    });

    render(<Settings onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.click(screen.getByText("More Job Boards"));

    const contactSummary = screen
      .getByText(/Last contacted or attempted:/i)
      .closest("div");
    expect(contactSummary).not.toBeNull();
    expect(
      within(contactSummary!).getByText("Website contacted"),
    ).toBeInTheDocument();
    expect(
      within(contactSummary!).queryByText("Source host"),
    ).not.toBeInTheDocument();
    expect(
      within(contactSummary!).getByText("api.jobswithgpt.example"),
    ).toBeInTheDocument();
    expect(
      within(contactSummary!).getByText(
        "Outcome unknown; request may not have been sent.",
      ),
    ).toBeInTheDocument();
    expect(
      within(contactSummary!).queryByText("Started"),
    ).not.toBeInTheDocument();
    expect(
      within(contactSummary!).getByText("Remote-only filter"),
    ).toBeInTheDocument();
    expect(within(contactSummary!).getByText("Yes")).toBeInTheDocument();
    expect(within(contactSummary!).getByText("No")).toBeInTheDocument();
    expect(
      within(contactSummary!).getByText("Data not sent"),
    ).toBeInTheDocument();
    expect(
      within(contactSummary!).getByText(
        "Resume text, salary floor, private notes, application history, full source link",
      ),
    ).toBeInTheDocument();
  });

});
