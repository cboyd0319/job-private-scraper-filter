import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../app/providers/ToastProvider";
import { openDeepLink } from "../../shared/search-links";
import {
  getLinkedInWorkbenchReviewStatus,
  recordLinkedInWorkbenchEvent,
  reviewLinkedInWorkbench,
  revokeLinkedInWorkbenchReview,
} from "./internal/linkedinWorkbenchClient";
import {
  BROWSER_ASSIST_LEARNING_ENABLED_STORAGE_KEY,
  BROWSER_ASSIST_LEARNING_STORAGE_KEY,
} from "../../shared/browserAssistLearning";
import { LinkedInWorkbench } from "./LinkedInWorkbench";

vi.mock("../../shared/search-links", () => ({
  openDeepLink: vi.fn(),
}));

vi.mock("./internal/linkedinWorkbenchClient", () => ({
  getLinkedInWorkbenchReviewStatus: vi.fn(),
  recordLinkedInWorkbenchEvent: vi.fn(),
  reviewLinkedInWorkbench: vi.fn(),
  revokeLinkedInWorkbenchReview: vi.fn(),
}));

function renderWorkbench() {
  render(
    <ToastProvider>
      <LinkedInWorkbench />
    </ToastProvider>,
  );
}

async function reviewWorkbench(user: ReturnType<typeof userEvent.setup>) {
  await user.click(
    screen.getByLabelText(/I understand\. Remember this on this computer/i),
  );
  await waitFor(() =>
    expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeEnabled(),
  );
}

describe("LinkedInWorkbench", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(window.localStorage.getItem).mockReturnValue(null);
    vi.mocked(openDeepLink).mockResolvedValue(undefined);
    vi.mocked(getLinkedInWorkbenchReviewStatus).mockResolvedValue("review_required");
    vi.mocked(reviewLinkedInWorkbench).mockResolvedValue(true);
    vi.mocked(revokeLinkedInWorkbenchReview).mockResolvedValue(true);
    vi.mocked(recordLinkedInWorkbenchEvent).mockResolvedValue({
      jobId: 1,
      jobHash: "hash",
      applicationId: 2,
      status: "applied",
      needsDetails: true,
      savedAsBookmark: true,
      hidden: false,
    });
  });

  it("keeps LinkedIn actions disabled until the user acknowledges the workbench", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /log applied/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /log interview/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /add reminder/i })).toBeDisabled();

    await reviewWorkbench(user);

    expect(reviewLinkedInWorkbench).toHaveBeenCalledOnce();
    expect(window.localStorage.setItem).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /log applied/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /log interview/i })).toBeEnabled();
    expect(screen.getByRole("button", { name: /add reminder/i })).toBeEnabled();
  });

  it("loads and revokes the backend-owned review without browser storage", async () => {
    const user = userEvent.setup();
    vi.mocked(getLinkedInWorkbenchReviewStatus).mockResolvedValue("reviewed");
    renderWorkbench();

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeEnabled(),
    );
    expect(
      screen.getByLabelText(/I understand\. Remember this on this computer/i),
    ).toBeChecked();

    await user.click(
      screen.getByLabelText(/I understand\. Remember this on this computer/i),
    );

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeDisabled(),
    );
    expect(revokeLinkedInWorkbenchReview).toHaveBeenCalledOnce();
    expect(window.localStorage.removeItem).not.toHaveBeenCalled();
  });

  it("keeps controls paused when provider policy evidence needs refresh", async () => {
    const user = userEvent.setup();
    vi.mocked(getLinkedInWorkbenchReviewStatus).mockResolvedValue(
      "policy_refresh_required",
    );
    renderWorkbench();

    expect(
      await screen.findByText(/paused while JobSentinel refreshes its LinkedIn policy review/i),
    ).toBeInTheDocument();
    const review = screen.getByLabelText(
      /I understand\. Remember this on this computer/i,
    );
    expect(review).not.toBeChecked();
    expect(review).toBeDisabled();
    expect(screen.getByRole("button", { name: /open linkedin jobs/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /log applied/i })).toBeDisabled();

    await user.click(review);
    expect(reviewLinkedInWorkbench).not.toHaveBeenCalled();
  });

  it("opens LinkedIn and shows a privacy reminder without a forced close", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await reviewWorkbench(user);
    await user.click(screen.getByRole("button", { name: /open linkedin jobs/i }));

    expect(openDeepLink).toHaveBeenCalledWith("https://www.linkedin.com/jobs/");
    expect(
      await screen.findByText(/JobSentinel will not force this session closed/i),
    ).toBeInTheDocument();
  });

  it("does not recommend policy-blocked LinkedIn page capture", () => {
    renderWorkbench();

    expect(
      screen.getByText(/browse normally, then add only the details you choose/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/LinkedIn does not permit third-party page capture/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /log follow-up/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /log rejected/i })).toBeInTheDocument();
    expect(screen.queryByText(/browser button/i)).not.toBeInTheDocument();
  });

  it("keeps local learning off until the user turns it on", () => {
    renderWorkbench();

    expect(
      screen.getByLabelText(/Help JobSentinel learn from my local job actions/i),
    ).not.toBeChecked();
    expect(screen.getByText(/Local learning is off/i)).toBeInTheDocument();
    expect(screen.queryByText(/Reviewable suggestions/i)).not.toBeInTheDocument();
  });

  it("uses pasted selected text as suggestions and logs applied with one click", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await reviewWorkbench(user);
    await user.type(
      screen.getByLabelText(/paste selected job text/i),
      "Principal Security Engineer at Example Co\nhttps://www.linkedin.com/jobs/view/123?token=secret\nli_at=raw-cookie",
    );
    await user.click(screen.getByRole("button", { name: /use pasted details/i }));
    await user.click(screen.getByRole("button", { name: /log applied/i }));

    await waitFor(() =>
      expect(recordLinkedInWorkbenchEvent).toHaveBeenCalledWith({
        eventType: "applied",
        title: "Principal Security Engineer",
        company: "Example Co",
        url: "https://www.linkedin.com/jobs/view/123",
        notes:
          "Principal Security Engineer at Example Co\nhttps://www.linkedin.com/jobs/view/123\nli_at=[REDACTED]",
      }),
    );
  });

  it("does not store learning signals while local learning is off", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await reviewWorkbench(user);
    await user.type(
      screen.getByRole("textbox", { name: "Job title" }),
      "Security Program Manager",
    );
    await user.type(screen.getByRole("textbox", { name: "Company" }), "Example Co");
    await user.click(screen.getByRole("button", { name: /save job/i }));

    await waitFor(() => expect(recordLinkedInWorkbenchEvent).toHaveBeenCalled());
    expect(window.localStorage.setItem).not.toHaveBeenCalledWith(
      BROWSER_ASSIST_LEARNING_STORAGE_KEY,
      expect.any(String),
    );
  });

  it("records reviewable suggestions after the user turns local learning on", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await user.click(
      screen.getByLabelText(/Help JobSentinel learn from my local job actions/i),
    );
    expect(window.localStorage.setItem).toHaveBeenCalledWith(
      BROWSER_ASSIST_LEARNING_ENABLED_STORAGE_KEY,
      "true",
    );
    await reviewWorkbench(user);
    await user.type(
      screen.getByRole("textbox", { name: "Job title" }),
      "Security Program Manager",
    );
    await user.type(screen.getByRole("textbox", { name: "Company" }), "Example Co");
    await user.click(screen.getByRole("button", { name: /save job/i }));

    await waitFor(() => {
      expect(window.localStorage.setItem).toHaveBeenCalledWith(
        BROWSER_ASSIST_LEARNING_STORAGE_KEY,
        expect.stringContaining("Security Program Manager"),
      );
      expect(screen.getByText(/Reviewable suggestions/i)).toBeInTheDocument();
      expect(screen.getByText("Security Program Manager")).toBeInTheDocument();
      expect(screen.getByText("Example Co")).toBeInTheDocument();
    });
    expect(
      screen.getByText(/does not read LinkedIn pages, browser storage, cookies/i),
    ).toBeInTheDocument();
  });

  it("lets the user clear learned local suggestions", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await user.click(
      screen.getByLabelText(/Help JobSentinel learn from my local job actions/i),
    );
    await reviewWorkbench(user);
    await user.type(
      screen.getByRole("textbox", { name: "Job title" }),
      "Content Strategist",
    );
    await user.click(screen.getByRole("button", { name: /save job/i }));

    await screen.findByText("Content Strategist");
    await user.click(screen.getByRole("button", { name: /clear learning/i }));

    expect(window.localStorage.removeItem).toHaveBeenCalledWith(
      BROWSER_ASSIST_LEARNING_STORAGE_KEY,
    );
    expect(screen.queryByText("Content Strategist")).not.toBeInTheDocument();
    expect(screen.getByText(/No local learning yet/i)).toBeInTheDocument();
  });

  it("fills suggestions immediately when the user pastes selected text", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await reviewWorkbench(user);

    fireEvent.paste(screen.getByLabelText(/paste selected job text/i), {
      clipboardData: {
        getData: () =>
          "Staff Security Engineer at Example Co\nhttps://www.linkedin.com/jobs/view/456?token=secret",
      },
    });

    expect(screen.getByRole("textbox", { name: "Job title" })).toHaveValue(
      "Staff Security Engineer",
    );
    expect(screen.getByRole("textbox", { name: "Company" })).toHaveValue(
      "Example Co",
    );
    expect(screen.getByRole("textbox", { name: "Job link" })).toHaveValue(
      "https://www.linkedin.com/jobs/view/456",
    );

    await user.click(screen.getByRole("button", { name: /save job/i }));

    await waitFor(() =>
      expect(recordLinkedInWorkbenchEvent).toHaveBeenCalledWith({
        eventType: "saved",
        title: "Staff Security Engineer",
        company: "Example Co",
        url: "https://www.linkedin.com/jobs/view/456",
        notes:
          "Staff Security Engineer at Example Co\nhttps://www.linkedin.com/jobs/view/456",
      }),
    );
  });

  it("records expanded local ledger actions from user clicks", async () => {
    const user = userEvent.setup();
    renderWorkbench();

    await reviewWorkbench(user);
    await user.type(
      screen.getByRole("textbox", { name: "Job title" }),
      "Content Strategist",
    );
    await user.type(screen.getByRole("textbox", { name: "Company" }), "Example Co");
    await user.click(screen.getByRole("button", { name: /log interview/i }));
    await user.click(screen.getByRole("button", { name: /log follow-up/i }));
    await user.click(screen.getByRole("button", { name: /add reminder/i }));
    await user.click(screen.getByRole("button", { name: /log rejected/i }));

    await waitFor(() => expect(recordLinkedInWorkbenchEvent).toHaveBeenCalledTimes(4));
    expect(recordLinkedInWorkbenchEvent).toHaveBeenNthCalledWith(1, {
      eventType: "interview",
      title: "Content Strategist",
      company: "Example Co",
      url: undefined,
      notes: undefined,
    });
    expect(recordLinkedInWorkbenchEvent).toHaveBeenNthCalledWith(2, {
      eventType: "follow_up",
      title: "Content Strategist",
      company: "Example Co",
      url: undefined,
      notes: undefined,
    });
    expect(recordLinkedInWorkbenchEvent).toHaveBeenNthCalledWith(3, {
      eventType: "reminder",
      title: "Content Strategist",
      company: "Example Co",
      url: undefined,
      notes: undefined,
    });
    expect(recordLinkedInWorkbenchEvent).toHaveBeenNthCalledWith(4, {
      eventType: "rejected",
      title: "Content Strategist",
      company: "Example Co",
      url: undefined,
      notes: undefined,
    });
  });
});
