import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  makeConfig,
  makeGhostConfig,
  mockInvoke,
} from "./SettingsPage.testSupport";
import Settings from "./SettingsPage";

async function openSourceRecommendations(
  user: ReturnType<typeof userEvent.setup>,
  title: string,
  keywords: string[],
) {
  const config = makeConfig();
  config.title_allowlist = [title];
  config.keywords_boost = keywords;
  config.location_preferences.allow_remote = true;
  config.location_preferences.cities = ["Austin"];
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
}

describe("Settings source setup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the app-composed LinkedIn workbench", async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return makeConfig();
      if (cmd === "has_credential") return false;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      return null;
    });

    render(
      <Settings
        linkedinWorkbench={<div>Composed LinkedIn workbench</div>}
        onClose={vi.fn()}
      />,
    );

    await user.click(
      await screen.findByRole("tab", { name: "Sources & Alerts" }),
    );

    expect(screen.getByText("Composed LinkedIn workbench")).toBeInTheDocument();
  });

  it("does not recommend tech-heavy sources for broad remote searches", async () => {
    const user = userEvent.setup();
    await openSourceRecommendations(user, "Office Manager", ["Scheduling"]);

    expect(
      screen.queryByText("Optional sources to review"),
    ).not.toBeInTheDocument();
  });

  it("does not recommend tech-heavy sources for broad searches with common tool keywords", async () => {
    const user = userEvent.setup();
    await openSourceRecommendations(user, "Accountant", ["SQL", "Excel"]);

    expect(
      screen.queryByText("Optional sources to review"),
    ).not.toBeInTheDocument();
  });

  it("recommends tech-heavy sources for technical searches", async () => {
    const user = userEvent.setup();
    await openSourceRecommendations(user, "Software Engineer", ["React"]);

    const sourceReview = screen.getByText("Optional sources to review");
    const panel = sourceReview.parentElement?.parentElement;
    expect(panel).toBeInstanceOf(HTMLElement);
    const panelQueries = within(panel as HTMLElement);

    expect(panelQueries.getByText("RemoteOK")).toBeInTheDocument();
    expect(panelQueries.getByText("WeWorkRemotely")).toBeInTheDocument();
    expect(
      panelQueries.getByText("Startup and tech job posts"),
    ).toBeInTheDocument();
    expect(
      panelQueries.queryByText("Hacker News Who's Hiring"),
    ).not.toBeInTheDocument();
    expect(panelQueries.getByText(/remote tech roles/i)).toBeInTheDocument();
    expect(
      panelQueries.getAllByRole("button", { name: "Review source" }).length,
    ).toBeGreaterThan(0);
  });

  it("does not restore retired scheduled controls from legacy config", async () => {
    const user = userEvent.setup();
    const config = makeConfig();
    config.simplyhired.enabled = true;
    config.glassdoor.enabled = true;

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

    expect(
      screen.queryByText(/Restricted source warning/i),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(
        /Scheduled access to Built In, Dice, SimplyHired, and Glassdoor is unavailable after provider policy review/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("See which sources are working and what to try next"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Cloudflare protection/i),
    ).not.toBeInTheDocument();
  });

  it("introduces additional job sources without provider-name prerequisites", async () => {
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

    expect(
      screen.getByText(/public company career pages and selected job sites/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Greenhouse, Lever, and other popular job boards/i),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", {
        name: /Turn Remote OK scheduled job checks on or off/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", {
        name: /Turn startup and tech hiring post checks on or off/i,
      }),
    ).toBeInTheDocument();
    await user.click(screen.getByText("More Job Boards"));
    expect(
      screen.queryByRole("checkbox", {
        name: /Turn YC Startup scheduled job checks on or off/i,
      }),
    ).not.toBeInTheDocument();
  });

  it("labels USAJobs source setup as optional scheduled checks", async () => {
    const user = userEvent.setup();
    const config = makeConfig();

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_config") return config;
      if (cmd === "get_ghost_config") return makeGhostConfig();
      if (cmd === "detect_location") return null;
      return null;
    });

    const { container } = render(<Settings onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("tab", { name: "Sources & Alerts" }));
    await user.click(
      screen.getByRole("checkbox", {
        name: /Turn USAJobs scheduled job checks on or off/i,
      }),
    );

    expect(
      screen.getByText(/Optional USAJobs scheduled checks/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Skip this if you only want to open USAJobs/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", {
        name: /Turn USAJobs scheduled job checks on or off/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/JobSentinel contacts USAJobs on your schedule/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /access code, USAJobs email, search words, location,\s*remote choice, how recent jobs should be/i,
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Search words")).toBeInTheDocument();
    expect(screen.getByText("Posted in last:")).toBeInTheDocument();
    expect(screen.getByText("Jobs to check:")).toBeInTheDocument();
    expect(screen.queryByLabelText("Keywords")).not.toBeInTheDocument();
    expect(screen.queryByText("Posted within:")).not.toBeInTheDocument();
    expect(screen.queryByText("Max results:")).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Optional USAJobs auto-check/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/automatic checks/i)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", { name: /automatic checks/i }),
    ).not.toBeInTheDocument();
    expect(screen.getByText(/scheduled USAJobs checks/i)).toBeInTheDocument();
    expect(container.textContent ?? "").not.toMatch(
      /automatic\s+USAJobs\s+checks/i,
    );
    expect(
      screen.getByRole("link", {
        name: /Open USAJobs search in your browser/i,
      }),
    ).toHaveAttribute("href", "https://www.usajobs.gov/Search/Results");
    expect(
      screen.getByRole("link", { name: /Open USAJobs access-code request/i }),
    ).toHaveAttribute("href", "https://developer.usajobs.gov/APIRequest/Index");
    expect(screen.queryByText(/Quick Setup/i)).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Advanced federal monitoring/i),
    ).not.toBeInTheDocument();
  });
});
