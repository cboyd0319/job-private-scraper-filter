/** Renders a selected saved job's local employer dossier without network access. */

import { memo, useEffect, useRef, useState } from "react";
import { invoke } from "../../platform/tauri";
import { Card } from "../../ui/Card";
import { removeStorageValue } from "../../shared/browserStorage";
import type { CompanyResearchPanelProps } from "../../shared/companyResearch";
import { EmployerDossierSection } from "../../shared/EmployerDossierSection";
import { decodeEmployerDossier, type EmployerDossier } from "../../shared/employerDossier";

type OpportunityCaseProjection = {
  job: { job_hash: string };
  employer_dossier: unknown;
};

export const CompanyResearchPanel = memo(function CompanyResearchPanel({
  companyName,
  jobHash,
  onClose,
}: CompanyResearchPanelProps) {
  const [dossier, setDossier] = useState<EmployerDossier | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);
  const [retry, setRetry] = useState(0);
  const requestEpoch = useRef(0);

  useEffect(() => {
    removeStorageValue("local", "jobsentinel_company_cache");
    const request = ++requestEpoch.current;
    if (!jobHash) {
      setDossier(null);
      setIsLoading(false);
      setHasError(false);
      return;
    }

    setDossier(null);
    setIsLoading(true);
    setHasError(false);
    void invoke<OpportunityCaseProjection>("open_opportunity_case", { jobHash })
      .then((projection) => {
        if (request !== requestEpoch.current || projection.job.job_hash !== jobHash) return;
        const decoded = decodeEmployerDossier(projection.employer_dossier);
        if (!decoded) throw new Error("invalid employer dossier");
        setDossier(decoded);
      })
      .catch(() => {
        if (request === requestEpoch.current) setHasError(true);
      })
      .finally(() => {
        if (request === requestEpoch.current) setIsLoading(false);
      });
  }, [jobHash, retry]);

  return (
    <Card className="w-full max-w-md">
      <div className="flex items-center justify-between border-b border-surface-200 p-4 dark:border-surface-700">
        <div>
          <h3 className="font-medium text-surface-900 dark:text-white">{companyName}</h3>
          <p className="text-xs text-surface-500">Company research</p>
        </div>
        {onClose && (
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 text-surface-400 hover:text-surface-600 dark:hover:text-surface-300"
            aria-label="Close"
          >
            ×
          </button>
        )}
      </div>
      <div className="p-4 text-sm text-surface-700 dark:text-surface-300">
        {!jobHash && <p>Choose a saved job to view its local employer dossier.</p>}
        {isLoading && <p role="status">Loading local employer dossier…</p>}
        {hasError && !isLoading && (
          <div className="space-y-3" role="alert">
            <p>Could not load the local employer dossier.</p>
            <button type="button" className="underline" onClick={() => setRetry((current) => current + 1)}>
              Retry
            </button>
          </div>
        )}
        {dossier && !isLoading && !hasError && <EmployerDossierSection dossier={dossier} />}
      </div>
    </Card>
  );
});
