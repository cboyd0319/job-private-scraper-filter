/** Displays durable signed-pack review facts and safe lifecycle failures. */

import { useCallback, useEffect, useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import { Modal } from "../../../ui/Modal";
import {
  parsePackManagementReviews,
  type PackManagementReview,
  type PackPurpose,
  type PackReleaseReview,
  type PackState,
  type QuarantineReason,
} from "./packManagementProjection";

const STATE_LABELS: Record<PackState, string> = {
  needs_review: "Needs review",
  ready: "Ready",
  disabled: "Disabled",
  quarantined: "Quarantined",
  removed: "Removed",
};

const STATE_TONES: Record<PackState, string> = {
  needs_review: "bg-warning/15 text-surface-800 dark:text-warning",
  ready: "bg-success/10 text-success",
  disabled:
    "bg-surface-100 text-surface-700 dark:bg-surface-700 dark:text-surface-200",
  quarantined: "bg-danger/10 text-danger",
  removed:
    "bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400",
};

const QUARANTINE_COPY: Record<QuarantineReason, string> = {
  self_test_failed: "Pack self-test failed",
  trust_revoked: "Publisher trust was revoked",
  interrupted: "Installation was interrupted",
  artifact_missing: "Installed file is missing",
  integrity_failed: "Installed file did not pass verification",
};

const PURPOSE_COPY: Record<PackPurpose, string> = {
  static_guidance: "Provides static job-search guidance.",
  resume_evidence_review: "Reviews selected resume evidence.",
  draft_application_packet: "Builds a draft application packet.",
  reviewed_agent: "Runs a reviewed local agent task.",
  reviewed_workflow: "Runs a reviewed local workflow.",
  role_guidance: "Adds role-specific job-search guidance.",
  regional_guidance: "Adds regional job-search guidance.",
  source_support: "Adds reviewed job-source support.",
  review_rubric: "Adds a local review rubric.",
  synthetic_product_evaluation: "Tests local product behavior with synthetic data.",
  job_search_template: "Adds a static job-search template.",
  operating_system_helper: "Adds a reviewed operating-system helper.",
};

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ")
    .replace(/\bAi\b/g, "AI")
    .replace(/\bOs\b/g, "OS");
}

function list(values: string[], empty: string): string {
  return values.length > 0 ? values.map(label).join(", ") : empty;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  return `${new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 1,
  }).format(bytes / 1024)} KB`;
}

function formatSelfTestedAt(value: string | null): string {
  if (!value) return "Not verified";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

interface PackManagementModalProps {
  onClose: () => void;
}

export function PackManagementModal({ onClose }: PackManagementModalProps) {
  const [packs, setPacks] = useState<PackManagementReview[] | null>(null);
  const [error, setError] = useState(false);
  const load = useCallback(async () => {
    setError(false);
    try {
      setPacks(
        parsePackManagementReviews(
          await invoke<unknown>("list_pack_management"),
        ),
      );
    } catch {
      setPacks(null);
      setError(true);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <Modal
      isOpen
      onClose={onClose}
      title="Packs"
      description="View signed additions, their permissions, and local status."
      size="wide"
      closeButtonLabel="Close pack view"
    >
      <div className="mb-4 flex items-start justify-between gap-4">
        <p className="text-sm text-surface-600 dark:text-surface-300">
          This view is read-only. Pack files stay local, signed permissions
          cannot expand, and packs cannot submit applications.
        </p>
        <Button variant="secondary" onClick={() => void load()}>
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="rounded-lg border border-danger/30 bg-danger/5 p-4">
          <p role="alert" className="text-sm text-danger">
            Pack information could not be loaded.
          </p>
          <Button
            variant="secondary"
            className="mt-3"
            onClick={() => void load()}
          >
            Try again
          </Button>
        </div>
      ) : null}

      {!packs && !error ? (
        <p className="text-sm text-surface-600 dark:text-surface-300">
          Checking installed packs...
        </p>
      ) : null}

      {packs?.length === 0 ? (
        <p className="rounded-lg border border-surface-200 bg-surface-50 p-4 text-sm text-surface-600 dark:border-surface-700 dark:bg-surface-800 dark:text-surface-300">
          No packs are installed.
        </p>
      ) : null}

      {packs && packs.length > 0 ? (
        <div className="space-y-4">
          {packs.map((pack) => (
            <PackCard
              key={`${pack.publisherKeyId}:${pack.packId}`}
              pack={pack}
            />
          ))}
        </div>
      ) : null}
    </Modal>
  );
}

function PackCard({ pack }: { pack: PackManagementReview }) {
  const release = pack.currentRelease;
  return (
    <article
      aria-label={`${release.publisherName} ${pack.packId}`}
      className="rounded-lg border border-surface-200 bg-white p-4 dark:border-surface-700 dark:bg-surface-900/30"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <h3 className="font-medium text-surface-900 dark:text-surface-100">
            {release.publisherName}
          </h3>
          <p className="break-all text-xs text-surface-500 dark:text-surface-400">
            {pack.packId}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <span
            aria-label={`Pack status: ${STATE_LABELS[pack.state]}`}
            className={`rounded-full px-2.5 py-1 text-xs font-medium ${STATE_TONES[pack.state]}`}
          >
            {STATE_LABELS[pack.state]}
          </span>
          {pack.updateAvailable ? (
            <span className="rounded-full bg-sentinel-100 px-2.5 py-1 text-xs font-medium text-sentinel-700 dark:bg-sentinel-900/30 dark:text-sentinel-300">
              Update available
            </span>
          ) : null}
        </div>
      </div>

      {release.quarantineReason ? (
        <p className="mt-3 text-sm text-danger">
          {QUARANTINE_COPY[release.quarantineReason]}
        </p>
      ) : null}
      {pack.cleanupPending ? (
        <p className="mt-2 text-sm text-warning">
          Artifact cleanup needs another attempt
        </p>
      ) : null}

      <section aria-label="Current release review">
        <ReleaseReviewFacts
          publisherKeyId={pack.publisherKeyId}
          publisherPublicKeySha256={pack.publisherPublicKeySha256}
          release={release}
        />
      </section>

      <details className="mt-4">
        <summary className="cursor-pointer text-sm font-medium text-sentinel-700 dark:text-sentinel-300">
          Release history
        </summary>
        <ol className="mt-2 space-y-2">
          {pack.releases.map((history) => (
            <li
              key={history.releaseSequence}
              aria-label={`Release ${history.releaseSequence}`}
              className="rounded-md border border-surface-200 p-2 text-xs text-surface-600 dark:border-surface-700 dark:text-surface-300"
            >
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">Release {history.releaseSequence}</span>
                <span>{history.packVersion}</span>
                <span>{label(history.state)}</span>
                {history.isActive ? <span>Active</span> : null}
                {history.isRollback ? <span>Rollback</span> : null}
                {history.isHighWater ? <span>Newest verified</span> : null}
                {history.quarantineReason ? (
                  <span>{QUARANTINE_COPY[history.quarantineReason]}</span>
                ) : null}
                {history.artifactCleanupPending ? <span>Cleanup pending</span> : null}
              </div>
              <details className="mt-2">
                <summary className="cursor-pointer font-medium text-sentinel-700 dark:text-sentinel-300">
                  Review signed permissions
                </summary>
                <ReleaseReviewFacts
                  publisherKeyId={pack.publisherKeyId}
                  publisherPublicKeySha256={pack.publisherPublicKeySha256}
                  release={history}
                />
              </details>
            </li>
          ))}
        </ol>
      </details>
    </article>
  );
}

function ReleaseReviewFacts({
  publisherKeyId,
  publisherPublicKeySha256,
  release,
}: {
  publisherKeyId: string;
  publisherPublicKeySha256: string;
  release: PackReleaseReview;
}) {
  return (
    <>
      <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
        <Fact label="Version" value={release.packVersion} />
        <Fact label="Pack type" value={label(release.packType)} />
        <Fact label="Execution" value={label(release.executionClass)} />
        <Fact label="Publisher key" value={publisherKeyId} />
        <Fact
          label="Publisher key fingerprint (SHA-256)"
          value={publisherPublicKeySha256}
        />
        <Fact label="License" value={release.license} />
        <Fact
          label="App compatibility"
          value={`${release.minimumAppVersion} to ${release.maximumAppVersion}`}
        />
        <Fact label="Signed size" value={formatBytes(release.payloadBytes)} />
        <Fact label="What it helps with" value={PURPOSE_COPY[release.purpose]} />
        <Fact
          label="Model download"
          value={release.modelDownloadRequired ? "Required" : "Not required"}
        />
        <Fact
          label="Last successful self-test"
          value={formatSelfTestedAt(release.lastSelfTestedAt)}
        />
        <Fact
          label="Privacy"
          value={list(release.privacyLabels, "No privacy label")}
        />
      </dl>
      <div className="mt-4 rounded-md bg-surface-50 p-3 text-sm dark:bg-surface-800">
        <p className="font-medium text-surface-800 dark:text-surface-100">
          Reviewed fixtures
        </p>
        <p className="mt-1 text-surface-600 dark:text-surface-300">
          {release.fixtureSummary}
        </p>
        <p className="mt-2 text-xs text-surface-500 dark:text-surface-400">
          Data: {list(release.allowedDataCategories, "No data access")}
        </p>
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          Tasks: {list(release.allowedTaskKinds, "No runnable tasks")}
        </p>
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          Actions: {list(release.allowedActions, "No actions")}
        </p>
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          Approval: {list(release.approvalGates, "No execution approval gate")}
        </p>
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          Gateway policy: {release.gatewayPolicyId ?? "Not used"}
        </p>
        <p className="mt-1 text-xs text-surface-500 dark:text-surface-400">
          {release.usesExternalAi
            ? `Outside AI destinations: ${
                release.externalDestinations.join(", ") ||
                "No destination declared"
              }`
            : "Does not use outside AI"}
        </p>
      </div>
    </>
  );
}

function Fact({ label: factLabel, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs font-medium text-surface-500 dark:text-surface-400">
        {factLabel}
      </dt>
      <dd className="mt-0.5 break-words text-surface-800 [overflow-wrap:anywhere] dark:text-surface-100">
        {value}
      </dd>
    </div>
  );
}
