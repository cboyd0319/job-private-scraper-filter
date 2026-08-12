import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LocalRecoveryPanel } from "./LocalRecoveryPanel";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const readyStorage = {
  state: "ready" as const,
  reclaimable_bytes: 4096,
  wal_bytes: 1024,
  incremental_vacuum_supported: true,
  cleanup_available: true,
  connectivity_required: false as const,
};

function makeReport() {
  return {
    schema_version: 2,
    connectivity_required: false as const,
    queued_local_work: {
      pending_url_imports: 2,
      capacity: 20,
      available_offline: true,
      connectivity_required: false as const,
    },
    storage: readyStorage,
    privacy_doctor: {
      schema_version: 1,
      overall: "needs_attention" as const,
      checks: [
        {
          id: "outside_ai",
          state: "needs_attention",
          message: "Outside AI needs your review.",
          action: "review_external_ai",
          connectivity_required: false as const,
        },
        {
          id: "telemetry",
          state: "looks_good",
          message: "Telemetry is off.",
          action: null,
          connectivity_required: false as const,
        },
      ],
      connectivity_required: false as const,
    },
    platform_health: {
      schema_version: 1,
      permissions: [
        {
          area: "application_data" as const,
          state: "private" as const,
          action: null,
          connectivity_required: false as const,
        },
        {
          area: "configuration" as const,
          state: "needs_repair" as const,
          action: "repair_locally" as const,
          connectivity_required: false as const,
        },
        {
          area: "cache" as const,
          state: "manual_review" as const,
          action: "follow_manual_guidance" as const,
          connectivity_required: false as const,
        },
      ],
      package_repair: {
        mode: "guidance_only" as const,
        actions: [
          {
            action: "use_downloaded_verified_installer" as const,
            connectivity_required: false as const,
          },
          {
            action: "obtain_verified_installer" as const,
            connectivity_required: true as const,
          },
        ],
      },
    },
  };
}

describe("LocalRecoveryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows local status before actions and renders bounded recovery guidance", async () => {
    mockInvoke.mockResolvedValueOnce(makeReport());

    render(<LocalRecoveryPanel />);

    expect(screen.getByText("Local only. No internet required.")).toBeVisible();
    expect(
      screen.getByRole("status", { name: "Local recovery status" }),
    ).toHaveTextContent("Checking local recovery");

    expect(
      await screen.findByRole("heading", { name: "Local storage" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Local storage is ready.")).toBeInTheDocument();
    expect(
      screen.getByText(/preserves every saved record/i),
    ).toBeInTheDocument();
    expect(screen.getByText("Outside AI needs your review.")).toBeInTheDocument();
    expect(
      screen.getByText("Review Outside AI settings before sending."),
    ).toBeInTheDocument();
    expect(screen.getByText("Telemetry is off.")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Queued local work" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/2 local URL imports are ready in this app session/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/available offline and expire locally/i),
    ).toBeInTheDocument();

    expect(
      screen.getByRole("button", {
        name: "Repair configuration permissions",
      }),
    ).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "Repair cache permissions" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        /Automatic repair is not available for cache\. Use your system's file-permission settings or help/i,
      ),
    ).toBeInTheDocument();

    expect(
      screen.getByText(
        /Use an already-downloaded, verified JobSentinel installer.*works offline/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Obtain a verified JobSentinel installer.*internet required/i,
      ),
    ).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("get_local_recovery_report");
  });

  it("runs ready storage cleanup locally and reports its bounded effect", async () => {
    const cleanedStorage = {
      ...readyStorage,
      reclaimable_bytes: 0,
      wal_bytes: 0,
    };
    mockInvoke
      .mockResolvedValueOnce(makeReport())
      .mockResolvedValueOnce(cleanedStorage);
    const user = userEvent.setup();

    render(<LocalRecoveryPanel />);

    await user.click(
      await screen.findByRole("button", { name: "Clean up local storage" }),
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("run_local_storage_cleanup");
    });
    expect(
      screen.getByRole("status", { name: "Local recovery status" }),
    ).toHaveTextContent(
      "Cleanup finished locally. Saved records were preserved; only already-free space was reclaimed.",
    );
    expect(
      screen.getByText("No already-free storage is ready to reclaim."),
    ).toBeInTheDocument();
  });

  it("disables cleanup for damaged storage and points to encrypted recovery", async () => {
    mockInvoke.mockResolvedValueOnce({
      ...makeReport(),
      storage: {
        ...readyStorage,
        state: "restore_from_backup_required",
        cleanup_available: false,
      },
    });

    render(<LocalRecoveryPanel />);

    expect(
      await screen.findByText("Local storage needs recovery."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/restore from an encrypted backup before cleanup/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Clean up local storage" }),
    ).toBeDisabled();
  });

  it("repairs only locally repairable permissions and refreshes the report", async () => {
    const refreshedReport = makeReport();
    refreshedReport.platform_health.permissions[1] = {
      area: "configuration",
      state: "private",
      action: null,
      connectivity_required: false,
    };
    mockInvoke
      .mockResolvedValueOnce(makeReport())
      .mockResolvedValueOnce({
        schema_version: 1,
        area: "configuration",
        outcome: "repaired",
        connectivity_required: false,
      })
      .mockResolvedValueOnce(refreshedReport);
    const user = userEvent.setup();

    render(<LocalRecoveryPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Repair configuration permissions",
      }),
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("repair_local_permissions", {
        area: "configuration",
      });
      expect(
        mockInvoke.mock.calls.filter(
          ([command]) => command === "get_local_recovery_report",
        ),
      ).toHaveLength(2);
    });
    expect(
      screen.getByRole("status", { name: "Local recovery status" }),
    ).toHaveTextContent("Configuration permissions were repaired locally.");
    expect(
      screen.queryByRole("button", {
        name: "Repair configuration permissions",
      }),
    ).not.toBeInTheDocument();
  });

  it("keeps a report failure inside the panel without exposing raw details", async () => {
    mockInvoke.mockRejectedValueOnce(
      new Error("/Users/private/jobs.db secret-token"),
    );

    render(<LocalRecoveryPanel />);

    expect(
      await screen.findByText(
        "Local recovery status is unavailable. Other settings are still available.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Retry local recovery check" }),
    ).toBeEnabled();
    expect(document.body).not.toHaveTextContent("/Users/private/jobs.db");
    expect(document.body).not.toHaveTextContent("secret-token");
  });
});
