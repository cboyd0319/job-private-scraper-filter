import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../app/providers/ToastProvider";
import { UndoProvider } from "../../app/providers/UndoProvider";
import ApplicationsPage from "./ApplicationsPage";
import type { ApplicationsByStatus } from "./applicationsModel";

const tauri = vi.hoisted(() => ({
  cachedInvoke: vi.fn(),
  invoke: vi.fn(),
  safeInvokeWithToast: vi.fn(),
}));

vi.mock("../../platform/tauri", () => ({
  cachedInvoke: tauri.cachedInvoke,
  invalidateCacheByCommand: vi.fn(),
  invoke: tauri.invoke,
  safeInvokeWithToast: tauri.safeInvokeWithToast,
}));

const applications: ApplicationsByStatus = {
  to_apply: [],
  applied: [
    {
      id: 30,
      job_hash: "office-assistant",
      job_title: "Office Assistant",
      company: "Example Services",
      status: "applied",
      applied_at: "2026-05-01T12:00:00Z",
      notes: "User-authored note",
      last_contact: null,
    },
  ],
  screening_call: [],
  phone_interview: [],
  technical_interview: [],
  onsite_interview: [],
  offer_received: [],
  offer_accepted: [],
  offer_rejected: [],
  rejected: [],
  withdrawn: [],
  ghosted: [],
};

function renderPage(
  overrides: Partial<ApplicationsByStatus> = {},
  props: Pick<
    React.ComponentProps<typeof ApplicationsPage>,
    "onBack" | "onOpenSalary" | "onOpenSources"
  > = {
    onBack: vi.fn(),
    onOpenSalary: vi.fn(),
    onOpenSources: vi.fn(),
  },
) {
  tauri.cachedInvoke.mockImplementation((command: string) =>
    Promise.resolve(
      command === "get_applications_kanban"
        ? { ...applications, ...overrides }
        : [],
    ),
  );

  render(
    <ToastProvider>
      <UndoProvider>
        <ApplicationsPage {...props} />
      </UndoProvider>
    </ToastProvider>,
  );
}

describe("ApplicationsPage daily mission", () => {
  beforeEach(() => {
    tauri.invoke.mockReset();
    tauri.safeInvokeWithToast.mockReset();
  });

  it("opens the selected quiet role without bulk-changing application status", async () => {
    renderPage();

    await screen.findByText("Daily mission");
    expect(
      screen.queryByRole("button", { name: "Review No Responses" }),
    ).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Review quiet roles" }));

    expect(await screen.findByTestId("application-detail-dialog")).toHaveTextContent(
      "Example Services",
    );
    expect(tauri.safeInvokeWithToast).not.toHaveBeenCalled();
  });

  it("opens the selected saved role in application tracking", async () => {
    renderPage({
      applied: [],
      to_apply: [
        {
          id: 31,
          job_hash: "saved-role",
          job_title: "Records Clerk",
          company: "Example County",
          status: "to_apply",
          applied_at: null,
          notes: null,
          last_contact: null,
        },
      ],
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Review this tracked role" }),
    );

    expect(await screen.findByTestId("application-detail-dialog")).toHaveTextContent(
      "Example County",
    );
    expect(tauri.safeInvokeWithToast).not.toHaveBeenCalled();
  });

  it("routes an offer action to offer and pay review", async () => {
    const onOpenSalary = vi.fn();
    renderPage(
      {
        applied: [],
        offer_received: [
          {
            id: 32,
            job_hash: "written-offer",
            job_title: "Support Specialist",
            company: "Example Health",
            status: "offer_received",
            applied_at: "2026-06-01T12:00:00Z",
            notes: null,
            last_contact: null,
          },
        ],
      },
      { onBack: vi.fn(), onOpenSalary, onOpenSources: vi.fn() },
    );

    fireEvent.click(
      await screen.findByRole("button", { name: "Review offer and pay" }),
    );

    expect(onOpenSalary).toHaveBeenCalledTimes(1);
    expect(tauri.safeInvokeWithToast).not.toHaveBeenCalled();
  });

  it("routes a source action to Sources and Alerts", async () => {
    const onOpenSources = vi.fn();
    renderPage(
      {},
      { onBack: vi.fn(), onOpenSalary: vi.fn(), onOpenSources },
    );

    fireEvent.click(
      await screen.findByRole("button", { name: "Review job sources" }),
    );

    expect(onOpenSources).toHaveBeenCalledTimes(1);
    expect(tauri.safeInvokeWithToast).not.toHaveBeenCalled();
  });
});
