import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../app/providers/ToastProvider";
import SetupWizard from "./SetupWizard";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);
const mockOnComplete = vi.fn();

function renderWizard(onSkip = vi.fn()) {
  return render(
    <ToastProvider>
      <SetupWizard onComplete={mockOnComplete} onSkip={onSkip} />
    </ToastProvider>,
  );
}

async function reachReview(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: /build my search/i }));
  await user.click(screen.getByRole("button", { name: /add office assistant job title/i }));
  await user.click(screen.getByRole("button", { name: /^continue$/i }));
  await user.click(screen.getByRole("button", { name: /^continue$/i }));
}

describe("SetupWizard first-run choices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.sessionStorage.clear();
  });

  it("skips only this session without saving or starting background work", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    renderWizard(onSkip);

    expect(screen.getByText(
      "Skipping lasts only for this session and saves no search. Setup returns next time. You can still review or import local jobs.",
    )).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Skip for now" }));

    expect(onSkip).toHaveBeenCalledOnce();
    expect(mockOnComplete).not.toHaveBeenCalled();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("retains choices after a save failure and retries the same setup", async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValueOnce(new Error("disk unavailable")).mockResolvedValueOnce(undefined);
    renderWizard();
    await reachReview(user);
    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));

    expect(await screen.findByText(
      "Could not save your search setup. Your choices are still here.",
    )).toBeInTheDocument();
    expect(mockOnComplete).not.toHaveBeenCalled();
    expect(screen.getByText("Office Assistant")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
      expect(mockOnComplete).toHaveBeenCalledOnce();
    });
  });

  it("does not start a second setup save while retrying", async () => {
    const user = userEvent.setup();
    let finishSave: (() => void) | undefined;
    mockInvoke
      .mockRejectedValueOnce(new Error("disk unavailable"))
      .mockImplementationOnce(() => new Promise<void>((resolve) => {
        finishSave = resolve;
      }));
    renderWizard();
    await reachReview(user);
    await user.click(screen.getByRole("button", { name: /start finding jobs/i }));
    await user.click(await screen.findByRole("button", { name: "Try again" }));

    const savingButton = await screen.findByRole("button", { name: "Saving setup" });
    expect(savingButton).toBeDisabled();
    await user.click(savingButton);
    expect(mockInvoke).toHaveBeenCalledTimes(2);

    finishSave?.();
    await waitFor(() => expect(mockOnComplete).toHaveBeenCalledOnce());
  });
});
