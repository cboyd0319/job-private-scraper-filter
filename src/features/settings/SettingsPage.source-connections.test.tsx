import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  makeConfig,
  makeGhostConfig,
  mockInvoke,
  mockToast,
} from "./SettingsPage.testSupport";
import Settings from "./SettingsPage";

describe("Settings source connection saves", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("blocks USAJobs save before keychain writes when required details are missing", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const config = makeConfig();

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return config;
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      if (cmd === "save_config") return null;
      if (cmd === "store_credential") return null;
      return null;
    });

    render(<Settings onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.click(
      screen.getByRole("checkbox", {
        name: /Turn USAJobs scheduled job checks on or off/i,
      }),
    );
    await user.click(screen.getByRole("button", { name: /save changes/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Finish USAJobs scheduled checks",
      "Add the USAJobs email and access code shown below, or turn USAJobs scheduled checks off.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "save_config",
      expect.anything(),
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "store_credential",
      expect.objectContaining({ key: "usajobs_api_key" }),
    );
    expect(onClose).not.toHaveBeenCalled();
  });

  it("does not offer retired restricted scheduled checks", async () => {
    const user = userEvent.setup();
    const config = makeConfig();

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return config;
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
    expect(
      screen.queryAllByRole("checkbox", {
        name: /Turn (BuiltIn|Dice|SimplyHired|Glassdoor) scheduled job checks on or off/i,
      }),
    ).toHaveLength(0);
    expect(
      screen.getByText(
        /scheduled access to Built In, Dice, SimplyHired, and Glassdoor is unavailable after provider policy review/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /Dice's official MCP option still needs a reviewed privacy, schema, and pacing contract/i,
      ),
    ).toBeInTheDocument();
  });

  it("blocks saving an invalid Discord connection link", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const config = makeConfig();
    config.alerts.discord.enabled = true;

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return config;
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      if (cmd === "save_config") return null;
      if (cmd === "store_credential") return null;
      return null;
    });

    render(<Settings onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.type(
      screen.getByPlaceholderText(/Discord connection link/i),
      "https://evil.com/api/webhooks/123/abc",
    );
    await user.click(screen.getByRole("button", { name: /save changes/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Check Discord connection link",
      "Paste the full Discord connection link copied from Discord. If you are not sure, leave it blank and set it up later.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "save_config",
      expect.anything(),
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "store_credential",
      expect.objectContaining({ key: "discord_webhook" }),
    );
    expect(onClose).not.toHaveBeenCalled();
  });

  it("blocks saving an invalid Teams connection link", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const config = makeConfig();
    config.alerts.teams.enabled = true;

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return config;
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      if (cmd === "save_config") return null;
      if (cmd === "store_credential") return null;
      return null;
    });

    render(<Settings onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.type(
      screen.getByPlaceholderText(/Teams connection link/i),
      "https://evil.com/webhook/abc",
    );
    await user.click(screen.getByRole("button", { name: /save changes/i }));

    expect(mockToast.error).toHaveBeenCalledWith(
      "Check Teams connection link",
      "Paste the full Teams connection link copied from Teams. If you are not sure, leave it blank and set it up later.",
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "save_config",
      expect.anything(),
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "store_credential",
      expect.objectContaining({ key: "teams_webhook" }),
    );
    expect(onClose).not.toHaveBeenCalled();
  });
});
