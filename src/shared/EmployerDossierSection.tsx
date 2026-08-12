/** Decodes and presents one bounded, provenance-first local employer dossier. */

import { formatEventDate } from "./dateFormatting";

const sourceClasses = [
  "official_public_api",
  "public_ats",
  "public_employer_page",
  "regional_board",
  "restricted_public_scheduled",
  "user_import",
  "restricted_user_opened",
] as const;
const sourceStatuses = [
  "current",
  "stale",
  "review_required",
  "policy_disabled",
  "policy_changed",
  "review_expired",
  "unavailable",
  "missing",
] as const;
const payClarities = ["range_listed", "minimum_only", "maximum_only", "not_listed", "unusable"] as const;
const salaryCoverages = ["none", "partial", "declared"] as const;
const applicationChannels = ["public_ats", "official_public_source", "employer_page", "user_provided", "unknown"] as const;
const uncertainties = [
  "employer_identity_not_canonical",
  "official_domain_unknown",
  "role_not_live_checked",
  "retrieval_date_unavailable",
  "jurisdiction_unknown",
  "exact_name_history_only",
  "source_missing",
  "source_not_current",
  "pay_not_listed",
  "pay_unusable",
  "pay_provenance_incomplete",
] as const;
const nextActions = ["open_saved_posting", "verify_on_employer_careers_page"] as const;

type SourceClass = (typeof sourceClasses)[number];
type SourceStatus = (typeof sourceStatuses)[number];
type PayClarity = (typeof payClarities)[number];
type SalaryCoverage = (typeof salaryCoverages)[number];
type ApplicationChannel = (typeof applicationChannels)[number];
type Uncertainty = (typeof uncertainties)[number];
type NextAction = (typeof nextActions)[number];

export interface EmployerDossier {
  employer: {
    name: string;
    identity_status: "unverified_saved_name";
    official_domain: null;
    posting_domain: string | null;
  };
  role: {
    title: string;
    status: "last_observed";
    posting_url: string | null;
    first_observed_at: string;
    last_observed_at: string;
    times_seen: number;
    repost_count: number;
  };
  source: {
    source_id: string;
    display_name: string | null;
    source_class: SourceClass | null;
    status: SourceStatus;
    documentation_url: string | null;
    observed_at: string;
    retrieved_at: string | null;
    verified_on: string | null;
    expires_on: string | null;
    jurisdiction: string | null;
    confidence_percent: number | null;
    policy_ref: string | null;
    policy_revision: number | null;
    terms_review_ref: string | null;
    robots_review_ref: string | null;
    parser_version: string | null;
    salary_coverage: SalaryCoverage | null;
    incomplete_coverage: boolean | null;
  };
  pay: {
    clarity: PayClarity;
    minimum: number | null;
    maximum: number | null;
    currency: string | null;
    observed_at: string;
  };
  local_history: {
    basis: "exact_saved_name";
    saved_job_count: number;
    application_count: number;
    interview_count: number;
    offer_count: number;
    terminal_outcome_count: number;
  };
  application_channel: ApplicationChannel;
  uncertainty: Uncertainty[];
  next_action: NextAction;
}

type UnknownRecord = Record<string, unknown>;

function record(value: unknown): UnknownRecord | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as UnknownRecord)
    : null;
}

function boundedString(value: unknown, maximum = 512): string | null {
  return typeof value === "string" && value.length > 0 && value.length <= maximum && ![...value].some((character) => {
    const code = character.charCodeAt(0);
    return code <= 31 || code === 127;
  })
    ? value
    : null;
}

function nullableString(value: unknown, maximum = 512): string | null | undefined {
  return value === null ? null : boundedString(value, maximum) ?? undefined;
}

function enumValue<T extends readonly string[]>(value: unknown, allowed: T): T[number] | null {
  return typeof value === "string" && allowed.includes(value) ? (value as T[number]) : null;
}

function count(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 && (value as number) <= 4_294_967_295
    ? (value as number)
    : null;
}

function positiveInteger(value: unknown, maximum: number): number | null {
  return Number.isInteger(value) && (value as number) > 0 && (value as number) <= maximum ? (value as number) : null;
}

function dateTime(value: unknown): string | null {
  const parsed = boundedString(value, 40);
  return parsed && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/u.test(parsed) && Number.isFinite(Date.parse(parsed))
    ? parsed
    : null;
}

function date(value: unknown): string | null {
  const parsed = boundedString(value, 10);
  const match = parsed?.match(/^(\d{4})-(\d{2})-(\d{2})$/u);
  if (!parsed || !match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const instant = new Date(Date.UTC(year, month - 1, day));
  return instant.getUTCFullYear() === year && instant.getUTCMonth() === month - 1 && instant.getUTCDate() === day ? parsed : null;
}

function nullableDate(value: unknown): string | null | undefined {
  return value === null ? null : date(value) ?? undefined;
}

function nullableDateTime(value: unknown): string | null | undefined {
  return value === null ? null : dateTime(value) ?? undefined;
}

function postingUrl(sourceId: string, value: unknown): { url: string; domain: string } | null | undefined {
  if (value === null) return null;
  if (typeof value !== "string" || value.length > 2_048) return undefined;
  try {
    const parsed = new URL(value);
    const domain = parsed.hostname.toLowerCase().replace(/\.$/u, "");
    const allowed =
      parsed.protocol === "https:" &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.port === "" &&
      parsed.search === "" &&
      parsed.hash === "" &&
      ((sourceId === "greenhouse" && ["boards.greenhouse.io", "job-boards.greenhouse.io"].includes(domain)) ||
        (sourceId === "lever" && domain === "jobs.lever.co"));
    return allowed ? { url: parsed.toString(), domain } : undefined;
  } catch {
    return undefined;
  }
}

// eslint-disable-next-line react-refresh/only-export-components -- The renderer and its runtime decoder share one closed contract.
export function decodeEmployerDossier(value: unknown): EmployerDossier | null {
  const dossier = record(value);
  const employer = record(dossier?.employer);
  const role = record(dossier?.role);
  const source = record(dossier?.source);
  const pay = record(dossier?.pay);
  const localHistory = record(dossier?.local_history);
  if (!dossier || !employer || !role || !source || !pay || !localHistory) return null;

  const name = boundedString(employer.name);
  const sourceId = boundedString(source.source_id, 128);
  const title = boundedString(role.title);
  const firstObservedAt = dateTime(role.first_observed_at);
  const lastObservedAt = dateTime(role.last_observed_at);
  const observedAt = dateTime(source.observed_at);
  const payObservedAt = dateTime(pay.observed_at);
  const timesSeen = count(role.times_seen);
  const repostCount = count(role.repost_count);
  const safePosting = sourceId ? postingUrl(sourceId, role.posting_url) : undefined;
  const postingDomain = nullableString(employer.posting_domain, 253);
  const sourceClass = source.source_class === null ? null : enumValue(source.source_class, sourceClasses);
  const sourceStatus = enumValue(source.status, sourceStatuses);
  const clarity = enumValue(pay.clarity, payClarities);
  const channel = enumValue(dossier.application_channel, applicationChannels);
  const action = enumValue(dossier.next_action, nextActions);
  if (!Array.isArray(dossier.uncertainty) || dossier.uncertainty.length > 16) return null;
  const uncertainty = dossier.uncertainty.map((item) => enumValue(item, uncertainties));
  if (
    !name ||
    !title ||
    !sourceId ||
    !firstObservedAt ||
    !lastObservedAt ||
    !observedAt ||
    !payObservedAt ||
    timesSeen === null ||
    repostCount === null ||
    safePosting === undefined ||
    postingDomain === undefined ||
    sourceClass === null && source.source_class !== null ||
    !sourceStatus ||
    !clarity ||
    !channel ||
    !action ||
    uncertainty.some((item) => item === null) ||
    new Set(uncertainty).size !== uncertainty.length ||
    employer.identity_status !== "unverified_saved_name" ||
    employer.official_domain !== null ||
    role.status !== "last_observed" ||
    localHistory.basis !== "exact_saved_name" ||
    (safePosting?.domain ?? null) !== postingDomain
  ) return null;

  const nullableFields = {
    displayName: nullableString(source.display_name),
    documentationUrl: nullableString(source.documentation_url, 2_048),
    retrievedAt: nullableDateTime(source.retrieved_at),
    verifiedOn: nullableDate(source.verified_on),
    expiresOn: nullableDate(source.expires_on),
    jurisdiction: nullableString(source.jurisdiction, 128),
    policyRef: nullableString(source.policy_ref, 256),
    termsReviewRef: nullableString(source.terms_review_ref, 256),
    robotsReviewRef: nullableString(source.robots_review_ref, 256),
    parserVersion: nullableString(source.parser_version, 128),
  };
  if (Object.values(nullableFields).some((item) => item === undefined)) return null;
  const confidence = source.confidence_percent === null ? null : positiveInteger(source.confidence_percent, 100);
  const policyRevision = source.policy_revision === null ? null : positiveInteger(source.policy_revision, 1_000_000);
  const salaryCoverage = source.salary_coverage === null ? null : enumValue(source.salary_coverage, salaryCoverages);
  const minimum = pay.minimum === null ? null : positiveInteger(pay.minimum, Number.MAX_SAFE_INTEGER);
  const maximum = pay.maximum === null ? null : positiveInteger(pay.maximum, Number.MAX_SAFE_INTEGER);
  const currency = pay.currency === null ? null : boundedString(pay.currency, 3);
  const historyCounts = [
    localHistory.saved_job_count,
    localHistory.application_count,
    localHistory.interview_count,
    localHistory.offer_count,
    localHistory.terminal_outcome_count,
  ].map(count);
  const payShapeIsValid =
    (clarity === "range_listed" && minimum !== null && maximum !== null && maximum >= minimum && currency !== null) ||
    (clarity === "minimum_only" && minimum !== null && maximum === null && currency !== null) ||
    (clarity === "maximum_only" && minimum === null && maximum !== null && currency !== null) ||
    ((clarity === "not_listed" || clarity === "unusable") && minimum === null && maximum === null && currency === null);
  const sourceProvenance = [
    sourceClass,
    nullableFields.displayName,
    nullableFields.verifiedOn,
    nullableFields.expiresOn,
    confidence,
    nullableFields.policyRef,
    policyRevision,
    nullableFields.termsReviewRef,
    nullableFields.robotsReviewRef,
    nullableFields.parserVersion,
    salaryCoverage,
    source.incomplete_coverage,
  ];
  const sourceShapeIsValid = sourceStatus === "missing"
    ? [...sourceProvenance, nullableFields.documentationUrl, nullableFields.retrievedAt, nullableFields.jurisdiction]
      .every((item) => item === null)
    : sourceProvenance.every((item) => item !== null);
  const expectedUncertainty: Uncertainty[] = [
    "employer_identity_not_canonical",
    "official_domain_unknown",
    "role_not_live_checked",
    "retrieval_date_unavailable",
    "jurisdiction_unknown",
    "exact_name_history_only",
  ];
  if (sourceStatus === "missing") expectedUncertainty.push("source_missing");
  else if (sourceStatus !== "current") expectedUncertainty.push("source_not_current");
  if (clarity === "not_listed") expectedUncertainty.push("pay_not_listed");
  else if (clarity === "unusable") expectedUncertainty.push("pay_unusable");
  else if (salaryCoverage !== "declared") expectedUncertainty.push("pay_provenance_incomplete");
  const uncertaintyIsComplete =
    uncertainty.length === expectedUncertainty.length &&
    expectedUncertainty.every((item) => uncertainty.includes(item));
  const channelMatchesSource =
    channel === "unknown" ||
    (channel === "public_ats" && sourceClass === "public_ats") ||
    (channel === "official_public_source" && sourceClass === "official_public_api") ||
    (channel === "employer_page" && sourceClass === "public_employer_page") ||
    (channel === "user_provided" && sourceClass === "user_import");
  const openActionIsValid =
    (action === "open_saved_posting") === (sourceStatus === "current" && safePosting !== null);
  const datesAreConsistent =
    Date.parse(firstObservedAt) <= Date.parse(lastObservedAt) &&
    observedAt === lastObservedAt &&
    payObservedAt === lastObservedAt &&
    (nullableFields.verifiedOn === null ||
      nullableFields.expiresOn === null ||
      (typeof nullableFields.verifiedOn === "string" &&
        typeof nullableFields.expiresOn === "string" &&
        nullableFields.verifiedOn <= nullableFields.expiresOn));
  if (
    (source.confidence_percent !== null && confidence === null) ||
    (source.policy_revision !== null && policyRevision === null) ||
    (source.salary_coverage !== null && salaryCoverage === null) ||
    typeof source.incomplete_coverage !== "boolean" && source.incomplete_coverage !== null ||
    (pay.minimum !== null && minimum === null) ||
    (pay.maximum !== null && maximum === null) ||
    (pay.currency !== null && (!currency || !/^[A-Z]{3}$/u.test(currency))) ||
    !payShapeIsValid ||
    !sourceShapeIsValid ||
    !uncertaintyIsComplete ||
    !channelMatchesSource ||
    !openActionIsValid ||
    !datesAreConsistent ||
    timesSeen < 1 ||
    historyCounts.some((item) => item === null) ||
    historyCounts[0] === 0
  ) return null;

  return {
    employer: { name, identity_status: "unverified_saved_name", official_domain: null, posting_domain: postingDomain },
    role: { title, status: "last_observed", posting_url: safePosting?.url ?? null, first_observed_at: firstObservedAt, last_observed_at: lastObservedAt, times_seen: timesSeen, repost_count: repostCount },
    source: { source_id: sourceId, display_name: nullableFields.displayName!, source_class: sourceClass, status: sourceStatus, documentation_url: nullableFields.documentationUrl!, observed_at: observedAt, retrieved_at: nullableFields.retrievedAt!, verified_on: nullableFields.verifiedOn!, expires_on: nullableFields.expiresOn!, jurisdiction: nullableFields.jurisdiction!, confidence_percent: confidence, policy_ref: nullableFields.policyRef!, policy_revision: policyRevision, terms_review_ref: nullableFields.termsReviewRef!, robots_review_ref: nullableFields.robotsReviewRef!, parser_version: nullableFields.parserVersion!, salary_coverage: salaryCoverage, incomplete_coverage: source.incomplete_coverage as boolean | null },
    pay: { clarity, minimum, maximum, currency, observed_at: payObservedAt },
    local_history: { basis: "exact_saved_name", saved_job_count: historyCounts[0]!, application_count: historyCounts[1]!, interview_count: historyCounts[2]!, offer_count: historyCounts[3]!, terminal_outcome_count: historyCounts[4]! },
    application_channel: channel,
    uncertainty: uncertainty as Uncertainty[],
    next_action: action,
  };
}

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
