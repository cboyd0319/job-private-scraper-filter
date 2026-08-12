/** Proves Company Research reads only the selected job's local opportunity-case dossier. */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { CompanyResearchPanel } from "./CompanyResearchPanel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

function caseFile(jobHash: string, company = "CareBridge Services") {
  return {
    job: { job_hash: jobHash },
    employer_dossier: {
      employer: { name: company, identity_status: "unverified_saved_name", official_domain: null, posting_domain: "job-boards.greenhouse.io" },
      role: { title: "Coordinator", status: "last_observed", posting_url: "https://job-boards.greenhouse.io/carebridge/jobs/1", first_observed_at: "2026-08-01T00:00:00Z", last_observed_at: "2026-08-02T00:00:00Z", times_seen: 1, repost_count: 0 },
      source: { source_id: "greenhouse", display_name: "Greenhouse", source_class: "public_ats", status: "current", documentation_url: "https://developers.greenhouse.io/job-board", observed_at: "2026-08-02T00:00:00Z", retrieved_at: null, verified_on: "2026-08-01", expires_on: "2026-08-31", jurisdiction: null, confidence_percent: 95, policy_ref: "policy", policy_revision: 1, terms_review_ref: "terms", robots_review_ref: "robots", parser_version: "v1", salary_coverage: "none", incomplete_coverage: false },
      pay: { clarity: "not_listed", minimum: null, maximum: null, currency: null, observed_at: "2026-08-02T00:00:00Z" },
      local_history: { basis: "exact_saved_name", saved_job_count: 1, application_count: 0, interview_count: 0, offer_count: 0, terminal_outcome_count: 0 },
      application_channel: "public_ats",
      uncertainty: ["employer_identity_not_canonical", "official_domain_unknown", "role_not_live_checked", "retrieval_date_unavailable", "jurisdiction_unknown", "exact_name_history_only", "pay_not_listed"],
      next_action: "open_saved_posting",
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => { resolve = next; });
  return { promise, resolve };
}

describe("CompanyResearchPanel", () => {
  beforeEach(() => vi.clearAllMocks());

  it("removes the obsolete static company cache", () => {
    window.localStorage.setItem("jobsentinel_company_cache", "private legacy data");

    render(<CompanyResearchPanel companyName="CareBridge Services" />);

    expect(window.localStorage.getItem("jobsentinel_company_cache")).toBeNull();
  });

  it("requires a saved job and makes no local projection request without its hash", () => {
    render(<CompanyResearchPanel companyName="CareBridge Services" />);

    expect(screen.getByText("Choose a saved job to view its local employer dossier.")).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("loads the selected job's local dossier without a network request", async () => {
    mockInvoke.mockResolvedValue(caseFile("job-carebridge"));
    const fetchSpy = vi.spyOn(globalThis, "fetch");

    render(<CompanyResearchPanel companyName="CareBridge Services" jobHash="job-carebridge" />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading local employer dossier…");
    await waitFor(() => expect(screen.getByRole("heading", { name: "Employer dossier" })).toBeInTheDocument());
    expect(mockInvoke).toHaveBeenCalledWith("open_opportunity_case", { jobHash: "job-carebridge" });
    expect(fetchSpy).not.toHaveBeenCalled();
    fetchSpy.mockRestore();
  });

  it("shows a recoverable error when the local dossier projection is unavailable", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("storage unavailable")).mockResolvedValueOnce(caseFile("job-carebridge"));
    render(<CompanyResearchPanel companyName="CareBridge Services" jobHash="job-carebridge" />);

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Could not load the local employer dossier."));
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Employer dossier" })).toBeInTheDocument());
  });

  it("does not replace a newly selected dossier with a late older response", async () => {
    const first = deferred<ReturnType<typeof caseFile>>();
    const second = deferred<ReturnType<typeof caseFile>>();
    mockInvoke.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const { rerender } = render(<CompanyResearchPanel companyName="First" jobHash="job-first" />);

    rerender(<CompanyResearchPanel companyName="Second" jobHash="job-second" />);
    second.resolve(caseFile("job-second", "Second"));
    await waitFor(() => expect(screen.getByText("Second. No official employer domain is recorded.")).toBeInTheDocument());
    first.resolve(caseFile("job-first", "First"));
    await waitFor(() => expect(screen.queryByText("First. No official employer domain is recorded.")).not.toBeInTheDocument());
  });
});
