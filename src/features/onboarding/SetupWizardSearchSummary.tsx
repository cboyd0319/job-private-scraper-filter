import type { SetupSearchSummary } from "./setupWizardPreferences";

interface SetupWizardSearchSummaryProps {
  summary: SetupSearchSummary;
  resumeSkillSummary?: {
    resumeName: string;
    skills: string[];
  } | null;
}

export function SetupWizardSearchSummary({
  summary,
  resumeSkillSummary,
}: SetupWizardSearchSummaryProps) {
  return (
    <>
      <section
        className="mb-6 border-t border-surface-200 pt-5"
        aria-labelledby="setup-search-summary-title"
      >
        <h3
          id="setup-search-summary-title"
          className="mb-4 flex items-center gap-2 font-semibold text-surface-800"
        >
          <SummaryCheckIcon className="w-5 h-5 text-sentinel-600" />
          Review your search
        </h3>
        <dl className="space-y-3 text-sm">
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Look for</dt>
            <dd className="text-surface-800">{summary.titles}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Show more</dt>
            <dd className="text-surface-800">{summary.wantedWork}</dd>
          </div>
          {resumeSkillSummary && resumeSkillSummary.skills.length > 0 && (
            <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
              <dt className="font-medium text-surface-600">Saved resume skills</dt>
              <dd className="text-surface-800">
                From {resumeSkillSummary.resumeName}:{" "}
                {resumeSkillSummary.skills.join(", ")}. Remove any you do not
                want before starting.
              </dd>
            </div>
          )}
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Rank lower</dt>
            <dd className="text-surface-800">{summary.avoidedWork}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Location</dt>
            <dd className="text-surface-800">{summary.location}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Freshness</dt>
            <dd className="text-surface-800">{summary.freshness}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Review list</dt>
            <dd className="text-surface-800">{summary.reviewVolume}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Job sources</dt>
            <dd className="text-surface-800">{summary.jobSources}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Alerts</dt>
            <dd className="text-surface-800">{summary.alerts}</dd>
          </div>
          <div className="grid gap-1 sm:grid-cols-[7rem_1fr]">
            <dt className="font-medium text-surface-600">Pay</dt>
            <dd className="text-surface-800">{summary.pay}</dd>
          </div>
        </dl>
        <p className="mt-4 text-sm text-surface-500">
          JobSentinel uses these answers to rank jobs. You can change them later.
        </p>
      </section>

      <div className="p-4 bg-surface-50 rounded-lg mb-6">
        <p className="text-sm text-surface-600">
          <span className="font-medium text-surface-700">Your privacy matters:</span> JobSentinel
          saves these search settings on this computer. After you start, it can contact only checked
          job sources in this review and any alert services you later turn on. It does not send
          resumes, private notes, saved answers, or application history to job sources.
        </p>
      </div>
    </>
  );
}

function SummaryCheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M5 13l4 4L19 7" />
    </svg>
  );
}
