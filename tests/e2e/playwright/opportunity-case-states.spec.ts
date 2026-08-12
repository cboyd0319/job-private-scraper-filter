// Exercises Opportunity Case states at desktop and narrow viewport widths.

import { expect, test, type Page } from "@playwright/test";
import { DashboardPage } from "./page-objects/DashboardPage";

const MOCK_INVOKE_CONTROLS_KEY = "jobsentinel.mockInvokeControls.v1";
const MOCK_STATE_KEY = "jobsentinel.mockState.v1";

const baseDossier = {
  employer: { name: "Shopify", identity_status: "unverified_saved_name", official_domain: null, posting_domain: null },
  role: { title: "SEO Manager", status: "last_observed", posting_url: null, first_observed_at: "2026-07-01T12:00:00Z", last_observed_at: "2026-07-21T12:00:00Z", times_seen: 1, repost_count: 0 },
  source: {
    source_id: "greenhouse", display_name: "Greenhouse", source_class: "public_ats", status: "current",
    documentation_url: "https://developers.greenhouse.io/job-board", observed_at: "2026-07-21T12:00:00Z", retrieved_at: null,
    verified_on: "2026-08-12", expires_on: "2026-09-11", jurisdiction: null, confidence_percent: 95,
    policy_ref: "jobsentinel.source-policy.greenhouse.public-job-board-api", policy_revision: 1,
    terms_review_ref: "source-review.greenhouse.public-get-api", robots_review_ref: "source-review.greenhouse.api-robots",
    parser_version: "greenhouse-job-board-api-v1", salary_coverage: "none", incomplete_coverage: false,
  },
  pay: { clarity: "range_listed", minimum: 95000, maximum: 130000, currency: "USD", observed_at: "2026-07-21T12:00:00Z" },
  local_history: { basis: "exact_saved_name", saved_job_count: 1, application_count: 0, interview_count: 0, offer_count: 0, terminal_outcome_count: 0 },
  application_channel: "public_ats",
  uncertainty: ["employer_identity_not_canonical", "official_domain_unknown", "role_not_live_checked", "retrieval_date_unavailable", "jurisdiction_unknown", "exact_name_history_only", "pay_provenance_incomplete"],
  next_action: "verify_on_employer_careers_page",
};

const baseCase = {
  job: {
    job_hash: "job-hash-1",
    title: "SEO Manager",
    company: "Shopify",
    location: "Remote",
    remote: true,
    times_seen: 1,
  },
  source: {
    name: "Saved job source",
    last_seen_at: "2026-07-21T12:00:00Z",
    connectivity_required: true,
    stale: false,
  },
  posting_risk: { score: 0, reasons: [] },
  application: null,
  interviews: null,
  offer: null,
  outcome: null,
  evidence: {
    confirmed_count: 0,
    current_packet_count: 0,
    stale_packet_count: 0,
    review_status: "ready",
    requirements: [],
  },
  decision: {
    kind: "research_more",
    reasons: ["Review the posting before tailoring."],
  },
  employer_dossier: baseDossier,
  timeline: [],
};

const cases = [
  {
    name: "empty",
    caseFile: baseCase,
    expected: "The posting does not contain enough recognized requirements for an evidence review.",
  },
  {
    name: "partial",
    caseFile: {
      ...baseCase,
      application: { status: "applied", has_contact: false },
      interviews: { upcoming_count: 1, completed_count: 2 },
      evidence: {
        ...baseCase.evidence,
        review_status: "ready",
        requirements: [
          {
            requirement: "Calendar coordination",
            importance: "required",
            match_state: "partial",
            hard_constraint: false,
            blocking: false,
            why_not: "partial_evidence",
            evidence: [],
          },
        ],
      },
    },
    expected: "Needs support",
  },
  {
    name: "duplicate",
    caseFile: { ...baseCase, job: { ...baseCase.job, times_seen: 2 } },
    expected: "Seen 2 times. Review duplicate postings before preparing.",
  },
  {
    name: "offline",
    caseFile: {
      ...baseCase,
      source: { ...baseCase.source, connectivity_required: false },
    },
    expected: "Review remains available offline.",
  },
  {
    name: "failed source",
    caseFile: {
      ...baseCase,
      source: { ...baseCase.source, stale: true },
      employer_dossier: {
        ...baseDossier,
        source: { ...baseDossier.source, status: "stale" },
        uncertainty: [...baseDossier.uncertainty, "source_not_current"],
      },
      timeline: [{ at: "2026-07-20T12:00:00Z", kind: "source_checked_failed" }],
    },
    expected: "Source check failed",
  },
  {
    name: "missing employer source",
    caseFile: {
      ...baseCase,
      employer_dossier: {
        ...baseDossier,
        source: {
          source_id: "unreviewed-source", display_name: null, source_class: null, status: "missing",
          documentation_url: null, observed_at: "2026-07-21T12:00:00Z", retrieved_at: null,
          verified_on: null, expires_on: null, jurisdiction: null, confidence_percent: null,
          policy_ref: null, policy_revision: null, terms_review_ref: null, robots_review_ref: null,
          parser_version: null, salary_coverage: null, incomplete_coverage: null,
        },
        application_channel: "unknown",
        uncertainty: [...baseDossier.uncertainty, "source_missing"],
      },
    },
    expected: "No reviewed source record",
  },
  {
    name: "restored data",
    caseFile: {
      ...baseCase,
      timeline: [{ at: "2026-07-21T12:00:00Z", kind: "recovery_restored" }],
    },
    expected: "Data restored",
  },
] as const;

async function setCaseResponse(page: Page, caseFile: object): Promise<void> {
  await page.addInitScript(
    ({ controlsKey, stateKey, response }) => {
      window.localStorage.removeItem(stateKey);
      window.localStorage.setItem(
        controlsKey,
        JSON.stringify({ delayMs: 0, responses: { open_opportunity_case: response } }),
      );
    },
    { controlsKey: MOCK_INVOKE_CONTROLS_KEY, stateKey: MOCK_STATE_KEY, response: caseFile },
  );
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const widths = await page.evaluate(() => [
    window.innerWidth,
    document.body.scrollWidth,
    document.documentElement.scrollWidth,
  ]);
  expect(Math.max(widths[1], widths[2])).toBeLessThanOrEqual(widths[0]);
}

test.describe("Opportunity case workflow states", () => {
  for (const viewport of [
    { name: "desktop", width: 1440, height: 1000 },
    { name: "narrow", width: 390, height: 844 },
  ]) {
    for (const state of cases) {
      test(`keeps ${state.name} state actionable at ${viewport.name} width`, async ({ page }) => {
        await page.setViewportSize(viewport);
        await setCaseResponse(page, state.caseFile);
        const dashboard = new DashboardPage(page);

        await dashboard.navigateTo();
        const card = dashboard.jobCards.first();
        await card.getByRole("button", { name: "Open case" }).click();
        const dialog = page.getByRole("dialog", { name: "Opportunity case" });

        await expect(dialog).toBeVisible();
        await expect(dialog.getByRole("heading", { name: "Employer dossier" })).toBeVisible();
        await expect(dialog.getByText("Local records and dated source evidence, not an employer rating.")).toBeVisible();
        if (state.name !== "offline") {
          await expect(dialog.getByText(state.expected, { exact: state.name !== "missing employer source" })).toBeVisible();
        }
        await expectNoHorizontalOverflow(page);

        if (state.name === "partial") {
          await expect(dialog.getByText("Application: Applied. No contact recorded.")).toBeVisible();
          await expect(dialog.getByText("1 upcoming interview, 2 completed interviews.")).toBeVisible();
        }
        if (state.name === "offline") {
          await dialog.getByRole("button", { name: "Prepare this job" }).click();
          await expect(dialog.getByText("Review remains available offline.")).toBeVisible();
        }
        if (state.name === "failed source") {
          await expect(dialog.getByText(/Source may be stale.*Source refresh needs a connection/)).toBeVisible();
        }
        await expectNoHorizontalOverflow(page);
      });
    }
  }

  for (const viewport of [
    { name: "desktop", width: 1440, height: 1000 },
    { name: "narrow", width: 390, height: 844 },
  ]) {
    test(`opens the saved job's local employer dossier from Research company at ${viewport.name} width`, async ({ page }) => {
      await page.setViewportSize(viewport);
      await setCaseResponse(page, baseCase);
      const dashboard = new DashboardPage(page);

      await dashboard.navigateTo();
      await dashboard.jobCards.first().getByRole("button", { name: "Research company" }).click();
      const dialog = page.getByRole("dialog", { name: "Company research for Shopify" });

      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole("heading", { name: "Employer dossier" })).toBeVisible();
      await expect(dialog.getByText("Local records and dated source evidence, not an employer rating.")).toBeVisible();
      await expect(dialog.getByText("Shopify. No official employer domain is recorded.")).toBeVisible();
      await expectNoHorizontalOverflow(page);
    });
  }
});
