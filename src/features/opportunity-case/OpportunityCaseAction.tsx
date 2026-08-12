/** Presents one current local opportunity case and its bounded preparation actions. */

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "../../platform/tauri";
import { formatCompactDateTime, formatEventDate } from "../../shared/dateFormatting";
import { Button } from "../../ui/Button";
import { Modal } from "../../ui/Modal";
import { decodeEmployerDossier, EmployerDossierSection, type EmployerDossier } from "../../shared/EmployerDossierSection";
import { PackPacketBuilder } from "./PackPacketBuilder";

type CaseFile = {
  job: {
    job_hash: string;
    title: string;
    company: string;
    location: string | null;
    remote: boolean | null;
    times_seen: number;
  };
  source: {
    name: string;
    last_seen_at: string;
    connectivity_required: boolean;
    stale: boolean;
  };
  posting_risk: { score: number | null; reasons: string[] };
  application: null | { status: string; has_contact: boolean };
  interviews: null | { upcoming_count: number; completed_count: number };
  offer: null | { status: "pending" | "accepted" | "declined" };
  outcome: null | {
    status: "offer_accepted" | "offer_rejected" | "rejected" | "ghosted" | "withdrawn";
  };
  evidence: {
    confirmed_count: number;
    current_packet_count: number;
    stale_packet_count: number;
    review_status: "ready" | "no_saved_match" | "needs_refresh";
    requirements: Array<{
      requirement: string;
      importance: "required" | "preferred" | "industry";
      match_state: "direct" | "strong" | "partial" | "implied" | "missing";
      hard_constraint: boolean;
      blocking: boolean;
      why_not: "partial_evidence" | "implied_evidence" | "missing_evidence" | null;
      evidence: Array<{
        kind: "resume_bullet" | "project" | "skill" | "certification" | "resume_evidence";
        confirmed: boolean;
      }>;
    }>;
  };
  decision: {
    kind: "apply" | "maybe" | "skip" | "research_more";
    reasons: string[];
  };
  employer_dossier: EmployerDossier;
  timeline: Array<{ at: string; kind: string }>;
};

interface OpportunityCaseActionProps {
  jobHash: string;
}

const timelineLabels: Record<string, string> = {
  case_created: "Case created",
  status_changed: "Status changed",
  evidence_linked: "Evidence linked",
  source_checked_succeeded: "Source checked",
  source_checked_failed: "Source check failed",
  source_checked_timed_out: "Source check timed out",
  source_checked_cancelled: "Source check cancelled",
  privacy_receipt_recorded: "Privacy receipt recorded",
  source_policy_changed: "Source policy changed",
  recovery_restored: "Data restored",
  recovery_failed: "Recovery failed",
  recovery_retried: "Recovery retried",
  application_status_changed: "Application status changed",
  application_email_received: "Application email received",
  application_email_sent: "Application email sent",
  application_phone_called: "Application phone call",
  application_interview_scheduled: "Interview scheduled",
  application_note_added: "Application note added",
  application_reminder_set: "Application reminder set",
};

function titleCase(value: string) {
  return value.replace(/_/g, " ").replace(/\b\w/g, (letter: string) => letter.toUpperCase());
}

function getCaseStatus(caseFile: CaseFile) {
  if (caseFile.outcome) return `Outcome: ${titleCase(caseFile.outcome.status)}`;
  if (caseFile.offer) return `Offer: ${titleCase(caseFile.offer.status)}`;
  if (caseFile.interviews && caseFile.interviews.upcoming_count > 0) {
    return `${caseFile.interviews.upcoming_count} upcoming interview${caseFile.interviews.upcoming_count === 1 ? "" : "s"}`;
  }
  if (caseFile.application) return `Application: ${titleCase(caseFile.application.status)}`;
  return "No application activity yet";
}

const decisionLabels: Record<CaseFile["decision"]["kind"], string> = {
  apply: "Apply",
  maybe: "Maybe",
  skip: "Skip",
  research_more: "Research more",
};

const requirementStateLabels: Record<CaseFile["evidence"]["requirements"][number]["match_state"], string> = {
  direct: "Visible evidence",
  strong: "Visible evidence",
  partial: "Needs support",
  implied: "Check wording",
  missing: "Not found",
};

const evidenceKindLabels: Record<CaseFile["evidence"]["requirements"][number]["evidence"][number]["kind"], string> = {
  resume_bullet: "Resume bullet",
  project: "Project",
  skill: "Skill",
  certification: "Certification",
  resume_evidence: "Resume evidence",
};

function getPreparationRows(caseFile: CaseFile) {
  const sourceReview = caseFile.source.stale ? "The saved source may be stale." : "The saved source is current for this local review.";
  const sourceRefresh = caseFile.source.connectivity_required ? "A refresh is separate and needs a connection." : "Review remains available offline.";

  let evidenceReview: string;
  if (caseFile.evidence.review_status === "no_saved_match") {
    evidenceReview = "Compare this job with your active saved resume before tailoring.";
  } else if (caseFile.evidence.review_status === "needs_refresh") {
    evidenceReview = "Refresh the active saved-resume evidence review before relying on it.";
  } else if (caseFile.evidence.requirements.length === 0) {
    evidenceReview = "No recognized requirements are available. Review the posting before tailoring.";
  } else {
    evidenceReview = `${decisionLabels[caseFile.decision.kind]}. ${caseFile.evidence.requirements.length} requirements reviewed and ${caseFile.evidence.confirmed_count} confirmed evidence link${caseFile.evidence.confirmed_count === 1 ? "" : "s"}. ${caseFile.decision.reasons.join(" ")}`;
  }

  const currentClaims =
    caseFile.evidence.current_packet_count === 0
      ? "No current reviewed claims are ready."
      : `${caseFile.evidence.current_packet_count} current reviewed claim${caseFile.evidence.current_packet_count === 1 ? " is" : "s are"} available. Review the exact wording before reuse.`;
  const staleClaims =
    caseFile.evidence.stale_packet_count === 0
      ? ""
      : ` ${caseFile.evidence.stale_packet_count} reviewed claim${caseFile.evidence.stale_packet_count === 1 ? " needs" : "s need"} confirmation before reuse.`;
  const finalReview = caseFile.outcome
    ? "This opportunity is closed. Review the recorded outcome instead of preparing another submission."
    : {
        apply: "Review every field and attachment, then submit on the employer site yourself.",
        maybe: "Review the listed uncertainties before deciding whether to prepare or submit an application.",
        skip: "Review the Skip recommendation and its reasons before deciding whether to prepare an application.",
        research_more: "Resolve the listed blockers before preparing or submitting an application.",
      }[caseFile.decision.kind];

  return [
    { label: "Case status", detail: getCaseStatus(caseFile) },
    {
      label: "Source and role",
      detail: `${caseFile.job.title} at ${caseFile.job.company}, saved from ${caseFile.source.name}. ${sourceReview} ${sourceRefresh}`,
    },
    { label: "Fit and evidence", detail: evidenceReview },
    { label: "Reviewed claims", detail: `${currentClaims}${staleClaims}` },
    {
      label: "Application materials",
      detail: "Choose and verify the exact resume, cover note, work samples, references, and other attachments. JobSentinel has not selected or attached files.",
    },
    {
      label: "Screening answers",
      detail:
        "Use the employer's exact wording and current records for citizenship, work authorization, clearance, and eligibility. Protected veteran status, disability, race/ethnicity, gender, and other voluntary sensitive personal questions stay local and are yours to answer.",
    },
    { label: "Final review", detail: finalReview },
  ];
}

export function OpportunityCaseAction({ jobHash }: OpportunityCaseActionProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [caseFile, setCaseFile] = useState<CaseFile | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);
  const [showPreparation, setShowPreparation] = useState(false);
  const requestEpoch = useRef(0);

  useEffect(() => {
    requestEpoch.current += 1;
    setIsOpen(false);
    setCaseFile(null);
    setIsLoading(false);
    setHasError(false);
    setShowPreparation(false);
  }, [jobHash]);

  const openCase = useCallback(async () => {
    const request = ++requestEpoch.current;
    const requestedHash = jobHash;
    setIsLoading(true);
    setHasError(false);
    try {
      const opened = await invoke<CaseFile>("open_opportunity_case", {
        jobHash: requestedHash,
      });
      if (request !== requestEpoch.current) return;
      if (opened.job.job_hash !== requestedHash) throw new Error("case identity changed");
      const employerDossier = decodeEmployerDossier(opened.employer_dossier);
      if (!employerDossier) throw new Error("employer dossier is invalid");
      setCaseFile({ ...opened, employer_dossier: employerDossier });
    } catch {
      if (request === requestEpoch.current) setHasError(true);
    } finally {
      if (request === requestEpoch.current) setIsLoading(false);
    }
  }, [jobHash]);

  const handleOpen = () => {
    setShowPreparation(false);
    setIsOpen(true);
    void openCase();
  };

  const close = () => {
    requestEpoch.current += 1;
    setIsOpen(false);
    setCaseFile(null);
    setHasError(false);
    setShowPreparation(false);
  };

  return (
    <>
      <Button variant="secondary" size="sm" onClick={handleOpen}>
        Open case
      </Button>
      <Modal isOpen={isOpen} onClose={close} title="Opportunity case" size="lg">
        {isLoading && <p role="status">Opening case…</p>}
        {hasError && (
          <div className="space-y-3" role="alert">
            <p>Could not open this case.</p>
            <Button variant="secondary" size="sm" onClick={() => void openCase()}>
              Retry
            </Button>
          </div>
        )}
        {caseFile && !isLoading && !hasError && (
          <div className="min-w-0 space-y-5 text-sm text-surface-700 dark:text-surface-300">
            <header>
              <h3 className="text-lg font-semibold text-surface-900 dark:text-white">{caseFile.job.title}</h3>
              <p>
                {caseFile.job.company}
                {caseFile.job.location ? `, ${caseFile.job.location}` : ""}
              </p>
              <p>{caseFile.job.times_seen > 1 ? `Seen ${caseFile.job.times_seen} times. Review duplicate postings before preparing.` : "Seen once."}</p>
            </header>

            <Button variant="secondary" size="sm" aria-controls="case-preparation" aria-expanded={showPreparation} onClick={() => setShowPreparation((visible) => !visible)}>
              {showPreparation ? "Back to case" : "Prepare this job"}
            </Button>

            {showPreparation ? (
              <section id="case-preparation" aria-labelledby="case-preparation-title" className="min-w-0 space-y-3">
                <div>
                  <h4 id="case-preparation-title" className="font-semibold text-surface-900 dark:text-white">
                    Preparation workup
                  </h4>
                  <p>Local only. Nothing is sent or submitted.</p>
                </div>
                <dl className="space-y-3">
                  {getPreparationRows(caseFile).map((row) => (
                    <div className="min-w-0 rounded-lg border border-surface-200 p-3 dark:border-surface-700" key={row.label}>
                      <dt className="font-semibold text-surface-900 dark:text-white">{row.label}</dt>
                      <dd className="break-words">{row.detail}</dd>
                    </div>
                  ))}
                </dl>
                {!caseFile.outcome && caseFile.offer?.status !== "accepted" && caseFile.evidence.review_status === "ready" ? <PackPacketBuilder jobHash={caseFile.job.job_hash} /> : null}
              </section>
            ) : (
              <>
                <section aria-labelledby="case-status">
                  <h4 id="case-status" className="font-semibold text-surface-900 dark:text-white">
                    Case status
                  </h4>
                  <p>{getCaseStatus(caseFile)}</p>
                </section>

                <section aria-labelledby="case-progress">
                  <h4 id="case-progress" className="font-semibold text-surface-900 dark:text-white">
                    Case progress
                  </h4>
                  <p>
                    {caseFile.application
                      ? `Application: ${titleCase(caseFile.application.status)}. ${caseFile.application.has_contact ? "Contact recorded." : "No contact recorded."}`
                      : "No application activity yet."}
                  </p>
                  {caseFile.interviews && (
                    <p>
                      {caseFile.interviews.upcoming_count} upcoming interview
                      {caseFile.interviews.upcoming_count === 1 ? "" : "s"}, {caseFile.interviews.completed_count} completed interview
                      {caseFile.interviews.completed_count === 1 ? "" : "s"}.
                    </p>
                  )}
                </section>

                <EmployerDossierSection dossier={caseFile.employer_dossier} />

                <section aria-labelledby="case-risk">
                  <h4 id="case-risk" className="font-semibold text-surface-900 dark:text-white">
                    Posting risk
                  </h4>
                  <p>{caseFile.posting_risk.score === null ? "Posting risk has not been scored." : `Posting risk: ${Math.round(caseFile.posting_risk.score * 100)}%.`}</p>
                  {caseFile.posting_risk.reasons.map((reason) => (
                    <p key={reason}>{reason}</p>
                  ))}
                </section>

                <section aria-labelledby="case-decision">
                  <h4 id="case-decision" className="font-semibold text-surface-900 dark:text-white">
                    Decision summary
                  </h4>
                  <p className="font-medium">{decisionLabels[caseFile.decision.kind]}</p>
                </section>

                <section aria-labelledby="case-why-not">
                  <h4 id="case-why-not" className="font-semibold text-surface-900 dark:text-white">
                    Why not this job?
                  </h4>
                  {caseFile.decision.kind === "apply" ? (
                    <p>No current blockers recorded.</p>
                  ) : (
                    <ul className="list-disc space-y-1 pl-5">
                      {caseFile.decision.reasons.map((reason) => (
                        <li key={reason}>{reason}</li>
                      ))}
                    </ul>
                  )}
                </section>

                <section aria-labelledby="case-evidence">
                  <h4 id="case-evidence" className="font-semibold text-surface-900 dark:text-white">
                    Evidence wall
                  </h4>
                  <p>
                    {caseFile.evidence.confirmed_count} confirmed, {caseFile.evidence.current_packet_count} current packet
                    {caseFile.evidence.current_packet_count === 1 ? "" : "s"}.
                  </p>
                  {caseFile.evidence.stale_packet_count > 0 && <p>Evidence needs review before reuse.</p>}
                  {caseFile.evidence.review_status === "no_saved_match" && <p>Compare this job with your active saved resume to build the evidence wall.</p>}
                  {caseFile.evidence.review_status === "needs_refresh" && <p>The saved-resume evidence review changed. Refresh it before relying on the results.</p>}
                  {caseFile.evidence.review_status === "ready" && caseFile.evidence.requirements.length === 0 && (
                    <p>The posting does not contain enough recognized requirements for an evidence review.</p>
                  )}
                  {caseFile.evidence.requirements.length > 0 && (
                    <div className="mt-3 space-y-3">
                      {caseFile.evidence.requirements.map((requirement) => (
                        <article className="rounded-lg border border-surface-200 p-3 dark:border-surface-700" key={`${requirement.requirement}-${requirement.importance}`}>
                          <div className="flex min-w-0 flex-wrap items-start justify-between gap-2">
                            <h5 className="break-words font-medium text-surface-900 dark:text-white">{requirement.requirement}</h5>
                            <div className="flex flex-wrap gap-2 text-xs font-medium">
                              <span>{titleCase(requirement.importance)}</span>
                              {requirement.blocking && <span>Hard blocker</span>}
                            </div>
                          </div>
                          <p>{requirementStateLabels[requirement.match_state]}</p>
                          {requirement.why_not && <p>Why not: {requirement.why_not.replace(/_/g, " ")}</p>}
                          {requirement.evidence.length === 0 ? (
                            <p>No supporting evidence available</p>
                          ) : (
                            <ul className="mt-2 space-y-1">
                              {requirement.evidence.map((evidence, index) => (
                                <li key={`${evidence.kind}-${index}`}>
                                  {evidenceKindLabels[evidence.kind]}, {evidence.confirmed ? "confirmed" : "not confirmed"}
                                </li>
                              ))}
                            </ul>
                          )}
                        </article>
                      ))}
                    </div>
                  )}
                </section>

                <section aria-labelledby="case-timeline">
                  <h4 id="case-timeline" className="font-semibold text-surface-900 dark:text-white">
                    Timeline
                  </h4>
                  {caseFile.timeline.length === 0 ? (
                    <p>No case activity yet.</p>
                  ) : (
                    <ul className="space-y-1">
                      {caseFile.timeline.map((event, index) => (
                        <li className="flex flex-wrap justify-between gap-x-3" key={`${event.at}-${event.kind}-${index}`}>
                          <span>{timelineLabels[event.kind] ?? titleCase(event.kind)}</span>
                          <time className="text-surface-500" dateTime={event.at}>
                            {formatCompactDateTime(event.at)}
                          </time>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>

                <p className="rounded-lg bg-surface-50 p-3 text-surface-600 dark:bg-surface-800 dark:text-surface-300">
                  Source: {caseFile.source.name}. Last checked <time dateTime={caseFile.source.last_seen_at}>{formatEventDate(caseFile.source.last_seen_at)}</time>.{" "}
                  {caseFile.source.stale ? "Source may be stale. " : ""}This local case is available offline.
                  {caseFile.source.connectivity_required ? " Source refresh needs a connection." : ""}
                </p>
              </>
            )}
          </div>
        )}
      </Modal>
    </>
  );
}
