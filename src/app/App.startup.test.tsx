import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App, { StartupRecovery } from "./App";
import * as supportReport from "../shared/errorReporting/supportReport";
import { ToastProvider } from "./providers/ToastProvider";
import { UndoProvider } from "./providers/UndoProvider";

const mockInvoke = vi.mocked(invoke);

vi.mock("../shared/errorReporting/logger", () => ({
  logError: vi.fn(),
}));

describe("App startup recovery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows recovery instead of first-run setup when startup status fails", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("permission denied"));

    render(<App />);

    await waitFor(() => {
      expect(
        screen.getByRole("heading", {
          name: /could not open saved setup/i,
        }),
      ).toBeInTheDocument();
    });

    expect(screen.getByRole("button", { name: "Try Again" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Copy Safe Support Report" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Save Safe Support Report" }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/pick a career path/i)).not.toBeInTheDocument();
  });

  it("enters the local app for this session when first-run setup is skipped", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(true);

    render(
      <ToastProvider>
        <UndoProvider>
          <App />
        </UndoProvider>
      </ToastProvider>,
    );

    await user.click(await screen.findByRole("button", { name: "Skip for now" }));

    expect(await screen.findByRole("navigation", { name: "Main navigation" })).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("is_first_run");
    expect(mockInvoke.mock.calls.some(([command]) => command === "complete_setup")).toBe(false);
  });

  it("lets users copy a safe support report from startup recovery", async () => {
    const user = userEvent.setup();
    vi.spyOn(supportReport, "copySanitizedDebugReport").mockResolvedValueOnce({
      content: "safe report",
      copied: true,
      errorCount: 0,
    });

    render(<StartupRecovery onRetry={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Copy Safe Support Report" }));

    expect(supportReport.copySanitizedDebugReport).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Safe support report copied")).toBeInTheDocument();
  });

  it("preserves invalid saved settings before resetting startup", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        required: true,
        platform: false,
        configuration: true,
        database: false,
        connectivity_required: false,
      })
      .mockResolvedValueOnce({
        outcome: "preserved_and_reset",
        connectivity_required: false,
      });

    render(<StartupRecovery onRetry={vi.fn()} />);

    await user.click(
      await screen.findByRole("button", {
        name: "Preserve and reset saved settings",
      }),
    );

    expect(mockInvoke).toHaveBeenLastCalledWith(
      "repair_invalid_startup_config",
    );
    expect(
      screen.getByRole("status", { name: "Startup recovery status" }),
    ).toHaveTextContent(
      "Saved settings were preserved locally. Restart JobSentinel to continue.",
    );
    expect(
      screen.queryByRole("button", { name: "Try Again" }),
    ).not.toBeInTheDocument();
  });

  it("offers an offline permission repair when platform setup failed", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        required: true,
        platform: true,
        configuration: false,
        database: true,
        connectivity_required: false,
      });
    for (let index = 0; index < 3; index += 1) {
      mockInvoke.mockResolvedValueOnce({
        outcome: "repaired",
        connectivity_required: false,
      });
    }

    render(<StartupRecovery onRetry={vi.fn()} />);

    await user.click(
      await screen.findByRole("button", {
        name: "Repair local permissions",
      }),
    );

    for (const area of ["application_data", "configuration", "cache"]) {
      expect(mockInvoke).toHaveBeenCalledWith("repair_local_permissions", {
        area,
      });
    }
    expect(
      screen.getByRole("status", { name: "Startup recovery status" }),
    ).toHaveTextContent(
      "Local permissions were repaired. Restart JobSentinel to continue.",
    );
    expect(screen.queryByLabelText("Backup passphrase")).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Repair local permissions and restart before restoring a backup.",
      ),
    ).toBeVisible();
  });

  it("stages an encrypted backup when the local database cannot open", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        required: true,
        platform: false,
        configuration: false,
        database: true,
        connectivity_required: false,
      })
      .mockResolvedValueOnce("none")
      .mockResolvedValueOnce({
        outcome: "staged",
        connectivity_required: false,
        restart_required: true,
      });

    render(<StartupRecovery onRetry={vi.fn()} />);

    const passphrase = await screen.findByLabelText("Backup passphrase");
    expect(
      screen.getByText(/keeps a private recovery copy/i),
    ).toBeVisible();
    await user.type(passphrase, "sixteen-letters!!");
    await user.click(
      screen.getByRole("button", { name: "Choose backup and stage restore" }),
    );

    expect(mockInvoke).toHaveBeenLastCalledWith("stage_portable_restore", {
      passphrase: "sixteen-letters!!",
    });
    expect(passphrase).toHaveValue("");
    expect(
      screen.getByRole("status", { name: "Startup recovery status" }),
    ).toHaveTextContent(
      "Encrypted restore is staged. Close and reopen JobSentinel to finish.",
    );
  });

  it("lets users cancel a staged database restore before restart", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        required: true,
        platform: false,
        configuration: false,
        database: true,
        connectivity_required: false,
      })
      .mockResolvedValueOnce("ready")
      .mockResolvedValueOnce({
        outcome: "cancelled",
        connectivity_required: false,
        restart_required: false,
      });

    render(<StartupRecovery onRetry={vi.fn()} />);

    await user.click(
      await screen.findByRole("button", { name: "Cancel staged restore" }),
    );

    expect(mockInvoke).toHaveBeenLastCalledWith("cancel_staged_restore");
    expect(await screen.findByLabelText("Backup passphrase")).toBeVisible();
    expect(
      screen.getByRole("status", { name: "Startup recovery status" }),
    ).toHaveTextContent("Staged restore was cancelled. Saved data was not changed.");
  });

  it("lets users preserve and clear an invalid restore marker", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        required: true,
        platform: false,
        configuration: false,
        database: true,
        connectivity_required: false,
      })
      .mockResolvedValueOnce("invalid")
      .mockResolvedValueOnce({
        outcome: "cancelled",
        connectivity_required: false,
        restart_required: false,
      });

    render(<StartupRecovery onRetry={vi.fn()} />);

    await user.click(
      await screen.findByRole("button", {
        name: "Preserve and remove invalid restore files",
      }),
    );

    expect(mockInvoke).toHaveBeenLastCalledWith("cancel_staged_restore");
    expect(await screen.findByLabelText("Backup passphrase")).toBeVisible();
  });

  it("opens the documented Search Links page from navigation", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(false);

    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: "Search Links (Cmd/Ctrl+9)" }),
    );

    expect(
      await screen.findByRole("heading", { name: "Job Site Search Links" }),
    ).toBeInTheDocument();
  });
});
