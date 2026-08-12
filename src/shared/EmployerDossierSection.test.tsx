/** Proves employer-dossier IPC decoding rejects contradictory, malformed, and unsafe facts. */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EmployerDossierSection } from "./EmployerDossierSection";
import { decodeEmployerDossier } from "./employerDossier";

function dossier() {
  return {
    employer: { name: "Example", identity_status: "unverified_saved_name", official_domain: null, posting_domain: "job-boards.greenhouse.io" },
    role: { title: "Analyst", status: "last_observed", posting_url: "https://job-boards.greenhouse.io/example/jobs/1", first_observed_at: "2026-07-01T00:00:00Z", last_observed_at: "2026-07-20T00:00:00Z", times_seen: 2, repost_count: 1 },
    source: {
      source_id: "greenhouse", display_name: "Greenhouse", source_class: "public_ats", status: "current",
      documentation_url: "https://developers.greenhouse.io/job-board", observed_at: "2026-07-20T00:00:00Z", retrieved_at: null,
      verified_on: "2026-07-19", expires_on: "2026-08-18", jurisdiction: null, confidence_percent: 95,
      policy_ref: "jobsentinel.source-policy.greenhouse.public-job-board-api", policy_revision: 1,
      terms_review_ref: "source-review.greenhouse.public-get-api", robots_review_ref: "source-review.greenhouse.api-robots",
      parser_version: "greenhouse-job-board-api-v1", salary_coverage: "none", incomplete_coverage: false,
    },
    pay: { clarity: "range_listed", minimum: 100000, maximum: 140000, currency: "USD", observed_at: "2026-07-20T00:00:00Z" },
    local_history: { basis: "exact_saved_name", saved_job_count: 1, application_count: 0, interview_count: 0, offer_count: 0, terminal_outcome_count: 0 },
    application_channel: "public_ats",
    uncertainty: ["employer_identity_not_canonical", "official_domain_unknown", "role_not_live_checked", "retrieval_date_unavailable", "jurisdiction_unknown", "exact_name_history_only", "pay_provenance_incomplete"],
    next_action: "open_saved_posting",
  };
}

describe("decodeEmployerDossier", () => {
  it("accepts the bounded production-shaped dossier", () => {
    expect(decodeEmployerDossier(dossier())).not.toBeNull();
  });

  it("rejects an unreviewed public host even when it has a Greenhouse-shaped job id", () => {
    const value = dossier();
    value.employer.posting_domain = "evil.example";
    value.role.posting_url = "https://evil.example/careers/job?gh_jid=7687193003";

    expect(decodeEmployerDossier(value)).toBeNull();
  });

  it("preserves each distinct local-history count", () => {
    const value = dossier();
    value.local_history = { basis: "exact_saved_name", saved_job_count: 1, application_count: 2, interview_count: 3, offer_count: 4, terminal_outcome_count: 5 };

    expect(decodeEmployerDossier(value)?.local_history).toEqual(value.local_history);
  });

  it.each([
    "https://job-boards.greenhouse.io.evil.example/example/jobs/1",
    "https://user:secret@job-boards.greenhouse.io/example/jobs/1",
    "https://job-boards.greenhouse.io/example/jobs/1?token=secret",
    "https://job-boards.greenhouse.io/example/jobs/1#candidate-data",
    "https://job-boards.greenhouse.io:8443/example/jobs/1",
    "https://jobs.lever.co:8443/example/00000000-0000-4000-8000-000000000001",
  ])("rejects unsafe saved posting URL %s", (postingUrl) => {
    const value = dossier();
    value.role.posting_url = postingUrl;
    value.employer.posting_domain = new URL(postingUrl).hostname.toLowerCase();
    expect(decodeEmployerDossier(value)).toBeNull();
  });

  it("rejects impossible dates and pay shapes that contradict their label", () => {
    const impossibleDate = dossier();
    impossibleDate.source.verified_on = "2026-02-31";
    expect(decodeEmployerDossier(impossibleDate)).toBeNull();

    const missingMaximum = dossier();
    missingMaximum.pay.maximum = null as unknown as number;
    expect(decodeEmployerDossier(missingMaximum)).toBeNull();
  });

  it("rejects current, chronology, and next-action contradictions", () => {
    const currentWithoutProvenance = dossier();
    currentWithoutProvenance.source.policy_ref = null as unknown as string;
    expect(decodeEmployerDossier(currentWithoutProvenance)).toBeNull();

    const invertedObservation = dossier();
    invertedObservation.role.first_observed_at = "2026-07-21T00:00:00Z";
    expect(decodeEmployerDossier(invertedObservation)).toBeNull();

    const staleOpenAction = dossier();
    staleOpenAction.source.status = "stale";
    expect(decodeEmployerDossier(staleOpenAction)).toBeNull();
  });

  it("rejects omitted required uncertainty and unbounded counts", () => {
    const hiddenUncertainty = dossier();
    hiddenUncertainty.uncertainty = hiddenUncertainty.uncertainty.filter((item) => item !== "official_domain_unknown");
    expect(decodeEmployerDossier(hiddenUncertainty)).toBeNull();

    const hugeCount = dossier();
    hugeCount.role.times_seen = Number.MAX_SAFE_INTEGER;
    expect(decodeEmployerDossier(hugeCount)).toBeNull();
  });

  it("rejects provenance metadata attached to a missing source", () => {
    const value = dossier();
    value.employer.posting_domain = null as unknown as string;
    value.role.posting_url = null as unknown as string;
    value.source = {
      ...value.source,
      display_name: null as unknown as string,
      source_class: null as unknown as string,
      status: "missing",
      verified_on: null as unknown as string,
      expires_on: null as unknown as string,
      confidence_percent: null as unknown as number,
      policy_ref: null as unknown as string,
      policy_revision: null as unknown as number,
      terms_review_ref: null as unknown as string,
      robots_review_ref: null as unknown as string,
      parser_version: null as unknown as string,
      salary_coverage: null as unknown as string,
      incomplete_coverage: null as unknown as boolean,
    };
    value.application_channel = "unknown";
    value.uncertainty = [
      "employer_identity_not_canonical",
      "official_domain_unknown",
      "role_not_live_checked",
      "retrieval_date_unavailable",
      "jurisdiction_unknown",
      "exact_name_history_only",
      "source_missing",
      "pay_provenance_incomplete",
    ];
    value.next_action = "verify_on_employer_careers_page";

    expect(decodeEmployerDossier(value)).toBeNull();
  });

  it("rejects oversized uncertainty before iterating untrusted entries", () => {
    const value = dossier();
    const oversized = Array.from({ length: 17 }, () => "official_domain_unknown");
    Object.defineProperty(oversized, 16, {
      get: () => {
        throw new Error("oversized entry was inspected");
      },
    });
    value.uncertainty = oversized;

    expect(() => decodeEmployerDossier(value)).not.toThrow();
    expect(decodeEmployerDossier(value)).toBeNull();
  });

  it("rejects timestamps without an explicit RFC 3339 timezone", () => {
    const value = dossier();
    value.role.last_observed_at = "2026-07-20T00:00:00";
    value.source.observed_at = "2026-07-20T00:00:00";
    value.pay.observed_at = "2026-07-20T00:00:00";

    expect(decodeEmployerDossier(value)).toBeNull();
  });
});

describe("EmployerDossierSection", () => {
  it("shows evidence separation, review limits, link disclosure, and safe response guidance", () => {
    const decoded = decodeEmployerDossier(dossier());
    expect(decoded).not.toBeNull();
    render(<EmployerDossierSection dossier={decoded!} />);
    fireEvent.click(screen.getByText("Provenance and uncertainty"));

    expect(screen.getByText(/Source policy metadata does not prove that this role is still open/i)).toBeVisible();
    expect(screen.getByText(/Terms review: source-review\.greenhouse\.public-get-api/i)).toBeVisible();
    expect(screen.getByText(/Salary coverage: none/i)).toBeVisible();
    expect(screen.getByText(/Opening the saved posting contacts job-boards\.greenhouse\.io/i)).toBeVisible();
    expect(screen.getByText(/Do not pay fees or share sensitive information/i)).toBeVisible();
    expect(screen.getByText(/does not label the employer fraudulent/i)).toBeVisible();
  });

  it("shows dated posting and pay history", () => {
    const decoded = decodeEmployerDossier(dossier());
    expect(decoded).not.toBeNull();
    render(<EmployerDossierSection dossier={decoded!} />);

    expect(screen.getByText(/First observed.*Seen 2 times.*1 repost/i)).toBeVisible();
    expect(screen.getByText(/100,000.*140,000 USD.*Observed/i)).toBeVisible();
  });
});
