/** Verifies pack disable, confirmed removal, cleanup retry, and safe failures. */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "../../../platform/tauri";
import { PackManagementModal } from "./PackManagementModal";
import { pack, release } from "./packManagementTestData";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

describe("PackLifecycleControls", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("disables a ready pack using its current generation and reloads truth", async () => {
    const user = userEvent.setup();
    const disabledRelease = release({ state: "ready" });
    mockInvoke
      .mockResolvedValueOnce([pack()])
      .mockResolvedValueOnce({ state: "disabled", generation: 5 })
      .mockResolvedValueOnce([
        pack({
          state: "disabled",
          generation: 5,
          currentRelease: disabledRelease,
          releases: [disabledRelease],
        }),
      ]);

    render(<PackManagementModal onClose={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Disable pack" }));

    expect(mockInvoke).toHaveBeenCalledWith("disable_pack", {
      publisherKeyId: "jobsentinel-release-v1",
      packId: "jobsentinel.sources.us",
      expectedGeneration: 4,
    });
    expect(await screen.findByLabelText("Pack status: Disabled")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Disable pack" })).not.toBeInTheDocument();
  });

  it("requires confirmation before removing local pack files", async () => {
    const user = userEvent.setup();
    const removedRelease = release({ isActive: false, state: "removed" });
    mockInvoke
      .mockResolvedValueOnce([pack()])
      .mockResolvedValueOnce({ generation: 5, cleanupPending: false })
      .mockResolvedValueOnce([
        pack({
          state: "removed",
          generation: 5,
          currentRelease: removedRelease,
          releases: [removedRelease],
        }),
      ]);

    render(<PackManagementModal onClose={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Remove pack" }));
    expect(screen.getByText(/removes its local files/i)).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Remove pack files" }));
    expect(mockInvoke).toHaveBeenCalledWith("uninstall_pack", {
      publisherKeyId: "jobsentinel-release-v1",
      packId: "jobsentinel.sources.us",
      expectedGeneration: 4,
    });
    expect(await screen.findByLabelText("Pack status: Removed")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Remove pack" })).not.toBeInTheDocument();
  });

  it("retries generation-bound cleanup and reloads the durable result", async () => {
    const user = userEvent.setup();
    const pendingRelease = release({
      artifactCleanupPending: true,
      isActive: false,
      state: "removed",
    });
    const cleanedRelease = release({ isActive: false, state: "removed" });
    mockInvoke
      .mockResolvedValueOnce([
        pack({
          state: "removed",
          cleanupPending: true,
          currentRelease: pendingRelease,
          releases: [pendingRelease],
        }),
      ])
      .mockResolvedValueOnce({ generation: 4, cleanupPending: false })
      .mockResolvedValueOnce([
        pack({
          state: "removed",
          cleanupPending: false,
          currentRelease: cleanedRelease,
          releases: [cleanedRelease],
        }),
      ]);

    render(<PackManagementModal onClose={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Retry cleanup" }));

    expect(mockInvoke).toHaveBeenCalledWith("retry_pack_cleanup", {
      publisherKeyId: "jobsentinel-release-v1",
      packId: "jobsentinel.sources.us",
      expectedGeneration: 4,
    });
    await waitFor(() => {
      expect(screen.queryByText("Artifact cleanup needs another attempt"))
        .not.toBeInTheDocument();
    });
  });

  it("keeps lifecycle failures local and leaves current truth actionable", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce([pack()])
      .mockRejectedValueOnce(new Error("raw database detail"))
      .mockResolvedValueOnce([
        pack({ state: "disabled", generation: 5 }),
      ]);

    render(<PackManagementModal onClose={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Disable pack" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Pack could not be disabled. Refresh and try again.",
    );
    expect(screen.queryByText("raw database detail")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Pack status: Ready")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Disable pack" })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "Refresh" }));
    expect(
      await screen.findByLabelText("Pack status: Disabled"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
