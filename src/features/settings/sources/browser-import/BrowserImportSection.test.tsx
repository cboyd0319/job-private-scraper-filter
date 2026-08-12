import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { BrowserImportSection } from "./BrowserImportSection";
import {
  BROWSER_ASSIST_LEARNING_ENABLED_STORAGE_KEY,
  BROWSER_ASSIST_LEARNING_STORAGE_KEY,
} from "../../../../shared/browserAssistLearning";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("BrowserImportSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it("copies an origin-bound browser button without exposing its pairing secret", async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === "get_bookmarklet_config") {
        return Promise.resolve({ port: 4321, enabled: true });
      }
      if (command === "get_pending_bookmarklet_imports") {
        return Promise.resolve([]);
      }
      if (command === "copy_bookmarklet_code") {
        return Promise.resolve(true);
      }
      return Promise.resolve(undefined);
    });

    render(<BrowserImportSection />);

    const copyButton = await screen.findByRole("button", {
      name: /copy browser button/i,
    });
    expect(copyButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/job page address/i), {
      target: { value: "https://jobs.example/posting/1" },
    });
    await waitFor(() => expect(copyButton).toBeEnabled());
    fireEvent.click(copyButton);

    expect(mockInvoke).toHaveBeenCalledWith("get_bookmarklet_config");
    expect(mockInvoke).toHaveBeenCalledWith("copy_bookmarklet_code", {
      targetUrl: "https://jobs.example/posting/1",
    });
    expect(screen.queryByText(/token-123/)).not.toBeInTheDocument();
    expect(screen.queryByText(/pairing secret/i)).not.toBeInTheDocument();
  });

  it("uses plain browser-import copy instead of server and structured-data jargon", async () => {
    mockInvoke.mockResolvedValueOnce({
      port: 4321,
      enabled: true,
    });

    render(<BrowserImportSection />);

    expect(
      await screen.findByRole("heading", { name: /install browser button/i }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Connection Number")).not.toBeInTheDocument();
    expect(screen.queryByText("Support number")).not.toBeInTheDocument();
    expect(screen.queryByText("Browser helper number")).not.toBeInTheDocument();
    expect(screen.queryByText("Import Helper")).not.toBeInTheDocument();
    expect(screen.getByText("Browser Import")).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /turn off/i }),
      ).toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", { name: /copy browser button/i }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(/copy a fresh.*every import/i).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getByText(/copy a fresh browser button/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/expires after about ten minutes/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/closed and reopened/i)).not.toBeInTheDocument();
    expect(
      screen.queryByText(/when JobSentinel restarts/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/local safety code/i)).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: /optional browser button setting/i }),
    );
    expect(
      screen.queryByText("Advanced browser button setting"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("Browser helper number")).not.toBeInTheDocument();
    expect(screen.getByText("Button setup number")).toBeInTheDocument();
    expect(screen.getByLabelText("Button setup number")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /save number/i })).toBeDisabled();
    expect(screen.queryByText("Connection Number")).not.toBeInTheDocument();
    expect(screen.queryByText("Support number")).not.toBeInTheDocument();
    expect(
      screen.getByText(/help instructions tell you otherwise/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/unless a support reply asks/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/server port/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/bookmarklet code/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/bookmarklet/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/local safety token/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/schema\.org/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/DOM parsing/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/any job posting/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/supported sites/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/major job boards/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/import helper/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^LinkedIn$/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/^Indeed$/i)).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /local review list/i }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Cmd\/Ctrl\+D/i)).not.toBeInTheDocument();
    expect(
      screen.queryByText(/bookmark address field/i),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/recommended: paste the job link into jobsentinel/i),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(
        /use browser button only if you already use browser bookmarks/i,
      ),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        /recommended: use the browser button on an individual job page/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/visible posting details you choose/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /paste a link only when the browser button cannot read the page/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Use your browser's Add Bookmark option/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/paste the copied text into the bookmark link field/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/where the bookmark stores the page address/i),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(/open an individual job page/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/company application pages/i)).toBeInTheDocument();
    expect(
      screen.getByText(/do not let JobSentinel read saved pages/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/respects those controls/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/job page address/i)).toHaveAttribute(
      "type",
      "url",
    );
    expect(
      screen.queryByText(/Restricted Site Warning/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("starts the local receiver without a renderer-only risk acknowledgement", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        port: 4321,
        enabled: false,
      })
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({
        port: 4321,
        enabled: true,
      });

    render(<BrowserImportSection />);

    const turnOnButton = await screen.findByRole("button", {
      name: /turn on/i,
    });
    fireEvent.click(turnOnButton);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("start_bookmarklet_server", {
        port: 4321,
      }),
    );
  });

  it("uses an available setup number and asks for a fresh browser button when the saved number is occupied", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        port: 4321,
        enabled: false,
      })
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({
        port: 54321,
        enabled: true,
      });

    render(<BrowserImportSection />);

    fireEvent.click(await screen.findByRole("button", { name: /turn on/i }));

    expect(
      await screen.findByText(
        /found an available setup number because the saved one was in use/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/copy a fresh browser button before importing/i),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: /optional browser button setting/i }),
    );
    expect(screen.getByLabelText("Button setup number")).toHaveValue(54321);
  });

  it("shows safe copy guidance when browser clipboard copy fails", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        port: 4321,
        enabled: true,
      })
      .mockResolvedValueOnce([])
      .mockRejectedValueOnce(new Error("token=secret resume=private-file"));

    render(<BrowserImportSection />);

    const copyButton = await screen.findByRole("button", {
      name: /copy browser button/i,
    });
    fireEvent.change(screen.getByLabelText(/job page address/i), {
      target: { value: "https://jobs.example/posting/1" },
    });
    await waitFor(() => expect(copyButton).toBeEnabled());
    fireEvent.click(copyButton);

    expect(
      await screen.findByText(/could not copy browser button/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /allow clipboard access, then click copy browser button again/i,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/safe support report/i)).toBeInTheDocument();
    expect(screen.queryByText(/token=secret/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/resume=private-file/i)).not.toBeInTheDocument();
  });

  it("shows plain recovery when Browser Import cannot load", async () => {
    mockInvoke.mockRejectedValueOnce(
      new Error("token=secret resume=private-file"),
    );

    render(<BrowserImportSection />);

    expect(
      await screen.findByText(/Browser Import could not load/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/close and reopen settings/i)).toBeInTheDocument();
    expect(screen.getByText(/safe support report/i)).toBeInTheDocument();
    expect(
      screen.queryByText(/browser import settings/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/token=secret/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/resume=private-file/i)).not.toBeInTheDocument();
  });

  it("shows plain recovery when Browser Import cannot be changed", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        port: 4321,
        enabled: false,
      })
      .mockResolvedValueOnce([])
      .mockRejectedValueOnce(new Error("token=secret resume=private-file"));

    render(<BrowserImportSection />);

    const turnOnButton = await screen.findByRole("button", {
      name: /turn on/i,
    });
    fireEvent.click(turnOnButton);

    expect(
      await screen.findByText(/Browser Import could not be changed/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/safe support report/i)).toBeInTheDocument();
    expect(
      screen.queryByText(/Could not update browser import/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/token=secret/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/resume=private-file/i)).not.toBeInTheDocument();
  });

  it("shows plain recovery when the browser button number cannot be saved", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        port: 4321,
        enabled: false,
      })
      .mockResolvedValueOnce([])
      .mockRejectedValueOnce(new Error("token=secret resume=private-file"));

    render(<BrowserImportSection />);

    fireEvent.click(
      await screen.findByRole("button", {
        name: /optional browser button setting/i,
      }),
    );
    fireEvent.change(screen.getByLabelText("Button setup number"), {
      target: { value: "4322" },
    });
    fireEvent.click(screen.getByRole("button", { name: /save number/i }));

    expect(
      await screen.findByText(/That browser button number could not be saved/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/help instructions give you a different number/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/browser import connection/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/token=secret/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/resume=private-file/i)).not.toBeInTheDocument();
  });

  it("validates the browser button number before saving", async () => {
    mockInvoke.mockResolvedValueOnce({
      port: 4321,
      enabled: false,
    });

    render(<BrowserImportSection />);

    fireEvent.click(
      await screen.findByRole("button", {
        name: /optional browser button setting/i,
      }),
    );
    const portInput = screen.getByLabelText("Button setup number");
    const saveButton = screen.getByRole("button", { name: /save number/i });

    fireEvent.change(portInput, { target: { value: "80" } });

    expect(
      await screen.findByText(/Use a number from 1024 to 65535/i),
    ).toBeInTheDocument();
    expect(portInput).toHaveAttribute("aria-invalid", "true");
    expect(saveButton).toBeDisabled();
    expect(mockInvoke).not.toHaveBeenCalledWith("set_bookmarklet_port", {
      port: 80,
    });

    fireEvent.change(portInput, { target: { value: "4322" } });

    await waitFor(() => expect(saveButton).toBeEnabled());
    fireEvent.click(saveButton);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("set_bookmarklet_port", {
        port: 4322,
      }),
    );
  });

  it("shows browser imports as review items and saves only after user confirmation", async () => {
    window.localStorage.setItem(
      BROWSER_ASSIST_LEARNING_ENABLED_STORAGE_KEY,
      "true",
    );
    mockInvoke.mockImplementation((command: string, args?: unknown) => {
      if (command === "get_bookmarklet_config") {
        return Promise.resolve({
          port: 4321,
          enabled: true,
        });
      }
      if (command === "get_pending_bookmarklet_imports") {
        return Promise.resolve([
          {
            id: "pending-1",
            title: "Principal Systems Security Engineer",
            company: "Example Cooperative",
            url: "https://careers.example.com/jobs/100",
            location: "Centennial, CO",
            description_preview: "Visible job details selected by the user",
            remote: false,
            operation: "visible_page_capture",
            missing_fields: [],
            received_at: "2026-06-19T12:00:00Z",
          },
        ]);
      }
      if (command === "confirm_pending_bookmarklet_imports") {
        expect(args).toEqual({ ids: ["pending-1"] });
        return Promise.resolve({ imported: 1, skipped: 0 });
      }
      return Promise.resolve(undefined);
    });

    render(<BrowserImportSection />);

    expect(
      await screen.findByText(/Jobs waiting for review/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Principal Systems Security Engineer"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Example Cooperative/)).toBeInTheDocument();
    expect(
      screen.getByText(/These jobs are not saved yet/i),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: /save principal systems security engineer/i,
      }),
    );

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "confirm_pending_bookmarklet_imports",
        {
          ids: ["pending-1"],
        },
      ),
    );
    expect(
      await screen.findByText(/Saved 1 browser import/i),
    ).toBeInTheDocument();
    expect(
      window.localStorage.getItem(BROWSER_ASSIST_LEARNING_STORAGE_KEY),
    ).toContain("Principal Systems Security Engineer");
    expect(
      window.localStorage.getItem(BROWSER_ASSIST_LEARNING_STORAGE_KEY),
    ).toContain("Example Cooperative");
    expect(
      window.localStorage.getItem(BROWSER_ASSIST_LEARNING_STORAGE_KEY),
    ).not.toContain("careers.example.com/jobs/100");
  });

});
