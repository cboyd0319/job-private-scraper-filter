// Implements browser-development Opportunity Case command projections.

import { getStringArg } from "../../mocks/handlers/commandHelpers";
import type { MockApplications, MockJob } from "../../mocks/handlers/types";

interface MockOpportunityCaseState {
  jobs: MockJob[];
  applications: MockApplications;
}

const reviewedSourceById = {
  greenhouse: {
    displayName: "Greenhouse",
    documentationUrl: "https://developers.greenhouse.io/job-board",
    policyRef: "jobsentinel.source-policy.greenhouse.public-job-board-api",
    termsReviewRef: "source-review.greenhouse.public-get-api",
    robotsReviewRef: "source-review.greenhouse.api-robots",
    parserVersion: "greenhouse-job-board-api-v1",
  },
  lever: {
    displayName: "Lever",
    documentationUrl: "https://github.com/lever/postings-api",
    policyRef: "jobsentinel.source-policy.lever.public-postings-api",
    termsReviewRef: "source-review.lever.public-postings-api",
    robotsReviewRef: "source-review.lever.api-robots",
    parserVersion: "lever-postings-api-v0",
  },
} as const;

interface MockOpportunityCaseResult {
  handled: boolean;
  shouldSave: boolean;
  state: MockOpportunityCaseState;
  value: unknown;
}

export function handleMockOpportunityCaseCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockOpportunityCaseState,
): MockOpportunityCaseResult {
  if (command !== "open_opportunity_case") {
    return { handled: false, shouldSave: false, state, value: undefined };
  }

  const jobHash = getStringArg(args, "jobHash");
  const job = state.jobs.find((candidate) => candidate.hash === jobHash);
  if (!job) throw new Error("Job is unavailable.");

  const application = Object.values(state.applications)
    .flat()
    .find((candidate) => candidate.job_hash === job.hash);
  const employerJobs = state.jobs.filter((candidate) => candidate.company === job.company);
  const employerApplications = Object.values(state.applications)
    .flat()
    .filter((candidate) => employerJobs.some((candidateJob) => candidateJob.hash === candidate.job_hash));
  const reviewedSource = reviewedSourceById[job.source as keyof typeof reviewedSourceById];
  const payListed = Boolean(job.salary_min && job.salary_max);

  return {
    handled: true,
    shouldSave: false,
    state,
    value: {
      job: {
        job_hash: job.hash,
        title: job.title,
        company: job.company,
        location: job.location,
        remote: job.remote,
        times_seen: 1,
      },
      source: {
        name: "Saved job source",
        last_seen_at: job.created_at,
        connectivity_required: true,
        stale: false,
      },
      posting_risk: { score: 0, reasons: [] },
      application: application && {
        status: application.status,
        has_contact: Boolean(application.last_contact),
      },
      interviews: null,
      offer: null,
      outcome: null,
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
          name: job.company,
          identity_status: "unverified_saved_name",
          official_domain: null,
          posting_domain: null,
        },
        role: {
          title: job.title,
          status: "last_observed",
          posting_url: null,
          first_observed_at: job.created_at,
          last_observed_at: job.created_at,
          times_seen: 1,
          repost_count: 0,
        },
        source: {
          source_id: job.source,
          display_name: reviewedSource?.displayName ?? null,
          source_class: reviewedSource ? "public_ats" : null,
          status: reviewedSource ? "current" : "missing",
          documentation_url: reviewedSource?.documentationUrl ?? null,
          observed_at: job.created_at,
          retrieved_at: null,
          verified_on: reviewedSource ? "2026-08-12" : null,
          expires_on: reviewedSource ? "2026-09-11" : null,
          jurisdiction: null,
          confidence_percent: reviewedSource ? 95 : null,
          policy_ref: reviewedSource?.policyRef ?? null,
          policy_revision: reviewedSource ? 1 : null,
          terms_review_ref: reviewedSource?.termsReviewRef ?? null,
          robots_review_ref: reviewedSource?.robotsReviewRef ?? null,
          parser_version: reviewedSource?.parserVersion ?? null,
          salary_coverage: reviewedSource ? "none" : null,
          incomplete_coverage: reviewedSource ? false : null,
        },
        pay: {
          clarity: payListed ? "range_listed" : "not_listed",
          minimum: job.salary_min ?? null,
          maximum: job.salary_max ?? null,
          currency: payListed ? "USD" : null,
          observed_at: job.created_at,
        },
        local_history: {
          basis: "exact_saved_name",
          saved_job_count: employerJobs.length,
          application_count: employerApplications.length,
          interview_count: 0,
          offer_count: employerApplications.filter((candidate) => candidate.status.startsWith("offer_")).length,
          terminal_outcome_count: employerApplications.filter((candidate) => ["offer_accepted", "offer_rejected", "rejected", "ghosted", "withdrawn"].includes(candidate.status)).length,
        },
        application_channel: reviewedSource ? "public_ats" : "unknown",
        uncertainty: [
          "employer_identity_not_canonical",
          "official_domain_unknown",
          "role_not_live_checked",
          "retrieval_date_unavailable",
          "jurisdiction_unknown",
          "exact_name_history_only",
          ...(reviewedSource ? [] : ["source_missing"]),
          payListed ? "pay_provenance_incomplete" : "pay_not_listed",
        ],
        next_action: "verify_on_employer_careers_page",
      },
      timeline: [],
    },
  };
}
