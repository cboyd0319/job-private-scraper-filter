import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScraperHealthDashboard } from "./ScraperHealthDashboard";
import {
  mockRuns,
  mockScrapers,
  mockSummary,
  mockTestResults,
} from "./ScraperHealthDashboard.testData";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("../../../../shared/errorReporting/logger", () => ({
  logError: vi.fn(),
}));

describe("ScraperHealthDashboard sources and history", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("source table", () => {
    beforeEach(() => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") return Promise.resolve(mockScrapers);
        return Promise.resolve(null);
      });
    });

    it("displays source display names", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });
      expect(screen.getByText("USAJobs")).toBeInTheDocument();
      expect(screen.getByText("Lever")).toBeInTheDocument();
      expect(screen.getByText("Monster")).toBeInTheDocument();
    });

    it("hides hostile restored rows for policy-blocked scheduled sources", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") {
          return Promise.resolve([
            ...mockScrapers,
            ...["builtin", "dice", "simplyhired", "glassdoor"].map((source) => ({
              ...mockScrapers[2],
              scraper_name: source,
              display_name: `Restored ${source}`,
            })),
          ]);
        }
        return Promise.resolve(null);
      });

      render(<ScraperHealthDashboard onClose={onClose} />);

      await screen.findByText("Lever");
      for (const source of ["builtin", "dice", "simplyhired", "glassdoor"]) {
        expect(screen.queryByText(`Restored ${source}`)).not.toBeInTheDocument();
        expect(
          screen.queryByRole("button", { name: new RegExp(source, "i") }),
        ).not.toBeInTheDocument();
      }
    });

    it("does not offer to enable JobsWithGPT while provider review is pending", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") {
          return Promise.resolve([
            ...mockScrapers,
            {
              ...mockScrapers[2],
              scraper_name: "jobswithgpt",
              display_name: "JobsWithGPT",
            },
          ]);
        }
        return Promise.resolve(null);
      });

      render(<ScraperHealthDashboard onClose={onClose} />);

      expect(
        await screen.findByRole("button", {
          name: "JobsWithGPT provider review pending",
        }),
      ).toBeDisabled();
      expect(
        screen.getByText("Provider endpoint and usage policy review pending."),
      ).toBeInTheDocument();
    });

    it("does not display internal source ids", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.queryByText("indeed")).not.toBeInTheDocument();
      });
      expect(screen.queryByText("glassdoor")).not.toBeInTheDocument();
    });

    it("shows setup label for sources requiring setup", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Needs setup")).toBeInTheDocument();
      });
    });

    it("displays health status badges", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        const healthyBadges = screen.getAllByText("Working");
        expect(healthyBadges.length).toBeGreaterThan(0);
      });
      expect(screen.getAllByText("Having trouble").length).toBeGreaterThan(0);
    });

    it("displays plain source kind labels", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByRole("table", { name: /job source status/i })).toBeInTheDocument();
      });
      expect(screen.getByText("Kind")).toBeInTheDocument();
      expect(screen.getAllByText("Website page").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("Official source").length).toBeGreaterThan(0);
      expect(screen.getByText("Public job list")).toBeInTheDocument();
      expect(screen.queryByText("Feed")).not.toBeInTheDocument();
    });

    it("displays plain recent source status labels", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Recent Status")).toBeInTheDocument();
        expect(screen.queryByText("Checks Worked")).not.toBeInTheDocument();
        expect(screen.getAllByText("Mostly working").length).toBeGreaterThan(0);
      });
      expect(screen.getByText("Some trouble")).toBeInTheDocument();
      expect(screen.getByText("Needs attention")).toBeInTheDocument();
      expect(screen.queryByText("95%")).not.toBeInTheDocument();
      expect(screen.queryByText("72%")).not.toBeInTheDocument();
      expect(screen.queryByText("35%")).not.toBeInTheDocument();
    });

    it("displays average duration", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("2.5s")).toBeInTheDocument();
      });
      expect(screen.getByText("3.5s")).toBeInTheDocument();
    });

    it("displays jobs found and runs", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("150")).toBeInTheDocument();
      });
      expect(screen.getByText("from 24 checks")).toBeInTheDocument();
    });

    it("displays relative time for last success", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("30m ago")).toBeInTheDocument();
      });
      expect(screen.getByText("2h ago")).toBeInTheDocument();
    });

    it("shows 'Never' for sources that never succeeded", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Never")).toBeInTheDocument();
      });
    });

    it("displays plain page-read health for website sources", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Reads Job Details")).toBeInTheDocument();
      });
      expect(screen.queryByText("Can Read Jobs")).not.toBeInTheDocument();
      expect(screen.queryByText("Page Check")).not.toBeInTheDocument();
      expect(screen.getAllByText("Yes").length).toBeGreaterThan(0);
      expect(screen.getAllByText("Having trouble").length).toBeGreaterThan(0);
      expect(screen.getByText("Cannot read jobs")).toBeInTheDocument();
      expect(screen.getAllByText("Uses official source").length).toBeGreaterThan(0);
      expect(screen.queryByText("Not needed")).not.toBeInTheDocument();
    });

    it("has accessible table structure", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByRole("table", { name: /job source status/i })).toBeInTheDocument();
      });
    });

    it("shows plain next steps for each source", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getAllByText("Working. No action needed.").length).toBeGreaterThan(0);
      });
      expect(
        screen.getByText("Update connection in Settings if this keeps happening."),
      ).toBeInTheDocument();
      expect(screen.getByText("Off. Turn on if useful.")).toBeInTheDocument();
      expect(screen.getByText("Try again later or turn this source off.")).toBeInTheDocument();
    });

    it("shows visible source control labels instead of icon-only actions", async () => {
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Actions")).toBeInTheDocument();
      });
      expect(screen.queryByText("Source Controls")).not.toBeInTheDocument();
      expect(screen.getAllByText("Turn Off").length).toBeGreaterThan(0);
      expect(screen.getByText("Turn On")).toBeInTheDocument();
      expect(screen.getAllByText("Check Now").length).toBeGreaterThan(0);
    });
  });

  describe("source actions", () => {
    beforeEach(() => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") return Promise.resolve(mockScrapers);
        if (cmd === "set_scraper_enabled") return Promise.resolve(undefined);
        if (cmd === "run_scraper_smoke_test") return Promise.resolve(mockTestResults[0]);
        return Promise.resolve(null);
      });
    });

    it("toggles source enabled state", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /turn indeed off/i }));

      expect(mockInvoke).toHaveBeenCalledWith("set_scraper_enabled", {
        scraperName: "indeed",
        enabled: false,
      });
    });

    it("runs source check for one source", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /check indeed now/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("run_scraper_smoke_test", {
          scraperName: "indeed",
        });
      });
    });
  });

  describe("check history", () => {
    beforeEach(() => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") return Promise.resolve(mockScrapers);
        if (cmd === "get_scraper_runs") return Promise.resolve(mockRuns);
        return Promise.resolve(null);
      });
    });

    it("shows check history when source is selected", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        expect(screen.getByText(/recent checks: indeed/i)).toBeInTheDocument();
      });
    });

    it("loads check history data when source selected", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("get_scraper_runs", {
          scraperName: "indeed",
          limit: 20,
        });
      });
    });

    it("displays check status badges", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        expect(screen.getByText("Worked")).toBeInTheDocument();
      });
      expect(screen.getByText(/problem found after another try/i)).toBeInTheDocument();
    });

    it("displays job counts in check history", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        const historyPanel = screen.getByRole("region", { name: /recent checks/i });
        expect(within(historyPanel).getByText("Problem")).toBeInTheDocument();
        expect(within(historyPanel).queryByText("Issue")).not.toBeInTheDocument();
        expect(within(historyPanel).getByText("15")).toBeInTheDocument();
        expect(within(historyPanel).getByText("+3")).toBeInTheDocument();
      });
    });

    it("displays safe issue guidance in check history", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        expect(screen.getByText(/check your internet connection/i)).toBeInTheDocument();
      });
      expect(screen.queryByText("Timeout")).not.toBeInTheDocument();
    });

    it("hides check history when same source is clicked again", async () => {
      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));
      await waitFor(() => {
        expect(screen.getByText(/recent checks: indeed/i)).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));
      await waitFor(() => {
        expect(screen.queryByText(/recent checks: indeed/i)).not.toBeInTheDocument();
      });
    });

    it("shows no recent checks message when empty", async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === "get_health_summary") return Promise.resolve(mockSummary);
        if (cmd === "get_scraper_health") return Promise.resolve(mockScrapers);
        if (cmd === "get_scraper_runs") return Promise.resolve([]);
        return Promise.resolve(null);
      });

      const user = userEvent.setup();
      render(<ScraperHealthDashboard onClose={onClose} />);

      await waitFor(() => {
        expect(screen.getByText("Indeed")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Indeed"));

      await waitFor(() => {
        expect(screen.getByText("No recent checks found.")).toBeInTheDocument();
      });
    });
  });
});
