import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../../app/providers/ToastProvider";
import { invalidateCacheByCommand, invoke } from "../../../platform/tauri";
import { SmartPasteFlow } from "./SmartPasteFlow";

vi.mock("../../../platform/tauri", () => ({
  invalidateCacheByCommand: vi.fn(),
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockInvalidateCacheByCommand = vi.mocked(invalidateCacheByCommand);

const text =
  "Office Manager\nExample Services\nCoordinate office operations.";
const incompleteDraft = {
  import_id: null,
  title: "Office Manager",
  company: "Example Services",
  url: "",
  location: null,
  description_preview: "Coordinate office operations.",
  salary: null,
  date_posted: null,
  valid_through: null,
  employment_types: [],
  remote: false,
  missing_fields: ["url"],
  already_exists: false,
};

const renderFlow = () =>
  render(
    <ToastProvider>
      <SmartPasteFlow
        onClose={vi.fn()}
        onUseLink={vi.fn()}
        onImportSuccess={vi.fn()}
      />
    </ToastProvider>,
  );

describe("SmartPasteFlow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("turns copied text into an editable local draft and restages exact edits", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce(incompleteDraft)
      .mockResolvedValueOnce({
        ...incompleteDraft,
        import_id: "smart-paste-1",
        url: "https://example.com/jobs/office-manager",
        missing_fields: [],
      })
      .mockResolvedValueOnce({ jobId: 42 });
    renderFlow();

    fireEvent.change(screen.getByLabelText("Pasted job details"), {
      target: { value: text },
    });
    await user.click(screen.getByRole("button", { name: "Create Draft" }));

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "preview_smart_paste", {
      text,
    });
    expect(screen.getByLabelText("Job title")).toHaveValue("Office Manager");
    expect(screen.getByLabelText("Company")).toHaveValue("Example Services");
    expect(screen.getByLabelText("Job link")).toHaveValue("");
    expect(
      screen.queryByRole("button", { name: "Save Job" }),
    ).not.toBeInTheDocument();

    await user.type(
      screen.getByLabelText("Job link"),
      "https://example.com/jobs/office-manager",
    );
    await user.click(screen.getByRole("button", { name: "Review Draft" }));

    expect(mockInvoke).toHaveBeenNthCalledWith(2, "preview_smart_paste", {
      text,
      title: "Office Manager",
      company: "Example Services",
      jobUrl: "https://example.com/jobs/office-manager",
      location: null,
    });
    await user.click(screen.getByRole("button", { name: "Save Job" }));

    expect(mockInvoke).toHaveBeenNthCalledWith(3, "confirm_smart_paste", {
      importId: "smart-paste-1",
    });
    expect(mockInvalidateCacheByCommand).toHaveBeenCalledWith(
      "get_recent_jobs",
    );
    expect(mockInvalidateCacheByCommand).toHaveBeenCalledWith("get_statistics");
  });

  it("keeps screenshots outside Smart Paste and offers the exact link path", async () => {
    const user = userEvent.setup();
    const onUseLink = vi.fn();
    render(
      <ToastProvider>
        <SmartPasteFlow
          onClose={vi.fn()}
          onUseLink={onUseLink}
          onImportSuccess={vi.fn()}
        />
      </ToastProvider>,
    );

    expect(
      screen.getByText(/Smart Paste reads copied text only/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/does not read screenshots/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Use a Job Link" }));
    expect(onUseLink).toHaveBeenCalledOnce();
  });
});
