import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import SetupWizard from "./SetupWizard";
import { ToastProvider } from "../../app/providers/ToastProvider";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

const renderWithProviders = (ui: React.ReactElement) => {
  return render(<ToastProvider>{ui}</ToastProvider>);
};

describe("SetupWizard alerts and sources", () => {
  const mockOnComplete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
  });

  it("keeps desktop alerts off until the user turns them on", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    await user.click(screen.getByRole("button", { name: /build my search/i }));
    await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    expect(screen.getByRole("checkbox", { name: /desktop alerts/i })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: /quiet job-search mode/i })).toBeDisabled();

    expect(screen.getAllByText("Alerts").length).toBeGreaterThan(0);
    expect(
      screen.getAllByText("Desktop alerts off; add alerts later in Settings").length,
    ).toBeGreaterThan(0);
    expect(screen.queryByLabelText(/slack connection link/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "complete_setup",
        expect.objectContaining({
          config: expect.objectContaining({
            title_allowlist: ["Office Manager"],
            alerts: expect.objectContaining({
              desktop: expect.objectContaining({
                enabled: false,
                play_sound: false,
                show_when_focused: false,
              }),
              slack: expect.objectContaining({
                enabled: false,
              }),
            }),
          }),
        }),
      );
    });
  });

  it("lets users turn desktop alert sound back on", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    await user.click(screen.getByRole("button", { name: /build my search/i }));
    await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    await user.click(screen.getByRole("checkbox", { name: /desktop alerts/i }));
    await user.click(screen.getByRole("checkbox", { name: /quiet job-search mode/i }));

    expect(screen.getByRole("checkbox", { name: /desktop alerts/i })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: /quiet job-search mode/i })).not.toBeChecked();
    expect(screen.getAllByText("Desktop alerts with sound").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "complete_setup",
        expect.objectContaining({
          config: expect.objectContaining({
            title_allowlist: ["Office Manager"],
            alerts: expect.objectContaining({
              desktop: expect.objectContaining({
                enabled: true,
                play_sound: true,
                show_when_focused: false,
              }),
            }),
          }),
        }),
      );
    });
  });

  it("offers tech-heavy sources but saves only selected sources", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    await user.click(screen.getByRole("button", { name: /build my search/i }));
    await user.type(screen.getByPlaceholderText("Add a job title..."), "Software Engineer{enter}");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    expect(screen.getByText("Job sources")).toBeInTheDocument();
    expect(
      screen.getByText("No outside job sources selected; add reviewed sources in Settings."),
    ).toBeInTheDocument();

    const remoteOkSource = screen.getByRole("checkbox", {
      name: /Remote OK/i,
    });
    const weWorkRemotelySource = screen.getByRole("checkbox", {
      name: /We Work Remotely/i,
    });
    const startupSource = screen.getByRole("checkbox", {
      name: /Startup and tech job posts/i,
    });

    expect(remoteOkSource).not.toBeChecked();
    expect(weWorkRemotelySource).not.toBeChecked();
    expect(startupSource).not.toBeChecked();

    await user.click(remoteOkSource);
    expect(screen.getByText("Remote OK selected.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "complete_setup",
        expect.objectContaining({
          config: expect.objectContaining({
            title_allowlist: ["Software Engineer"],
            remoteok: expect.objectContaining({ enabled: true }),
            hn_hiring: expect.objectContaining({ enabled: false }),
            weworkremotely: expect.objectContaining({ enabled: false }),
          }),
        }),
      );
    });
  });

  it("does not offer retired scheduled sources for non-technical searches", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    await user.click(screen.getByRole("button", { name: /build my search/i }));
    await user.type(screen.getByPlaceholderText("Add a job title..."), "Office Manager{enter}");
    await user.type(screen.getByPlaceholderText("Add a skill..."), "Scheduling{enter}");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));

    expect(
      screen.queryByRole("checkbox", { name: /SimplyHired/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("No outside job sources selected; add reviewed sources in Settings."),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "complete_setup",
        expect.objectContaining({
          config: expect.objectContaining({
            title_allowlist: ["Office Manager"],
            keywords_boost: ["Scheduling"],
            simplyhired: expect.objectContaining({
              enabled: false,
              query: "",
            }),
          }),
        }),
      );
    });
  });

  it("should have aria-live region for validation errors", () => {
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    const liveRegions = document.querySelectorAll('[aria-live="assertive"]');
    const validationRegion = Array.from(liveRegions).find((region) =>
      region.className?.includes("sr-only"),
    );

    expect(validationRegion).toBeDefined();
    expect(validationRegion).toHaveAttribute("aria-atomic", "true");
  });

  it("should have separate regions for step and validation announcements", () => {
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    const politeLiveRegions = document.querySelectorAll('[aria-live="polite"]');
    const assertiveLiveRegions = document.querySelectorAll('[aria-live="assertive"]');

    expect(politeLiveRegions.length).toBeGreaterThan(0);
    expect(assertiveLiveRegions.length).toBeGreaterThan(0);

    const politeElement = Array.from(politeLiveRegions).find((region) =>
      region.className?.includes("sr-only"),
    );
    const assertiveElement = Array.from(assertiveLiveRegions).find((region) =>
      region.className?.includes("sr-only"),
    );

    expect(politeElement).not.toBe(assertiveElement);
  });

  it("should use assertive aria-live for validation errors", () => {
    renderWithProviders(<SetupWizard onComplete={mockOnComplete} />);

    const assertiveRegions = document.querySelectorAll('[aria-live="assertive"]');
    const validationRegion = Array.from(assertiveRegions).find((region) =>
      region.className?.includes("sr-only"),
    );

    expect(validationRegion).toHaveAttribute("aria-live", "assertive");
    expect(validationRegion).toHaveAttribute("aria-atomic", "true");
  });
});
