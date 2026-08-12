import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BrowserImportSection } from "./BrowserImportSection";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("BrowserImportSection applied logging", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it("copies a separate one-use applied draft button", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_bookmarklet_config") {
        return Promise.resolve({ port: 4321, enabled: true });
      }
      if (command === "get_pending_bookmarklet_imports") {
        return Promise.resolve([]);
      }
      if (command === "copy_applied_bookmarklet_code") {
        return Promise.resolve(true);
      }
      return Promise.resolve(undefined);
    });

    render(<BrowserImportSection />);

    const copyButton = await screen.findByRole("button", {
      name: /copy i just applied button/i,
    });
    expect(copyButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/job page address/i), {
      target: { value: "https://jobs.example/posting/1" },
    });
    await waitFor(() => expect(copyButton).toBeEnabled());
    fireEvent.click(copyButton);

    expect(mockInvoke).toHaveBeenCalledWith("copy_applied_bookmarklet_code", {
      targetUrl: "https://jobs.example/posting/1",
    });
  });

  it("surfaces missing details before saving an applied draft", async () => {
    mockInvoke.mockImplementation((command: string, args?: unknown) => {
      if (command === "get_bookmarklet_config") {
        return Promise.resolve({ port: 4321, enabled: true });
      }
      if (command === "get_pending_bookmarklet_imports") {
        return Promise.resolve([
          {
            id: "applied-1",
            title: "Job title not added",
            company: "Company not added",
            url: "https://jobs.example/posting/1",
            location: null,
            description_preview: null,
            remote: false,
            operation: "applied_logging",
            missing_fields: ["title", "company"],
            received_at: "2026-07-19T12:00:00Z",
          },
        ]);
      }
      if (command === "confirm_pending_bookmarklet_imports") {
        expect(args).toEqual({ ids: ["applied-1"] });
        return Promise.resolve({ imported: 1, skipped: 0 });
      }
      return Promise.resolve(undefined);
    });

    render(<BrowserImportSection />);

    expect(await screen.findByText("Applied draft")).toBeInTheDocument();
    expect(
      screen.getByText("Missing details: Job title, Company"),
    ).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "confirm_pending_bookmarklet_imports",
      expect.anything(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: /save applied draft/i }),
    );

    expect(
      await screen.findByText("Saved 1 applied draft."),
    ).toBeInTheDocument();
  });
});
