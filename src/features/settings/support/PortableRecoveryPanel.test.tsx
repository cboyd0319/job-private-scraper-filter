import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PortableRecoveryPanel } from "./PortableRecoveryPanel";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("PortableRecoveryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("none");
      }
      return Promise.resolve({
        outcome: "succeeded",
        connectivity_required: false,
      });
    });
  });

  it("requires a matching 16-character passphrase before creating a local backup", async () => {
    const user = userEvent.setup();
    render(<PortableRecoveryPanel />);

    expect(
      screen.getByText("Local only. No internet required."),
    ).toBeVisible();
    expect(
      screen.getByText("Credential and secret export remains unavailable."),
    ).toBeVisible();

    const passphrase = screen.getByLabelText("Backup passphrase");
    const confirmation = screen.getByLabelText("Confirm backup passphrase");
    const create = screen.getByRole("button", {
      name: "Create encrypted backup",
    });

    expect(passphrase).toHaveAttribute("type", "password");
    expect(passphrase).toHaveAttribute("autocomplete", "new-password");
    expect(confirmation).toHaveAttribute("type", "password");
    expect(confirmation).toHaveAttribute("autocomplete", "new-password");
    expect(create).toBeDisabled();

    await user.type(passphrase, "sixteen-letters!!");
    await user.type(confirmation, "does-not-match!!");
    expect(create).toBeDisabled();

    await user.clear(confirmation);
    await user.type(confirmation, "sixteen-letters!!");
    expect(create).toBeEnabled();

    await user.click(create);

    expect(mockInvoke).toHaveBeenCalledWith("create_portable_backup", {
      passphrase: "sixteen-letters!!",
    });
    expect(passphrase).toHaveValue("");
    expect(confirmation).toHaveValue("");
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent("Encrypted backup saved locally.");
  });

  it("loads staged restore state independently and lets the user cancel before restart", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("ready");
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

    expect(
      await screen.findByText(
        "A restore is staged. You can cancel it until you restart JobSentinel.",
      ),
    ).toBeVisible();
    expect(
      screen.getByText(
        /On restart, JobSentinel verifies the backup before promotion and keeps a private recovery copy/i,
      ),
    ).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "Cancel staged restore" }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("cancel_staged_restore");
    expect(
      screen.queryByRole("button", { name: "Cancel staged restore" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent("Staged restore cancelled locally.");
  });

  it("stages a selected encrypted backup without applying it during the session", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("none");
      }
      if (command === "stage_portable_restore") {
        return Promise.resolve({
          outcome: "staged",
          connectivity_required: false,
          restart_required: true,
        });
      }
      return Promise.reject(new Error("unexpected command"));
    });
    const user = userEvent.setup();

    render(<PortableRecoveryPanel />);

    const passphrase = screen.getByLabelText("Restore passphrase");
    expect(passphrase).toHaveAttribute("type", "password");
    expect(passphrase).toHaveAttribute("autocomplete", "off");

    await user.type(passphrase, "restore-passphrase");
    await user.click(
      screen.getByRole("button", { name: "Choose and stage backup" }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("stage_portable_restore", {
      passphrase: "restore-passphrase",
    });
    expect(screen.queryByLabelText("Restore passphrase")).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("restore-passphrase");
    expect(
      screen.getByText(
        "A restore is staged. You can cancel it until you restart JobSentinel.",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent(
      "Restore staged locally. Restart JobSentinel to verify and apply it.",
    );
  });

  it("defaults reviewed plaintext export to excluding protected records", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("none");
      }
      if (command === "create_reviewed_export") {
        return Promise.resolve({
          outcome: "succeeded",
          connectivity_required: false,
        });
      }
      return Promise.reject(new Error("unexpected command"));
    });
    const user = userEvent.setup();

    render(<PortableRecoveryPanel />);

    expect(
      screen.getByText(
        /Plaintext can expose private details in user-authored text/i,
      ),
    ).toBeVisible();
    expect(
      screen.getByText(
        /Managed credentials and private application paths are always excluded/i,
      ),
    ).toBeVisible();
    expect(
      screen.getByText(/Existing files are never overwritten/i),
    ).toBeVisible();

    const includeProtected = screen.getByRole("checkbox", {
      name: /Include protected application answers, veteran, disability, clearance, and military fields, and structured drafts/i,
    });
    expect(includeProtected).not.toBeChecked();

    await user.click(
      screen.getByRole("button", {
        name: "Review and create plaintext export",
      }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("create_reviewed_export", {
      includeProtectedRecords: false,
    });
    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent("Reviewed plaintext export saved locally.");

    await user.click(includeProtected);
    await user.click(
      screen.getByRole("button", {
        name: "Review and create plaintext export",
      }),
    );
    expect(mockInvoke).toHaveBeenLastCalledWith("create_reviewed_export", {
      includeProtectedRecords: true,
    });
    expect(includeProtected).not.toBeChecked();
  });

  it("disables other recovery actions without mislabeling them while one runs", async () => {
    let finishBackup:
      | ((value: {
          outcome: "succeeded";
          connectivity_required: false;
        }) => void)
      | undefined;
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("none");
      }
      if (command === "create_portable_backup") {
        return new Promise((resolve) => {
          finishBackup = resolve;
        });
      }
      return Promise.reject(new Error("unexpected command"));
    });
    const user = userEvent.setup();

    render(<PortableRecoveryPanel />);
    await user.type(
      screen.getByLabelText("Backup passphrase"),
      "1234567890123456",
    );
    await user.type(
      screen.getByLabelText("Confirm backup passphrase"),
      "1234567890123456",
    );
    await user.click(
      screen.getByRole("button", { name: "Create encrypted backup" }),
    );

    expect(screen.getByLabelText("Backup passphrase")).toHaveValue("");
    expect(screen.getByLabelText("Confirm backup passphrase")).toHaveValue("");
    expect(screen.getByLabelText("Restore passphrase")).toBeDisabled();
    expect(
      screen.getByRole("button", {
        name: "Review and create plaintext export",
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole("checkbox", {
        name: /Include protected application answers/i,
      }),
    ).toBeDisabled();

    finishBackup?.({
      outcome: "succeeded",
      connectivity_required: false,
    });
    await waitFor(() => {
      expect(screen.getByLabelText("Restore passphrase")).toBeEnabled();
    });
  });

  it("shows fixed failure text without raw paths or secrets", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_staged_restore_status") {
        return Promise.resolve("none");
      }
      if (command === "create_reviewed_export") {
        return Promise.reject(
          new Error("/Users/private/jobs.db secret-token"),
        );
      }
      return Promise.reject(new Error("unexpected command"));
    });
    const user = userEvent.setup();

    render(<PortableRecoveryPanel />);
    await user.click(
      screen.getByRole("button", {
        name: "Review and create plaintext export",
      }),
    );

    expect(
      screen.getByRole("status", { name: "Portable recovery status" }),
    ).toHaveTextContent("Reviewed plaintext export could not be created.");
    expect(document.body).not.toHaveTextContent("/Users/private/jobs.db");
    expect(document.body).not.toHaveTextContent("secret-token");
  });
});
