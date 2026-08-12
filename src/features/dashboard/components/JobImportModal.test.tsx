import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { ToastProvider } from "../../../app/providers/ToastProvider";
import { invalidateCacheByCommand, invoke } from "../../../platform/tauri";
import { JobImportModal } from "./JobImportModal";

vi.mock("../../../platform/tauri", () => ({
  invalidateCacheByCommand: vi.fn(),
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockInvalidateCacheByCommand = vi.mocked(invalidateCacheByCommand);

const renderModal = () =>
  render(
    <ToastProvider>
      <JobImportModal isOpen onClose={vi.fn()} />
    </ToastProvider>,
  );

describe("JobImportModal", () => {
  const privateFailure = new Error(
    "token=raw-secret private@example.test resume=private-file",
  );

  const preview = {
    import_id: "reviewed-import-1",
    title: "Office Manager",
    company: "Example Co",
    url: "https://example.com/jobs/office-manager",
    location: "Denver, CO",
    description_preview: "Coordinate office operations.",
    salary: "$60,000-$70,000",
    date_posted: "2026-05-01",
    valid_through: null,
    employment_types: ["Full-time"],
    remote: false,
    missing_fields: [],
    already_exists: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockReset();
  });

  it("uses plain job-link copy and broad role examples", () => {
    renderModal();

    expect(
      screen.getByRole("heading", { name: "Import Job from Link" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Paste a link to an individual job page/),
    ).toBeInTheDocument();
    expect(screen.getByText(/review before saving/i)).toBeInTheDocument();

    const input = screen.getByLabelText("Job link");
    expect(input).toHaveAttribute(
      "placeholder",
      "https://example.com/jobs/office-manager",
    );
    expect(screen.queryByText("Job URL")).not.toBeInTheDocument();
    expect(screen.queryByText(/any website/i)).not.toBeInTheDocument();
    expect(
      screen.queryByText(/automatically extract/i),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByPlaceholderText(/software-engineer/i),
    ).not.toBeInTheDocument();
  });

  it("opens Smart Paste from the existing import entry point", async () => {
    const user = userEvent.setup();
    renderModal();

    await user.click(
      screen.getByRole("button", { name: "Paste Job Details" }),
    );

    expect(screen.getByLabelText("Pasted job details")).toBeInTheDocument();
    expect(screen.queryByLabelText("Job link")).not.toBeInTheDocument();
  });

  it("guides users when the job link is missing", async () => {
    const user = userEvent.setup();
    renderModal();

    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Add a job link from your browser address bar.",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it("explains invalid links without technical URL wording", async () => {
    const user = userEvent.setup();
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "not a link" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Paste the full job link from your browser address bar.",
    );
    expect(invoke).not.toHaveBeenCalled();
  });

  it("does not show raw private details when preview fails", async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValueOnce(privateFailure);
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText("JobSentinel ran into a problem."),
    ).toBeInTheDocument();
    expect(screen.getByText(/safe support report/i)).toBeInTheDocument();
    expect(
      screen.queryByText(
        /raw-secret|private@example\.test|resume=private-file/,
      ),
    ).not.toBeInTheDocument();
  });

  it("shows the static policy recovery path for a blocked pasted link", async () => {
    const message =
      "JobSentinel cannot fetch this pasted link. Open it in your browser and use visible Browser Import or manual entry.";
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://www.dice.com/jobs/1" },
    });

    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(
      screen.queryByLabelText(
        /I understand this risk and want JobSentinel to check this job link/i,
      ),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Check Job Link" }),
    ).toBeDisabled();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    "http://localhost:4321/private-job",
    "http://127.0.0.1/private-job",
    "http://192.168.1.10/private-job",
    "file:///Users/example/private-job.html",
  ])("blocks private or local links before preview: %s", async (privateUrl) => {
    const user = userEvent.setup();
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: privateUrl },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText(
        "Paste a public job posting link from your browser address bar.",
      ),
    ).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
    expect(
      screen.queryByText("JobSentinel ran into a problem."),
    ).not.toBeInTheDocument();
  });

  it("blocks plaintext public job links before preview", async () => {
    const user = userEvent.setup();
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "http://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText(
        "Paste an https job posting link from your browser address bar.",
      ),
    ).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("requires acknowledgement before checking a restricted-source job link", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      url: "https://www.indeed.com/viewjob?jk=123456789",
    });
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://www.indeed.com/viewjob?jk=123456789" },
    });

    expect(
      screen.getByText(/rules about automated tools/i),
    ).toBeInTheDocument();
    const checkButton = screen.getByRole("button", { name: "Check Job Link" });
    expect(checkButton).toBeDisabled();
    expect(invoke).not.toHaveBeenCalled();

    await user.click(
      screen.getByLabelText(
        /I understand this risk and want JobSentinel to check this job link/i,
      ),
    );
    expect(checkButton).not.toBeDisabled();

    await user.click(checkButton);

    await screen.findByText("Office Manager");
    expect(mockInvoke).toHaveBeenCalledWith("preview_job_import", {
      url: "https://www.indeed.com/viewjob?jk=123456789",
    });
  });

  it("does not show raw private details when saving fails", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce(preview)
      .mockRejectedValueOnce(privateFailure);
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));
    expect(await screen.findByText("Office Manager")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save Job" }));

    expect(
      await screen.findByText("JobSentinel ran into a problem."),
    ).toBeInTheDocument();
    expect(screen.getByText(/safe support report/i)).toBeInTheDocument();
    expect(
      screen.queryByText(
        /raw-secret|private@example\.test|resume=private-file/,
      ),
    ).not.toBeInTheDocument();
  });

  it("asks for a fresh review when the staged preview expires", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce(preview)
      .mockRejectedValueOnce(
        "This job preview expired. Check the job link again before saving.",
      );
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));
    await user.click(await screen.findByRole("button", { name: "Save Job" }));

    expect(
      await screen.findAllByText(
        "This job preview expired. Check the job link again before saving.",
      ),
    ).toHaveLength(2);
  });

  it("does not offer to save a preview without required details", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      import_id: null,
      title: "",
      missing_fields: ["title"],
    });
    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/incomplete" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText(/Check a different job page before saving/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save Job" }),
    ).not.toBeInTheDocument();
  });

  it("lets users save jobs even when the preview is missing details", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        ...preview,
        missing_fields: ["salary", "date posted"],
      })
      .mockResolvedValueOnce({ jobId: 123 });

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText(/Details to check: pay range, posting date/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/You can still save this job/i),
    ).toBeInTheDocument();
    const saveButton = screen.getByRole("button", { name: "Save Job" });
    expect(saveButton).not.toBeDisabled();

    await user.click(saveButton);

    expect(mockInvoke).toHaveBeenLastCalledWith("confirm_job_import", {
      importId: "reviewed-import-1",
    });
    expect(mockInvalidateCacheByCommand).toHaveBeenNthCalledWith(
      1,
      "get_recent_jobs",
    );
    expect(mockInvalidateCacheByCommand).toHaveBeenNthCalledWith(
      2,
      "get_statistics",
    );
  });

  it("shows readable labels when imported missing fields use backend keys", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      missing_fields: ["salary_min", "salary_max", "company_name", "job_url"],
    });

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText(
        /Details to check: pay range, company name, job link/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/salary_min|salary_max|company_name|job_url/),
    ).not.toBeInTheDocument();
  });

  it("labels posting pay as listed pay in the preview", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce(preview);

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(await screen.findByText("Listed pay:")).toBeInTheDocument();
    expect(screen.getByText("$60,000-$70,000")).toBeInTheDocument();
    expect(screen.queryByText("Salary:")).not.toBeInTheDocument();
  });

  it("shows a listed-pay review cue when the preview has no pay", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      salary: null,
    });

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText("Listed pay not shown."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Verify pay before tailoring/i),
    ).toBeInTheDocument();
    expect(screen.queryByText("Salary:")).not.toBeInTheDocument();
  });

  it("shows the closing date when the posting provides one", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      valid_through: "2026-06-15T00:00:00Z",
    });

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(await screen.findByText("Posted: 5/1/2026")).toBeInTheDocument();
    expect(
      await screen.findByText("Closing date: 6/15/2026"),
    ).toBeInTheDocument();
  });

  it("shows plain unavailable copy for malformed preview dates", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValueOnce({
      ...preview,
      date_posted: "not a date",
      valid_through: "also not a date",
    });

    renderModal();

    fireEvent.change(screen.getByLabelText("Job link"), {
      target: { value: "https://example.com/jobs/office-manager" },
    });
    await user.click(screen.getByRole("button", { name: "Check Job Link" }));

    expect(
      await screen.findByText("Posted: Date not shown"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Closing date: Date not shown"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Invalid Date/i)).not.toBeInTheDocument();
  });
});
