/** Proves current opportunity-case review, preparation eligibility, and request identity. */

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OpportunityCaseAction } from "./OpportunityCaseAction";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./PackPacketBuilder", () => ({
  PackPacketBuilder: ({ jobHash }: { jobHash: string }) => <div data-testid="packet-builder">Packet Builder for {jobHash}</div>,
}));

const { invoke } = await import("@tauri-apps/api/core");
const mockInvoke = vi.mocked(invoke);

const caseFile = {
  job: {
    job_hash: "job-1",
    title: "Office Assistant",
    company: "CareBridge",
    location: "Denver, CO",
    remote: false,
    times_seen: 2,
  },
  source: {
    name: "Employer careers page",
    last_seen_at: "2026-07-21T12:00:00Z",
    connectivity_required: true,
    stale: true,
  },
  posting_risk: {
    score: 0.4,
    reasons: ["Verify the role on the employer site before tailoring."],
  },
  application: null,
  interviews: null,
  offer: null,
  outcome: null,
  evidence: {
    confirmed_count: 1,
    current_packet_count: 0,
    stale_packet_count: 1,
    review_status: "ready",
    requirements: [
      {
        requirement: "Scheduling",
        importance: "required",
        match_state: "direct",
        hard_constraint: false,
        blocking: false,
        why_not: null,
        evidence: [
          { kind: "resume_bullet", confirmed: true },
          { kind: "skill", confirmed: false },
        ],
      },
      {
        requirement: "Active license",
        importance: "required",
        match_state: "missing",
        hard_constraint: true,
        blocking: true,
        why_not: "missing_evidence",
        evidence: [],
      },
    ],
  },
  decision: {
    kind: "research_more",
    reasons: ["Verify this required qualification before deciding: Active license.", "The saved source snapshot may be stale."],
  },
  employer_dossier: {
    employer: {
      name: "CareBridge",
      identity_status: "unverified_saved_name",
      official_domain: null,
      posting_domain: "job-boards.greenhouse.io",
    },
    role: {
      title: "Office Assistant",
      status: "last_observed",
      posting_url: "https://job-boards.greenhouse.io/carebridge/jobs/1",
      first_observed_at: "2026-07-01T12:00:00Z",
      last_observed_at: "2026-07-21T12:00:00Z",
      times_seen: 2,
      repost_count: 1,
    },
    source: {
      source_id: "greenhouse",
      display_name: "Greenhouse",
      source_class: "public_ats",
      status: "current",
      documentation_url: "https://developers.greenhouse.io/job-board",
      observed_at: "2026-07-21T12:00:00Z",
      retrieved_at: null,
      verified_on: "2026-08-12",
      expires_on: "2026-09-11",
      jurisdiction: null,
      confidence_percent: 95,
      policy_ref: "jobsentinel.source-policy.greenhouse.public-job-board-api",
      policy_revision: 1,
      terms_review_ref: "source-review.greenhouse.public-get-api",
      robots_review_ref: "source-review.greenhouse.api-robots",
      parser_version: "greenhouse-job-board-api-v1",
      salary_coverage: "none",
      incomplete_coverage: false,
    },
    pay: {
      clarity: "range_listed",
      minimum: 100000,
      maximum: 140000,
      currency: "USD",
      observed_at: "2026-07-21T12:00:00Z",
    },
    local_history: {
      basis: "exact_saved_name",
      saved_job_count: 2,
      application_count: 1,
      interview_count: 1,
      offer_count: 0,
      terminal_outcome_count: 0,
    },
    application_channel: "public_ats",
    uncertainty: [
      "employer_identity_not_canonical",
      "official_domain_unknown",
      "role_not_live_checked",
      "retrieval_date_unavailable",
      "jurisdiction_unknown",
      "exact_name_history_only",
      "pay_provenance_incomplete",
    ],
    next_action: "open_saved_posting",
  },
  timeline: [
    { at: "2026-07-20T12:00:00Z", kind: "source_checked_failed" },
    { at: "2026-07-21T12:00:00Z", kind: "recovery_restored" },
  ],
};

describe("OpportunityCaseAction", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("opens a local case with only the job hash and makes review state visible", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(caseFile);
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("open_opportunity_case", {
        jobHash: "job-1",
      }),
    );
    expect(await screen.findByRole("heading", { name: "Office Assistant" })).toBeVisible();
    expect(screen.getByText("Seen 2 times. Review duplicate postings before preparing.")).toBeVisible();
    expect(screen.getByText("Posting risk: 40%.")).toBeVisible();
    expect(screen.getByText("Verify the role on the employer site before tailoring.")).toBeVisible();
    expect(screen.getByText("Case status")).toBeVisible();
    expect(screen.getByText("Case progress")).toBeVisible();
    expect(screen.getByText("No application activity yet.")).toBeVisible();
    expect(screen.getByText("Decision summary")).toBeVisible();
    expect(screen.getByText("Research more")).toBeVisible();
    expect(screen.getByText("Why not this job?")).toBeVisible();
    expect(screen.getByText("Evidence wall")).toBeVisible();
    expect(screen.getByRole("heading", { name: "Employer dossier" })).toBeVisible();
    expect(screen.getByText(/dated source evidence, not an employer rating/i)).toBeVisible();
    expect(screen.getAllByText(/No official employer domain is recorded/i)).toHaveLength(2);
    expect(screen.getAllByText(/Tue, Jul 21, 2026/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/100,000–140,000 USD.*Observed/i)).toBeVisible();
    expect(screen.getByText(/2 saved jobs, 1 application, 1 interview/i)).toBeVisible();
    expect(screen.getByText(/Current through Sep 11, 2026/i)).toBeVisible();
    const postingLink = screen.getByRole("link", { name: "Open saved posting" });
    expect(postingLink).toHaveAttribute(
      "href",
      "https://job-boards.greenhouse.io/carebridge/jobs/1",
    );
    expect(postingLink).toHaveAttribute("rel", "noopener noreferrer");
    expect(screen.getByText("Timeline")).toBeVisible();
    expect(screen.getByText(/The saved source snapshot may be stale/i)).toBeVisible();
    expect(screen.getByRole("heading", { name: "Scheduling" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Active license" })).toBeVisible();
    expect(screen.getByText("Visible evidence")).toBeVisible();
    expect(screen.getByText("Not found")).toBeVisible();
    expect(screen.getByText("Hard blocker")).toBeVisible();
    expect(screen.getByText("Resume bullet, confirmed")).toBeVisible();
    expect(screen.getByText("Skill, not confirmed")).toBeVisible();
    expect(screen.getByText("No supporting evidence available")).toBeVisible();
    expect(screen.getByText(/Why not: missing evidence/i)).toBeVisible();
    expect(screen.getByText(/Source refresh needs a connection/i)).toBeVisible();
    expect(screen.getByText(/Source may be stale/i)).toBeVisible();
    expect(screen.getByText(/Evidence needs review/i)).toBeVisible();
    expect(screen.getByText("Source check failed")).toBeVisible();
    expect(screen.getByText("Data restored")).toBeVisible();
    expect(screen.getByText("Source check failed").closest("li")?.querySelector("time")).toHaveAttribute("datetime", "2026-07-20T12:00:00Z");
    expect(screen.getAllByText(/Tue, Jul 21, 2026/i).length).toBeGreaterThan(0);
    expect(screen.queryByText("job-1")).not.toBeInTheDocument();
  });

  it("builds a local preparation workup without another command", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(caseFile);
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    await screen.findByRole("heading", { name: "Office Assistant" });
    await waitFor(() => expect(screen.getByRole("dialog").querySelector(".app-modal-panel")).toHaveFocus());
    const prepareButton = screen.getByRole("button", {
      name: "Prepare this job",
    });
    await user.click(prepareButton);

    expect(screen.getByRole("heading", { name: "Preparation workup" })).toBeVisible();
    expect(screen.getByText(/Local only.*nothing is sent or submitted/i)).toBeVisible();
    expect(screen.getByText("Source and role")).toBeVisible();
    expect(screen.getByText("Fit and evidence")).toBeVisible();
    expect(screen.getByText("Reviewed claims")).toBeVisible();
    expect(screen.getByText("Application materials")).toBeVisible();
    expect(screen.getByText("Screening answers")).toBeVisible();
    expect(screen.getByText("Final review")).toBeVisible();
    expect(screen.getByText(/refresh is separate and needs a connection/i)).toBeVisible();
    expect(screen.getByText(/Verify this required qualification before deciding/i)).toBeVisible();
    expect(screen.getByText(/1 reviewed claim needs confirmation/i)).toBeVisible();
    expect(screen.getByText(/employer's exact wording and current records for citizenship/i)).toBeVisible();
    expect(screen.getByText(/protected veteran status, disability, race\/ethnicity, gender, and other voluntary sensitive personal questions/i)).toBeVisible();
    expect(screen.getByText(/Use the employer's exact wording and current records for citizenship, work authorization, clearance, and eligibility/i)).toBeVisible();
    expect(screen.getByText(/Resolve the listed blockers before preparing or submitting/i)).toBeVisible();
    expect(screen.queryByText(/submit on the employer site yourself/i)).not.toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Back to case" })).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Back to case" }));
    expect(screen.queryByRole("heading", { name: "Preparation workup" })).not.toBeInTheDocument();
    expect(screen.getByText("Evidence wall")).toBeVisible();
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("keeps an empty recognized-requirement review unresolved", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue({
      ...caseFile,
      evidence: {
        ...caseFile.evidence,
        confirmed_count: 0,
        stale_packet_count: 0,
        requirements: [],
      },
      decision: {
        kind: "research_more",
        reasons: ["Review the posting before tailoring."],
      },
    });
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    await screen.findByRole("heading", { name: "Office Assistant" });
    await user.click(screen.getByRole("button", { name: "Prepare this job" }));
    expect(screen.getByTestId("packet-builder")).toHaveTextContent("job-1");

    expect(screen.getByText(/No recognized requirements are available/i)).toBeVisible();
    expect(screen.getByText(/Resolve the listed blockers before preparing or submitting/i)).toBeVisible();
  });

  it("keeps a safe error in the sheet and retries the same opaque request", async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValueOnce(new Error("network details"));
    mockInvoke.mockResolvedValueOnce(caseFile);
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    expect(await screen.findByText("Could not open this case.")).toBeVisible();
    expect(screen.queryByText("network details")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Retry" }));
    await screen.findByRole("heading", { name: "Office Assistant" });
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });

  it("keeps partial evidence and source state usable on a narrow screen", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue({
      ...caseFile,
      posting_risk: { score: null, reasons: [] },
      timeline: [],
      application: { status: "applied", has_contact: false },
      interviews: { upcoming_count: 1, completed_count: 0 },
      evidence: {
        confirmed_count: 0,
        current_packet_count: 0,
        stale_packet_count: 0,
        review_status: "no_saved_match",
        requirements: [],
      },
      decision: {
        kind: "research_more",
        reasons: ["No current saved-resume evidence review is available.", "The saved source snapshot may be stale."],
      },
    });
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));

    expect(await screen.findByText(/The saved source snapshot may be stale/i)).toBeVisible();
    expect(screen.getByText("Posting risk has not been scored.")).toBeVisible();
    expect(screen.getByText("Application: Applied. No contact recorded.")).toBeVisible();
    expect(screen.getByText("1 upcoming interview, 0 completed interviews.")).toBeVisible();
    expect(screen.getByText("0 confirmed, 0 current packets.")).toBeVisible();
    expect(screen.getByText("Compare this job with your active saved resume to build the evidence wall.")).toBeVisible();
    expect(screen.getByText("No case activity yet.")).toBeVisible();
    expect(screen.getByText("1 upcoming interview")).toBeVisible();
    expect(screen.getByText((_, element) => element?.tagName === "P" && Boolean(element.textContent?.includes("Source: Employer careers page. Last checked")))).toBeVisible();
    expect(screen.getByRole("dialog").querySelector(".min-w-0")).toHaveClass("min-w-0");
    expect(screen.getByRole("dialog").querySelector(".app-modal-panel")).toHaveClass("max-h-[calc(100dvh-2rem)]");

    await user.click(screen.getByRole("button", { name: "Prepare this job" }));
    expect(screen.getByText(/Compare this job with your active saved resume before tailoring/i)).toBeVisible();
  });

  it("shows changed evidence and accepted offers as non-apply states", async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({
        ...caseFile,
        evidence: {
          ...caseFile.evidence,
          review_status: "needs_refresh",
          requirements: [],
        },
        decision: {
          kind: "research_more",
          reasons: ["The saved-resume evidence review needs to be refreshed."],
        },
      })
      .mockResolvedValueOnce({
        ...caseFile,
        job: { ...caseFile.job, job_hash: "job-2" },
        outcome: { status: "offer_accepted" },
        decision: {
          kind: "skip",
          reasons: ["This opportunity closed with an accepted offer."],
        },
      });
    const { rerender } = render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    expect(await screen.findByText(/evidence review changed/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Prepare this job" }));
    expect(screen.getByText(/Refresh the active saved-resume evidence review/i)).toBeVisible();
    expect(screen.queryByTestId("packet-builder")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Close modal" }));
    rerender(<OpportunityCaseAction jobHash="job-2" />);
    await user.click(screen.getByRole("button", { name: "Open case" }));

    expect(await screen.findByText("Skip")).toBeVisible();
    expect(screen.getByText("This opportunity closed with an accepted offer.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Prepare this job" }));
    expect(screen.getByText(/Review the recorded outcome instead of preparing another submission/i)).toBeVisible();
    expect(screen.queryByText(/submit on the employer site yourself/i)).not.toBeInTheDocument();
    expect(screen.queryByTestId("packet-builder")).not.toBeInTheDocument();
  });

  it("does not render a stale case after the requested job changes", async () => {
    const user = userEvent.setup();
    let resolveA: (value: typeof caseFile) => void = () => undefined;
    let resolveB: (value: typeof caseFile) => void = () => undefined;
    mockInvoke
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveA = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveB = resolve;
          }),
      );
    const { rerender } = render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    rerender(<OpportunityCaseAction jobHash="job-2" />);
    await user.click(screen.getByRole("button", { name: "Open case" }));
    await act(async () =>
      resolveB({
        ...caseFile,
        job: { ...caseFile.job, job_hash: "job-2", title: "Current job" },
      }),
    );
    expect(await screen.findByRole("heading", { name: "Current job" })).toBeVisible();

    await act(async () => resolveA(caseFile));
    expect(screen.queryByRole("heading", { name: "Office Assistant" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Current job" })).toBeVisible();
  });

  it("shows a loading state before the local snapshot arrives", async () => {
    const user = userEvent.setup();
    let resolveCase: (value: typeof caseFile) => void;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCase = resolve;
        }),
    );
    render(<OpportunityCaseAction jobHash="job-1" />);

    await user.click(screen.getByRole("button", { name: "Open case" }));
    expect(screen.getByRole("status")).toHaveTextContent("Opening case");

    resolveCase!(caseFile);
    expect(await screen.findByRole("heading", { name: "Office Assistant" })).toBeVisible();
  });
});
