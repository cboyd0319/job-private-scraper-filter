// Verifies browser-development Opportunity Case command projections.

import { describe, expect, it } from "vitest";
import { mockApplications, mockJobs } from "../../mocks/data";
import { handleMockOpportunityCaseCommand } from "./commands";

describe("Opportunity case mock command", () => {
  it("returns one local-safe case snapshot for the requested job hash", () => {
    const result = handleMockOpportunityCaseCommand(
      "open_opportunity_case",
      { jobHash: mockJobs[0].hash },
      { jobs: mockJobs, applications: mockApplications },
    );

    expect(result).toMatchObject({ handled: true, shouldSave: false });
    expect(result.value).toMatchObject({
      job: { job_hash: mockJobs[0].hash, title: mockJobs[0].title },
      source: { connectivity_required: true, stale: false },
      evidence: {
        confirmed_count: 0,
        current_packet_count: 0,
        stale_packet_count: 0,
        review_status: "no_saved_match",
        requirements: [],
      },
      decision: {
        kind: "research_more",
        reasons: ["No current saved-resume evidence review is available."],
      },
      employer_dossier: {
        employer: {
          name: mockJobs[0].company,
          identity_status: "unverified_saved_name",
          official_domain: null,
        },
        source: { source_id: mockJobs[0].source, source_class: "public_ats" },
        pay: { clarity: "range_listed", currency: "USD" },
        local_history: { basis: "exact_saved_name", saved_job_count: 1 },
        application_channel: "public_ats",
        next_action: "verify_on_employer_careers_page",
      },
      timeline: [],
    });
    expect(result.value).not.toHaveProperty("id");
  });

  it("uses the exact reviewed Lever provenance for Lever jobs", () => {
    const result = handleMockOpportunityCaseCommand(
      "open_opportunity_case",
      { jobHash: mockJobs[2].hash },
      { jobs: mockJobs, applications: mockApplications },
    );

    expect(result).toMatchObject({
      handled: true,
      value: {
        employer_dossier: {
          source: {
            source_id: "lever",
            status: "current",
            policy_ref: "jobsentinel.source-policy.lever.public-postings-api",
            terms_review_ref: "source-review.lever.public-postings-api",
            parser_version: "lever-postings-api-v0",
          },
        },
      },
    });
  });

  it("does not invent reviewed ATS provenance for unsupported saved sources", () => {
    const result = handleMockOpportunityCaseCommand(
      "open_opportunity_case",
      { jobHash: mockJobs[4].hash },
      { jobs: mockJobs, applications: mockApplications },
    );

    expect(result).toMatchObject({
      handled: true,
      value: {
        employer_dossier: {
          source: {
            source_id: "direct",
            source_class: null,
            status: "missing",
            verified_on: null,
            policy_ref: null,
          },
          application_channel: "unknown",
          next_action: "verify_on_employer_careers_page",
        },
      },
    });
  });

  it("uses the no-pay uncertainty when a saved job has no range", () => {
    const noPayJob = {
      ...mockJobs[0],
      hash: "job-without-pay",
      salary_min: null,
      salary_max: null,
    } as unknown as (typeof mockJobs)[number];
    const result = handleMockOpportunityCaseCommand(
      "open_opportunity_case",
      { jobHash: noPayJob.hash },
      { jobs: [noPayJob], applications: mockApplications },
    );

    expect(result).toMatchObject({
      handled: true,
      value: {
        employer_dossier: {
          pay: { clarity: "not_listed", minimum: null, maximum: null, currency: null },
          uncertainty: expect.arrayContaining(["pay_not_listed"]),
        },
      },
    });
    expect(result.value).not.toMatchObject({
      employer_dossier: { uncertainty: expect.arrayContaining(["pay_provenance_incomplete"]) },
    });
  });

  it("does not claim commands owned by another feature", () => {
    const state = { jobs: mockJobs, applications: mockApplications };

    expect(handleMockOpportunityCaseCommand("get_jobs", undefined, state)).toMatchObject({
      handled: false,
      shouldSave: false,
      state,
    });
  });
});
