/** Reviews and imports one user-selected public job page through the governed source boundary. */

import { useState, useCallback, useEffect } from "react";
import { Modal, ModalFooter } from "../../../ui/Modal";
import { Button } from "../../../ui/Button";
import { useToast } from "../../../shared/toast/useToast";
import { invalidateCacheByCommand, invoke } from "../../../platform/tauri";
import {
  isPolicyBlockedAutomationUrl,
  isRestrictedJobSourceUrl,
  RESTRICTED_JOB_SOURCE_WARNING,
} from "../../../shared/restrictedSourceTaxonomy";
import { isValidJobUrl } from "../jobUrlValidation";
import {
  formatImportDate,
  formatMissingDetails,
  fullJobLinkMessage,
  getSafeJobImportError,
  policyBlockedPastedLinkMessage,
  publicJobLinkMessage,
  secureJobLinkMessage,
  type JobImportPreview,
  type JobImportResult,
} from "./jobImportModel";
import { SmartPasteFlow } from "./SmartPasteFlow";

interface JobImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onImportSuccess?: () => void;
}

export function JobImportModal({
  isOpen,
  onClose,
  onImportSuccess,
}: JobImportModalProps) {
  const [url, setUrl] = useState("");
  const [preview, setPreview] = useState<JobImportPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restrictedSourceAcknowledged, setRestrictedSourceAcknowledged] =
    useState(false);
  const [smartPaste, setSmartPaste] = useState(false);

  const toast = useToast();
  const policyBlockedAutomation = isPolicyBlockedAutomationUrl(url.trim());
  const needsRestrictedSourceGate =
    !policyBlockedAutomation && isRestrictedJobSourceUrl(url.trim());

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setUrl("");
      setPreview(null);
      setError(null);
      setLoading(false);
      setImporting(false);
      setRestrictedSourceAcknowledged(false);
      setSmartPaste(false);
    }
  }, [isOpen]);

  // Preview the job import
  const handlePreview = useCallback(async () => {
    if (!url.trim()) {
      setError("Add a job link from your browser address bar.");
      return;
    }

    if (policyBlockedAutomation) {
      setError(policyBlockedPastedLinkMessage);
      return;
    }

    if (needsRestrictedSourceGate && !restrictedSourceAcknowledged) {
      setError(
        "Review the restricted-source warning and check the box if you want to continue.",
      );
      return;
    }

    const trimmedUrl = url.trim();

    let parsedUrl: URL;
    try {
      parsedUrl = new URL(trimmedUrl);
    } catch {
      setError(fullJobLinkMessage);
      return;
    }

    if (parsedUrl.protocol !== "http:" && parsedUrl.protocol !== "https:") {
      setError(publicJobLinkMessage);
      return;
    }

    if (parsedUrl.protocol === "http:") {
      const httpsEquivalent = new URL(parsedUrl);
      httpsEquivalent.protocol = "https:";
      setError(
        isValidJobUrl(httpsEquivalent.toString())
          ? secureJobLinkMessage
          : publicJobLinkMessage,
      );
      return;
    }

    if (!isValidJobUrl(trimmedUrl)) {
      setError(publicJobLinkMessage);
      return;
    }

    setLoading(true);
    setError(null);
    setPreview(null);

    try {
      const result = await invoke<JobImportPreview>("preview_job_import", {
        url: trimmedUrl,
      });
      setPreview(result);

      if (result.already_exists) {
        toast.info(
          "Job already saved",
          "This job is already in your saved jobs",
        );
      }
    } catch (err) {
      const safeError = getSafeJobImportError(err);
      setError(safeError.message);
      toast.error("Could not check job link", safeError.action);
    } finally {
      setLoading(false);
    }
  }, [
    url,
    toast,
    policyBlockedAutomation,
    needsRestrictedSourceGate,
    restrictedSourceAcknowledged,
  ]);

  // Import the job
  const handleImport = useCallback(async () => {
    if (!preview?.import_id || preview.already_exists) {
      return;
    }

    setImporting(true);
    setError(null);

    try {
      await invoke<JobImportResult>("confirm_job_import", {
        importId: preview.import_id,
      });

      invalidateCacheByCommand("get_recent_jobs");
      invalidateCacheByCommand("get_statistics");
      toast.success("Job saved", `Saved "${preview.title}"`);

      // Call success callback
      onImportSuccess?.();

      // Close modal
      onClose();
    } catch (err) {
      const safeError = getSafeJobImportError(err);
      setError(safeError.message);
      toast.error("Could not save job", safeError.action);
    } finally {
      setImporting(false);
    }
  }, [preview, toast, onImportSuccess, onClose]);

  // Handle Enter key in URL input
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" && !loading && !preview) {
        e.preventDefault();
        handlePreview();
      }
    },
    [loading, preview, handlePreview],
  );

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={smartPaste ? "Smart Paste Job" : "Import Job from Link"}
      size="lg"
      aria-labelledby="import-job-title"
      aria-describedby="import-job-description"
    >
      {smartPaste ? (
        <SmartPasteFlow
          onClose={onClose}
          onUseLink={() => setSmartPaste(false)}
          onImportSuccess={onImportSuccess}
        />
      ) : (
        <>
      <div className="space-y-4">
        {/* Description */}
        <p
          id="import-job-description"
          className="text-sm text-gray-600 dark:text-gray-400"
        >
          Paste a link to an individual job page from a job board or company
          career site. JobSentinel will check for job details you can review
          before saving.
        </p>

        <Button onClick={() => setSmartPaste(true)} variant="secondary">
          Paste Job Details
        </Button>

        {/* URL Input */}
        <div>
          <label
            htmlFor="job-url"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Job link
          </label>
          <input
            id="job-url"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="https://example.com/jobs/office-manager"
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white"
            disabled={loading || importing}
            autoFocus
          />
        </div>

        {policyBlockedAutomation ? (
          <div
            role="alert"
            className="rounded-lg border-2 border-amber-300 bg-amber-50 p-4 text-sm leading-6 text-amber-900 dark:border-amber-700 dark:bg-amber-900/25 dark:text-amber-100"
          >
            {policyBlockedPastedLinkMessage}
          </div>
        ) : needsRestrictedSourceGate ? (
          <div className="rounded-lg border-2 border-amber-300 bg-amber-50 p-4 dark:border-amber-700 dark:bg-amber-900/25">
            <p className="mb-2 text-base font-semibold text-amber-900 dark:text-amber-100">
              Restricted source warning
            </p>
            <p className="text-sm leading-6 text-amber-800 dark:text-amber-200">
              {RESTRICTED_JOB_SOURCE_WARNING} Import only individual job pages
              you choose, and verify the saved details before using them.
            </p>
            <label className="mt-4 flex items-start gap-3 text-sm font-medium text-amber-900 dark:text-amber-100">
              <input
                type="checkbox"
                className="mt-0.5 h-5 w-5 rounded border-amber-300 text-amber-700 focus:ring-amber-500"
                checked={restrictedSourceAcknowledged}
                onChange={(event) =>
                  setRestrictedSourceAcknowledged(event.target.checked)
                }
              />
              <span>
                I understand this risk and want JobSentinel to check this job
                link.
              </span>
            </label>
          </div>
        ) : null}

        {/* Error Display */}
        {error && (
          <div
            role="alert"
            aria-live="assertive"
            className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg"
          >
            <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
          </div>
        )}

        {/* Preview Button */}
        {!preview && (
          <div className="flex justify-end">
            <Button
              onClick={handlePreview}
              disabled={
                loading ||
                importing ||
                policyBlockedAutomation ||
                (needsRestrictedSourceGate && !restrictedSourceAcknowledged)
              }
              loading={loading}
              variant="primary"
            >
              {loading ? "Checking job link..." : "Check Job Link"}
            </Button>
          </div>
        )}

        {/* Job Preview */}
        {preview && (
          <div className="mt-4 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-gray-50 dark:bg-gray-800/50">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">
              Job Details
            </h3>

            <div className="space-y-3">
              {/* Title and Company */}
              <div>
                <div className="text-xl font-bold text-gray-900 dark:text-white">
                  {preview.title}
                </div>
                <div className="text-lg text-gray-700 dark:text-gray-300">
                  {preview.company}
                </div>
              </div>

              {/* Location & Remote */}
              {(preview.location || preview.remote) && (
                <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                  {preview.location && <span>{preview.location}</span>}
                  {preview.remote && (
                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300">
                      Remote
                    </span>
                  )}
                </div>
              )}

              {/* Employment Types */}
              {preview.employment_types.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {preview.employment_types.map((type) => (
                    <span
                      key={type}
                      className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300"
                    >
                      {type}
                    </span>
                  ))}
                </div>
              )}

              {/* Salary */}
              {preview.salary ? (
                <div className="text-sm text-gray-700 dark:text-gray-300">
                  <span className="font-medium">Listed pay:</span>{" "}
                  {preview.salary}
                </div>
              ) : (
                <div className="text-sm text-gray-700 dark:text-gray-300">
                  <span className="font-medium">Listed pay not shown.</span>{" "}
                  Verify pay before tailoring.
                </div>
              )}

              {/* Description Preview */}
              {preview.description_preview && (
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  <span className="font-medium text-gray-700 dark:text-gray-300">
                    Description:
                  </span>{" "}
                  {preview.description_preview}
                </div>
              )}

              {/* Date Posted */}
              {preview.date_posted && (
                <div className="text-xs text-gray-500 dark:text-gray-500">
                  Posted: {formatImportDate(preview.date_posted)}
                </div>
              )}

              {/* Closing Date */}
              {preview.valid_through && (
                <div className="text-xs text-gray-500 dark:text-gray-500">
                  Closing date: {formatImportDate(preview.valid_through)}
                </div>
              )}

              {/* Missing details warning */}
              {preview.missing_fields.length > 0 && (
                <div className="p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded">
                  <p className="flex items-start gap-1.5 text-xs text-yellow-800 dark:text-yellow-200">
                    <WarningIcon className="mt-0.5 h-3.5 w-3.5 flex-shrink-0" />
                    <span>
                      Details to check:{" "}
                      {formatMissingDetails(preview.missing_fields)}.{" "}
                      {preview.import_id
                        ? "You can still save this job and verify the missing details before tailoring."
                        : "Check a different job page before saving."}
                    </span>
                  </p>
                </div>
              )}

              {/* Already Exists Warning */}
              {preview.already_exists && (
                <div className="p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded">
                  <p className="flex items-start gap-1.5 text-xs text-blue-800 dark:text-blue-200">
                    <InfoIcon className="mt-0.5 h-3.5 w-3.5 flex-shrink-0" />
                    <span>This job is already in your saved jobs</span>
                  </p>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Modal Footer */}
      <ModalFooter>
        {preview?.import_id && !preview.already_exists && (
          <Button
            onClick={() => setPreview(null)}
            variant="secondary"
            disabled={importing}
          >
            Change Link
          </Button>
        )}
        <Button onClick={onClose} variant="secondary" disabled={importing}>
          Cancel
        </Button>
        {preview?.import_id && !preview.already_exists && (
          <Button
            onClick={handleImport}
            disabled={importing}
            loading={importing}
            variant="primary"
          >
            {importing ? "Saving..." : "Save Job"}
          </Button>
        )}
      </ModalFooter>
        </>
      )}
    </Modal>
  );
}

function WarningIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 9v2m0 4h.01m-6.94 4h13.88c1.54 0 2.5-1.67 1.73-3L13.73 4c-.77-1.33-2.69-1.33-3.46 0L3.33 16c-.77 1.33.19 3 1.73 3z"
      />
    </svg>
  );
}

function InfoIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M12 16v-4m0-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
      />
    </svg>
  );
}
