import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PortableRecoveryPanel } from "./PortableRecoveryPanel";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("PortableRecoveryPanel interrupted staging", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("incomplete");
      }
      if (command === "cancel_staged_restore") {
        return Promise.resolve({
          outcome: "cancelled",
          connectivity_required: false,
          restart_required: false,
        });
      }
      return Promise.reject(new Error("unexpected command"));
    });
  });

  it("keeps an unauthorized incomplete stage ignored until local cleanup", async () => {
    const user = userEvent.setup();
    render(<PortableRecoveryPanel />);

    expect(
      await screen.findByText(
        /An interrupted restore staging attempt was found.*Startup will ignore it because it has no authorization marker/i,
      ),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Choose and stage backup" }),
    ).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", {
        name: "Remove incomplete restore files",
      }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("cancel_staged_restore");
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent("Incomplete restore files removed locally.");
    expect(
      screen.getByRole("button", { name: "Choose and stage backup" }),
    ).toBeDisabled();
  });

  it("preserves a malformed regular restore marker before clearing it", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("invalid");
      }
      if (command === "cancel_staged_restore") {
        return Promise.resolve({
          outcome: "cancelled",
          connectivity_required: false,
          restart_required: false,
        });
      }
      return Promise.reject(new Error("unexpected command"));
    });
    const user = userEvent.setup();
    render(<PortableRecoveryPanel />);

    await user.click(
      await screen.findByRole("button", {
        name: "Preserve and remove invalid restore files",
      }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("cancel_staged_restore");
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent(
      "Invalid restore files preserved privately and cleared locally.",
    );
  });
});
