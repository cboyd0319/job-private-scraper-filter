/** Proves native dropped files remain inert until the matching explicit review action. */

import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invalidateCacheByCommand, invoke } from "../platform/tauri";
import { listen } from "../platform/tauri/events";
import { NativeFileDropReview } from "./NativeFileDropReview";

vi.mock("../platform/tauri", () => ({
  invalidateCacheByCommand: vi.fn(),
  invoke: vi.fn(),
}));

let receiveDrop:
  | ((event: {
      payload: {
        dropId: string | null;
        name: string | null;
        error: string | null;
      };
    }) => void)
  | undefined;

vi.mock("../platform/tauri/events", () => ({
  listen: vi.fn((_event, handler) => {
    receiveDrop = handler;
    return Promise.resolve(() => {});
  }),
}));

const mockInvoke = vi.mocked(invoke);
const mockInvalidateCacheByCommand = vi.mocked(invalidateCacheByCommand);
const preview = {
  import_id: null,
  title: "Care Coordinator",
  company: "Example Health",
  url: "https://jobs.example.com/care-coordinator",
  location: null,
  description_preview: "Coordinate local care services.",
  salary: null,
  date_posted: null,
  valid_through: null,
  employment_types: [],
  remote: false,
  missing_fields: [],
  already_exists: false,
};

async function emit(payload: {
  dropId: string | null;
  name: string | null;
  error: string | null;
}) {
  await waitFor(() => expect(receiveDrop).toBeDefined());
  await act(async () => receiveDrop?.({ payload }));
}

async function openDrop(name = "resume.pdf") {
  render(<NativeFileDropReview />);
  await emit({ dropId: "drop-1", name, error: null });
}

describe("NativeFileDropReview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    receiveDrop = undefined;
  });

  it("rejects invalid events without invoking an import command", async () => {
    render(<NativeFileDropReview />);
    await emit({ dropId: null, name: null, error: "unsafe event" });

    expect(screen.getByRole("alert")).toHaveTextContent(
      /drop one regular file/i,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(
      /nothing was imported/i,
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("shows only the sanitized name and explicit local review choices", async () => {
    await openDrop("/private/path/resume.pdf");

    expect(
      screen.getByRole("dialog", { name: /review dropped file/i }),
    ).toHaveTextContent("resume.pdf");
    expect(
      screen.queryByText("/private/path/resume.pdf"),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText(/stays local/i)).not.toHaveLength(0);
    expect(
      screen.getByRole("button", { name: "Add resume" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Review job posting" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Backup/Recovery" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/add resume, import job, or backup\/recovery/i),
    ).toBeInTheDocument();
  });

  it("discards the token when cancelled", async () => {
    const user = userEvent.setup();
    await openDrop();

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(mockInvoke).toHaveBeenCalledWith("discard_native_file_drop", {
      dropId: "drop-1",
    });
  });

  it("imports a resume only after its explicit action", async () => {
    const user = userEvent.setup();
    await openDrop();

    await user.click(screen.getByRole("button", { name: "Add resume" }));

    expect(mockInvoke).toHaveBeenCalledWith("import_dropped_resume", {
      dropId: "drop-1",
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      /review it on the resumes page/i,
    );
  });

  it("stages a signed pack for separate review without activating it", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      publisherKeyId: "jobsentinel-source-publisher-v1",
      packId: "jobsentinel.sources.us",
      state: "needs_review",
      generation: 1,
      releaseSequence: 1,
    });
    await openDrop("source-release.jspack");

    await user.click(
      screen.getByRole("button", { name: "Verify and stage signed pack" }),
    );

    expect(mockInvoke).toHaveBeenCalledWith("stage_dropped_pack", {
      dropId: "drop-1",
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      /staged locally for review in Settings > Packs/i,
    );
    expect(screen.queryByText(/activated/i)).not.toBeInTheDocument();
  });

  it.each([
    ["ready", "is already active"],
    ["removed", "was previously removed"],
  ])(
    "reports a replayed %s pack without claiming it was newly staged",
    async (state, copy) => {
      const user = userEvent.setup();
      mockInvoke.mockResolvedValueOnce({
        publisherKeyId: "jobsentinel-source-publisher-v1",
        packId: "jobsentinel.sources.us",
        state,
        generation: 4,
        releaseSequence: 1,
      });
      await openDrop("source-release.jspack");

      await user.click(
        screen.getByRole("button", { name: "Verify and stage signed pack" }),
      );

      expect(await screen.findByRole("status")).toHaveTextContent(copy);
      expect(screen.getByRole("status")).not.toHaveTextContent(
        "staged locally for review",
      );
    },
  );

  it("keeps a newer drop visible when an earlier import finishes", async () => {
    const user = userEvent.setup();
    let finishImport: ((value: number) => void) | undefined;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishImport = resolve;
        }),
    );
    await openDrop("first-resume.pdf");

    await user.click(screen.getByRole("button", { name: "Add resume" }));
    await emit({ dropId: "drop-2", name: "second-resume.pdf", error: null });
    await act(async () => finishImport?.(42));

    expect(
      screen.getByRole("dialog", { name: /review dropped file/i }),
    ).toHaveTextContent("second-resume.pdf");
    expect(screen.getByRole("status")).toHaveTextContent("first-resume.pdf");
    expect(screen.getByRole("status")).not.toHaveTextContent(
      "second-resume.pdf",
    );
  });

  it("previews a dropped job without saving, then restages explicit edits", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(preview).mockResolvedValueOnce({
      ...preview,
      import_id: "draft-1",
      url: "https://jobs.example.com/care-coordinator",
      missing_fields: [],
    });
    await openDrop();

    await user.click(
      screen.getByRole("button", { name: "Review job posting" }),
    );
    expect(mockInvoke).toHaveBeenCalledWith("preview_dropped_job", {
      dropId: "drop-1",
    });
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "confirm_smart_paste",
      expect.anything(),
    );
    expect(
      screen.queryByRole("button", { name: "Save Job" }),
    ).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Job title"), {
      target: { value: "Care Coordinator II" },
    });
    await user.click(screen.getByRole("button", { name: "Review Draft" }));

    expect(mockInvoke).toHaveBeenLastCalledWith("preview_dropped_job", {
      dropId: "drop-1",
      title: "Care Coordinator II",
      company: "Example Health",
      jobUrl: "https://jobs.example.com/care-coordinator",
      location: null,
    });
    expect(
      await screen.findByRole("button", { name: "Save Job" }),
    ).toBeEnabled();
  });

  it("does not attach an earlier preview to a newer dropped file", async () => {
    const user = userEvent.setup();
    let finishPreview: ((value: typeof preview) => void) | undefined;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishPreview = resolve;
        }),
    );
    await openDrop("first-job.txt");

    await user.click(
      screen.getByRole("button", { name: "Review job posting" }),
    );
    await emit({ dropId: "drop-2", name: "second-job.txt", error: null });
    await act(async () => finishPreview?.(preview));

    expect(
      screen.getByRole("dialog", { name: /review dropped file/i }),
    ).toHaveTextContent("second-job.txt");
    expect(
      screen.getByRole("button", { name: "Review job posting" }),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("Job title")).not.toBeInTheDocument();
  });

  it("does not offer Save Job for a duplicate preview", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      import_id: "duplicate",
      already_exists: true,
    });
    await openDrop();

    await user.click(
      screen.getByRole("button", { name: "Review job posting" }),
    );

    expect(
      await screen.findByText(/already in your saved jobs/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save Job" }),
    ).not.toBeInTheDocument();
  });

  it("keeps a successful save accurate when temporary drop cleanup is stale", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({ ...preview, import_id: "draft-1" })
      .mockResolvedValueOnce({ jobId: 42 })
      .mockRejectedValueOnce(new Error("stale drop"));
    await openDrop("care-coordinator.txt");

    await user.click(
      screen.getByRole("button", { name: "Review job posting" }),
    );
    await user.click(await screen.findByRole("button", { name: "Save Job" }));

    expect(mockInvoke).toHaveBeenNthCalledWith(2, "confirm_smart_paste", {
      importId: "draft-1",
    });
    expect(mockInvalidateCacheByCommand).toHaveBeenCalledWith(
      "get_recent_jobs",
    );
    expect(mockInvalidateCacheByCommand).toHaveBeenCalledWith("get_statistics");
    expect(await screen.findByRole("status")).toHaveTextContent(
      "care-coordinator.txt was saved locally as a job",
    );
    expect(
      screen.queryByText(/nothing new was saved/i),
    ).not.toBeInTheDocument();
  });

  it("requires a 16-character passphrase and stages a backup only on click", async () => {
    const user = userEvent.setup();
    await openDrop();

    await user.click(screen.getByRole("button", { name: "Backup/Recovery" }));
    const stage = screen.getByRole("button", { name: "Stage backup" });
    expect(stage).toBeDisabled();
    const passphrase = screen.getByLabelText("Backup passphrase");
    fireEvent.change(passphrase, { target: { value: "😀".repeat(15) } });
    expect(stage).toBeDisabled();
    fireEvent.change(passphrase, { target: { value: "😀".repeat(16) } });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(stage).toBeEnabled();

    await user.click(stage);

    expect(mockInvoke).toHaveBeenCalledWith("stage_dropped_portable_restore", {
      dropId: "drop-1",
      passphrase: "😀".repeat(16),
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      /restart is required/i,
    );
  });

  it("clears a backup passphrase when leaving its review step", async () => {
    const user = userEvent.setup();
    await openDrop();
    await user.click(screen.getByRole("button", { name: "Backup/Recovery" }));
    fireEvent.change(screen.getByLabelText("Backup passphrase"), {
      target: { value: "local-backup-passphrase" },
    });

    await user.click(
      screen.getByRole("button", { name: "Choose another option" }),
    );
    await user.click(screen.getByRole("button", { name: "Backup/Recovery" }));

    expect(screen.getByLabelText("Backup passphrase")).toHaveValue("");
    expect(screen.getByRole("button", { name: "Stage backup" })).toBeDisabled();
  });

  it("uses narrow-safe action wrapping and an accessible close control", async () => {
    await openDrop();

    expect(
      screen.getByRole("button", { name: "Close file drop review" }),
    ).toBeInTheDocument();
    expect(screen.getByTestId("native-file-drop-actions")).toHaveClass(
      "flex-col",
    );
    expect(listen).toHaveBeenCalledWith(
      "native-file-drop",
      expect.any(Function),
    );
    const panel = document.querySelector<HTMLElement>(".app-modal-panel");
    await waitFor(() => expect(document.activeElement).toBe(panel));
  });
});
