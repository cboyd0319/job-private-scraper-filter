import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import SetupWizard from "./SetupWizard";
import { ToastProvider } from "../../app/providers/ToastProvider";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <ToastProvider>
      {ui}
    </ToastProvider>
  );
};

describe("SetupWizard Accessibility", () => {
  const mockOnComplete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
  });

  it("keeps the step heading legible in the dark app shell", () => {
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    expect(
      screen.getByRole("heading", { name: "Work You Want" }),
    ).toHaveClass("dark:text-white");
  });

  describe("Progress Announcements", () => {
    it("should have aria-live region for step announcements", () => {
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      // Find live region with step announcement
      const liveRegions = document.querySelectorAll('[aria-live="polite"]');
      const stepRegion = Array.from(liveRegions).find(region =>
        region.textContent?.includes("Step 1 of 4")
      );

      expect(stepRegion).toBeDefined();
      expect(stepRegion).toHaveAttribute("aria-atomic", "true");
      expect(stepRegion?.className).toContain("sr-only");
    });

    it("should announce initial step on render", () => {
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      const liveRegions = document.querySelectorAll('[aria-live="polite"]');
      const stepRegion = Array.from(liveRegions).find(region =>
        region.textContent?.includes("Work You Want")
      );

      expect(stepRegion?.textContent).toContain("Step 1 of 4: Work You Want");
    });

    it("should have proper ARIA attributes on live region", () => {
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      const politeLiveRegions = document.querySelectorAll('[aria-live="polite"]');
      const stepRegion = Array.from(politeLiveRegions).find(r =>
        r.className?.includes("sr-only")
      );

      expect(stepRegion).toHaveAttribute("aria-live", "polite");
      expect(stepRegion).toHaveAttribute("aria-atomic", "true");
    });
  });

  describe("Validation Feedback", () => {
    it("lets users continue with their own search by default", async () => {
      const user = userEvent.setup();
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      const continueButton = screen.getByRole("button", {
        name: /build my search/i,
      });

      expect(continueButton).toBeEnabled();
      await user.click(continueButton);

      expect(screen.getByText("Job Basics")).toBeInTheDocument();
      expect(screen.getByText("Add at least one job title")).toBeInTheDocument();
    });

    it("lets users add a common starter job title without typing", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.click(screen.getByRole("button", { name: /add office assistant job title/i }));

      expect(screen.getByText("Office Assistant")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Office Assistant"],
            }),
          }),
        );
      });
    });

    it("starts office and admin users with non-technical local defaults", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(
        screen.getByRole("radio", {
          name: /Office & Admin: Administrative assistants, office managers, and coordinators/i,
        }),
      );
      await user.click(screen.getByRole("button", { name: /use these starting ideas/i }));

      expect(screen.getByText(/Started with/i)).toBeInTheDocument();
      expect(screen.getByText("Office & Admin")).toBeInTheDocument();
      expect(screen.getByText("Office Manager")).toBeInTheDocument();
      expect(screen.getByText("Administrative Assistant")).toBeInTheDocument();
      expect(screen.getByText("Scheduling")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));

      expect(
        screen.getByText("No outside job sources selected; add reviewed sources in Settings."),
      ).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: expect.arrayContaining([
                "Office Manager",
                "Administrative Assistant",
              ]),
              keywords_boost: expect.arrayContaining(["Scheduling"]),
              salary_floor_usd: 40000,
              location_preferences: expect.objectContaining({
                allow_remote: false,
                allow_hybrid: true,
                allow_onsite: true,
              }),
              remoteok: expect.objectContaining({ enabled: false }),
              hn_hiring: expect.objectContaining({ enabled: false }),
              weworkremotely: expect.objectContaining({ enabled: false }),
            }),
          }),
        );
      });
    });

    it("saves work to avoid as search words to rank lower", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));

      await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
      await user.type(screen.getByPlaceholderText("Add a skill..."), "Scheduling{enter}");
      await user.type(
        screen.getByPlaceholderText("e.g., night shift, heavy travel"),
        "night shift{enter}",
      );
      await user.type(screen.getByLabelText("Minimum pay"), "65000");

      expect(screen.getByText("night shift")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Office Manager"],
              keywords_boost: ["Scheduling"],
              keywords_exclude: ["night shift"],
              salary_floor_usd: 65000,
              immediate_alert_threshold: 0.9,
              remoteok: expect.objectContaining({
                enabled: false,
                limit: 50,
              }),
              hn_hiring: expect.objectContaining({
                enabled: false,
                limit: 100,
              }),
              weworkremotely: expect.objectContaining({
                enabled: false,
                limit: 50,
              }),
              ghost_config: expect.objectContaining({
                stale_threshold_days: 30,
                repost_threshold: 2,
                warning_threshold: 0.2,
                hide_threshold: 0.75,
              }),
            }),
          }),
        );
      });
    });

    it("lets users add common schedule or travel deal breakers without typing", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
      await user.click(screen.getByRole("button", { name: /add night shift to rank lower/i }));

      expect(screen.getByText("night shift")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Office Manager"],
              keywords_exclude: ["night shift"],
            }),
          }),
        );
      });
    });

    it("shows a plain search summary before scanning starts", async () => {
      const user = userEvent.setup();
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
      await user.type(screen.getByPlaceholderText("Add a skill..."), "Scheduling{enter}");
      await user.type(
        screen.getByPlaceholderText("e.g., night shift, heavy travel"),
        "night shift{enter}",
      );
      await user.type(screen.getByLabelText("Minimum pay"), "65000");

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));

      const reviewHeading = screen.getByRole("heading", { name: /review your search/i });
      const reviewSection = reviewHeading.closest("section");
      expect(reviewSection).not.toBeNull();
      const review = within(reviewSection as HTMLElement);

      expect(screen.getByText("Look for")).toBeInTheDocument();
      expect(screen.getByText("Office Manager")).toBeInTheDocument();
      expect(screen.getByText("Show more")).toBeInTheDocument();
      expect(screen.getByText("Scheduling")).toBeInTheDocument();
      expect(screen.getByText("Rank lower")).toBeInTheDocument();
      expect(screen.getByText("night shift")).toBeInTheDocument();
      expect(screen.getByText("remote, hybrid, on-site")).toBeInTheDocument();
      expect(review.getByText("Freshness")).toBeInTheDocument();
      expect(review.getByText("Fresh and verified first")).toBeInTheDocument();
      expect(review.getByText("Review list")).toBeInTheDocument();
      expect(review.getByText("Balanced list")).toBeInTheDocument();
      expect(review.getByText("Job sources")).toBeInTheDocument();
      expect(
        review.getByText("No outside job sources selected; add reviewed sources in Settings."),
      ).toBeInTheDocument();
      expect(
        screen.getByText("At least $65,000/year"),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/desktop alerts are optional/i),
      ).toBeInTheDocument();
      expect(screen.getByText(/saves these search settings on this computer/i)).toBeInTheDocument();
      expect(screen.getByText(/can contact only checked job sources in this review/i)).toBeInTheDocument();
      expect(screen.getByText(/does not send resumes, private notes, saved answers, or application history/i)).toBeInTheDocument();
      expect(screen.queryByText(/only contacts job sources or alert services/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/nothing is sent anywhere/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/great matches|great jobs/i)).not.toBeInTheDocument();
    });

    it("lets users mark pay as not sure before scanning starts", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");

      const payInput = screen.getByLabelText("Minimum pay");
      await user.type(payInput, "65000");
      expect(payInput).toHaveValue(65000);
      expect(screen.getByText("$65,000/year")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /not sure about pay yet/i }));

      expect(payInput).toHaveValue(null);
      expect(
        screen.getByText("Jobs without pay stay visible and marked."),
      ).toBeInTheDocument();
      expect(screen.queryByText(/warn when listed pay is below/i)).not.toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      expect(
        screen.getByText("Show jobs even when pay is missing or not listed"),
      ).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Office Manager"],
              salary_floor_usd: 0,
            }),
          }),
        );
      });
    });

    it("saves an hourly pay floor as yearly salary floor", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Warehouse Associate{enter}");

      await user.click(screen.getByRole("radio", { name: /^Hourly$/i }));
      const payInput = screen.getByLabelText("Minimum pay");
      await user.type(payInput, "20");

      expect(payInput).toHaveValue(20);
      expect(screen.getByText("$20/hour, about $41,600/year")).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      expect(
        screen.getByText("At least $20/hour, about $41,600/year"),
      ).toBeInTheDocument();

      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Warehouse Associate"],
              salary_floor_usd: 41600,
            }),
          }),
        );
      });
    });

    it("keeps raw chat alert setup out of first-run setup", async () => {
      const user = userEvent.setup();
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));

      expect(screen.getByText(/desktop alerts are optional/i)).toBeInTheDocument();
      expect(screen.getByText(/email or chat alerts can be added later in settings/i)).toBeInTheDocument();
      expect(screen.queryByText(/in-app alerts/i)).not.toBeInTheDocument();
      expect(screen.queryByLabelText(/slack connection link/i)).not.toBeInTheDocument();
      expect(screen.queryByPlaceholderText(/hooks\.slack/i)).not.toBeInTheDocument();
    });

    it("saves a wider freshness preference without technical setup", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Bookkeeper{enter}");
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));

      await user.click(screen.getByRole("radio", { name: /widest search/i }));

      expect(screen.getByText("Freshness")).toBeInTheDocument();
      expect(screen.getAllByText("Widest search").length).toBeGreaterThan(0);

      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Bookkeeper"],
              ghost_config: expect.objectContaining({
                stale_threshold_days: 120,
                repost_threshold: 5,
                warning_threshold: 0.5,
                hide_threshold: 0.85,
              }),
            }),
          }),
        );
      });
    });

    it("saves broad review volume as a wider local search", async () => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValue(undefined);
      renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

      await user.click(screen.getByRole("button", { name: /build my search/i }));
      await user.type(screen.getByPlaceholderText("Add a job title..."), "Medical Assistant{enter}");
      await user.click(screen.getByRole("button", { name: /^continue$/i }));
      await user.click(screen.getByRole("button", { name: /^continue$/i }));

      await user.click(screen.getByRole("radio", { name: /broad discovery/i }));

      expect(screen.getByText("Review list")).toBeInTheDocument();
      expect(screen.getAllByText("Broad discovery").length).toBeGreaterThan(0);

      await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          "complete_setup",
          expect.objectContaining({
            config: expect.objectContaining({
              title_allowlist: ["Medical Assistant"],
              immediate_alert_threshold: 0.85,
              remoteok: expect.objectContaining({
                enabled: false,
                limit: 75,
              }),
              hn_hiring: expect.objectContaining({
                enabled: false,
                limit: 150,
              }),
              weworkremotely: expect.objectContaining({
                enabled: false,
                limit: 75,
              }),
            }),
          }),
        );
      });
    });

  });

});
