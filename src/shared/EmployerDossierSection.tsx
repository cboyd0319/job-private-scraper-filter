/** Presents one bounded, provenance-first local employer dossier. */

import { formatEventDate } from "./dateFormatting";
import type { EmployerDossier } from "./employerDossier";

type SourceStatus = EmployerDossier["source"]["status"];
type ApplicationChannel = EmployerDossier["application_channel"];
type SalaryCoverage = NonNullable<EmployerDossier["source"]["salary_coverage"]>;
type Uncertainty = EmployerDossier["uncertainty"][number];

const sourceStatusLabels: Record<SourceStatus, string> = {
  current: "Current",
  stale: "Stale",
  review_required: "Review required",
  policy_disabled: "Policy disabled",
  policy_changed: "Policy changed",
  review_expired: "Review expired",
  unavailable: "Unavailable",
  missing: "No reviewed source record",
};
const channelLabels: Record<ApplicationChannel, string> = {
  public_ats: "Public ATS posting",
  official_public_source: "Official public source",
  employer_page: "Employer page",
  user_provided: "User-provided source",
  unknown: "Unknown",
};
const salaryCoverageLabels: Record<SalaryCoverage, string> = {
  none: "none",
  partial: "partial",
  declared: "declared",
};
const uncertaintyLabels: Record<Uncertainty, string> = {
  employer_identity_not_canonical: "The saved employer name is not a canonical identity.",
  official_domain_unknown: "No official employer domain is recorded.",
  role_not_live_checked: "Role availability has not been checked live.",
  retrieval_date_unavailable: "A separate retrieval timestamp is unavailable.",
  jurisdiction_unknown: "Jurisdiction is unknown.",
  exact_name_history_only: "Local history includes exact saved-name matches only.",
  source_missing: "No reviewed source manifest is available.",
  source_not_current: "The source record is not current.",
  pay_not_listed: "No pay range is listed in the saved record.",
  pay_unusable: "Saved pay data is incomplete or malformed.",
  pay_provenance_incomplete: "The source manifest does not declare complete salary coverage.",
};

function shortDate(value: string): string {
  return new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", year: "numeric", timeZone: "UTC" }).format(new Date(`${value}T00:00:00Z`));
}

function payLabel(pay: EmployerDossier["pay"]): string {
  const number = new Intl.NumberFormat("en-US");
  if (pay.clarity === "range_listed") return `${number.format(pay.minimum!)}–${number.format(pay.maximum!)} ${pay.currency}`;
  if (pay.clarity === "minimum_only") return `From ${number.format(pay.minimum!)} ${pay.currency}`;
  if (pay.clarity === "maximum_only") return `Up to ${number.format(pay.maximum!)} ${pay.currency}`;
  return pay.clarity === "not_listed" ? "Not listed" : "Saved pay data is unusable";
}

function counted(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

export function EmployerDossierSection({ dossier }: { dossier: EmployerDossier }) {
  const status = dossier.source.status === "current" && dossier.source.expires_on
    ? `Current through ${shortDate(dossier.source.expires_on)}`
    : sourceStatusLabels[dossier.source.status];
  return (
    <section aria-labelledby="employer-dossier-title" className="min-w-0 space-y-3">
      <div>
        <h4 id="employer-dossier-title" className="font-semibold text-surface-900 dark:text-white">Employer dossier</h4>
        <p>Local records and dated source evidence, not an employer rating.</p>
      </div>
      <dl className="grid min-w-0 gap-3 sm:grid-cols-2">
        <div><dt className="font-medium text-surface-900 dark:text-white">Identity</dt><dd>{dossier.employer.name}. No official employer domain is recorded.</dd></div>
        <div><dt className="font-medium text-surface-900 dark:text-white">Role status</dt><dd>{dossier.role.title} was last observed <time dateTime={dossier.role.last_observed_at}>{formatEventDate(dossier.role.last_observed_at)}</time>. This is not a live availability check.</dd></div>
        <div><dt className="font-medium text-surface-900 dark:text-white">Posting history</dt><dd>First observed <time dateTime={dossier.role.first_observed_at}>{formatEventDate(dossier.role.first_observed_at)}</time>. Seen {dossier.role.times_seen} times; {counted(dossier.role.repost_count, "repost")}.</dd></div>
        <div><dt className="font-medium text-surface-900 dark:text-white">Pay clarity</dt><dd>{payLabel(dossier.pay)}. Observed <time dateTime={dossier.pay.observed_at}>{formatEventDate(dossier.pay.observed_at)}</time>.</dd></div>
        <div><dt className="font-medium text-surface-900 dark:text-white">Application channel</dt><dd>{channelLabels[dossier.application_channel]}</dd></div>
        <div className="sm:col-span-2"><dt className="font-medium text-surface-900 dark:text-white">Local history</dt><dd>{counted(dossier.local_history.saved_job_count, "saved job")}, {counted(dossier.local_history.application_count, "application")}, {counted(dossier.local_history.interview_count, "interview")}, {counted(dossier.local_history.offer_count, "offer")}, {counted(dossier.local_history.terminal_outcome_count, "terminal outcome")}. Exact saved-name matches only.</dd></div>
      </dl>
      <p><span className="font-medium text-surface-900 dark:text-white">Source status:</span> {status}. {dossier.source.display_name ?? dossier.source.source_id}; last observed <time dateTime={dossier.source.observed_at}>{formatEventDate(dossier.source.observed_at)}</time>.</p>
      <p>Next action: {dossier.next_action === "open_saved_posting" ? "Open the saved posting and compare it with the employer careers page." : "Verify the role and employer domain on the employer careers page."}</p>
      <p>Safety check: This dossier does not label the employer fraudulent. Verify the employer-controlled domain and role. Do not pay fees or share sensitive information until both are confirmed.</p>
      {dossier.role.posting_url && (
        <div className="space-y-1">
          <a className="break-all text-primary-700 underline dark:text-primary-300" href={dossier.role.posting_url} target="_blank" rel="noopener noreferrer">Open saved posting</a>
          <p>Opening the saved posting contacts {dossier.employer.posting_domain} and discloses this page request to that site.</p>
        </div>
      )}
      <details>
        <summary className="cursor-pointer font-medium text-surface-900 dark:text-white">Provenance and uncertainty</summary>
        <div className="mt-2 space-y-2">
          <p>{dossier.source.source_class ? dossier.source.source_class.replace(/_/gu, " ") : "Unclassified source"}. Source metadata confidence: {dossier.source.confidence_percent ?? "unknown"}{dossier.source.confidence_percent === null ? "" : "%"}; this is not an employer score.</p>
          {dossier.source.verified_on && <p>Verified <time dateTime={dossier.source.verified_on}>{shortDate(dossier.source.verified_on)}</time>. Policy {dossier.source.policy_ref ?? "unknown"}, revision {dossier.source.policy_revision ?? "unknown"}. Parser {dossier.source.parser_version ?? "unknown"}.</p>}
          <p>Source policy metadata does not prove that this role is still open. Public ATS evidence, saved role observations, and exact-name local history remain separately attributed.</p>
          {dossier.source.terms_review_ref && <p>Terms review: {dossier.source.terms_review_ref}. Robots review: {dossier.source.robots_review_ref ?? "unavailable"}.</p>}
          <p>Retrieved: {dossier.source.retrieved_at ? formatEventDate(dossier.source.retrieved_at) : "unavailable"}. Jurisdiction: {dossier.source.jurisdiction ?? "unavailable"}.</p>
          <p>Salary coverage: {dossier.source.salary_coverage ? salaryCoverageLabels[dossier.source.salary_coverage] : "unknown"}. Source coverage is {dossier.source.incomplete_coverage === null ? "unknown" : dossier.source.incomplete_coverage ? "incomplete" : "not marked incomplete"}.</p>
          <ul className="list-disc space-y-1 pl-5">{dossier.uncertainty.map((item) => <li key={item}>{uncertaintyLabels[item]}</li>)}</ul>
        </div>
      </details>
    </section>
  );
}
