import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Button } from "../../../ui/Button";
import { Modal, ModalFooter } from "../../../ui/Modal";
import { hasSensitiveData } from "../../../shared/externalAi/externalAiPayloadPolicy";
import type { ExternalAiRequest } from "../../../shared/externalAi/externalAiTypes";

interface ExternalAiReviewDialogProps {
  isOpen: boolean;
  providerLabel: string;
  request: ExternalAiRequest | null;
  allowSensitivePayloads: boolean;
  onCancel: () => void;
  onApprove: (request: ExternalAiRequest) => void | Promise<void>;
}

interface ParsedReviewDetails {
  payload: Record<string, unknown> | null;
  error: string | null;
}

function stringifyReviewPayload(request: ExternalAiRequest | null): string {
  if (!request) return "{}";
  return JSON.stringify(request.redactedPayload ?? request.payload, null, 2);
}

function parseReviewDetails(value: string): ParsedReviewDetails {
  try {
    const parsed = JSON.parse(value) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return {
        payload: null,
        error: "Use the same structured format shown here.",
      };
    }

    return {
      payload: parsed as Record<string, unknown>,
      error: null,
    };
  } catch {
    return {
      payload: null,
      error: "Fix the formatting before sending.",
    };
  }
}

function formatList(values: readonly string[]): string {
  return values.length ? values.join(", ") : "None";
}

function changesStoredValue(
  request: ExternalAiRequest | null,
  payload: Record<string, unknown> | null,
): boolean {
  if (!request || !payload || Object.keys(payload).length === 0) return true;
  const original = request.redactedPayload ?? request.payload;
  return Object.entries(payload).some(
    ([key, value]) =>
      !(key in original) ||
      JSON.stringify(value) !== JSON.stringify(original[key]),
  );
}

export function ExternalAiReviewDialog({
  isOpen,
  providerLabel,
  request,
  allowSensitivePayloads,
  onCancel,
  onApprove,
}: ExternalAiReviewDialogProps) {
  const [reviewText, setReviewText] = useState(() =>
    stringifyReviewPayload(request),
  );
  const [sensitiveConfirmed, setSensitiveConfirmed] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [retryBlocked, setRetryBlocked] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const initialReviewText = useMemo(() => stringifyReviewPayload(request), [
    request,
  ]);
  const parsedDetails = useMemo(
    () => parseReviewDetails(reviewText),
    [reviewText],
  );
  const includesSensitiveDetails = Boolean(
    request &&
      parsedDetails.payload &&
      hasSensitiveData(request, parsedDetails.payload),
  );
  const sensitiveBlocked = includesSensitiveDetails && !allowSensitivePayloads;
  const needsSensitiveConfirmation =
    includesSensitiveDetails && allowSensitivePayloads && !sensitiveConfirmed;
  const hasChangedStoredValue = changesStoredValue(
    request,
    parsedDetails.payload,
  );
  const sendDisabled =
    !request ||
    Boolean(parsedDetails.error) ||
    hasChangedStoredValue ||
    sensitiveBlocked ||
    needsSensitiveConfirmation ||
    retryBlocked ||
    isSubmitting;

  useEffect(() => {
    if (!isOpen) return;

    setReviewText(initialReviewText);
    setSensitiveConfirmed(false);
    setSubmitError(null);
    setRetryBlocked(false);
    setIsSubmitting(false);
  }, [initialReviewText, isOpen]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!request || !parsedDetails.payload || sendDisabled) return;

    setIsSubmitting(true);
    setSubmitError(null);

    try {
      await onApprove({
        ...request,
        redactedPayload: parsedDetails.payload,
        previewShown: true,
        userApproved: true,
        explicitlyIncludedSensitiveData:
          includesSensitiveDetails ||
          request.explicitlyIncludedSensitiveData ||
          undefined,
      });
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "JobSentinel could not send these details.";
      setSubmitError(message);
      if (/do not retry/i.test(message)) setRetryBlocked(true);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen && Boolean(request)}
      onClose={onCancel}
      title="Review Outside AI Details"
      description={`JobSentinel will use ${providerLabel} only with the details shown here.`}
      size="xl"
      closeOnOverlayClick={!isSubmitting}
    >
      {request && (
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div className="grid gap-3 rounded-lg border border-surface-200 bg-surface-50 p-3 text-sm text-surface-700 dark:border-surface-700 dark:bg-surface-900 dark:text-surface-200 md:grid-cols-3">
            <div>
              <span className="block text-xs font-medium uppercase tracking-wide text-surface-500 dark:text-surface-400">
                Feature
              </span>
              <span className="mt-1 block break-words">{request.feature}</span>
            </div>
            <div>
              <span className="block text-xs font-medium uppercase tracking-wide text-surface-500 dark:text-surface-400">
                Labels
              </span>
              <span className="mt-1 block break-words">
                {formatList(request.labels)}
              </span>
            </div>
            <div>
              <span className="block text-xs font-medium uppercase tracking-wide text-surface-500 dark:text-surface-400">
                Details
              </span>
              <span className="mt-1 block break-words">
                {formatList(request.dataCategories)}
              </span>
            </div>
          </div>

          <label className="block text-sm">
            <span className="mb-1 block font-medium text-surface-800 dark:text-surface-100">
              Details to send
            </span>
            <textarea
              value={reviewText}
              onChange={(event) => {
                setReviewText(event.target.value);
                setSubmitError(null);
              }}
              rows={12}
              spellCheck={false}
              className="w-full rounded-lg border border-surface-300 bg-white px-3 py-2 font-mono text-sm text-surface-900 focus:border-sentinel-500 focus:ring-1 focus:ring-sentinel-500 dark:border-surface-600 dark:bg-surface-950 dark:text-surface-100"
            />
          </label>

          {parsedDetails.error && (
            <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
              {parsedDetails.error}
            </p>
          )}

          {!parsedDetails.error && hasChangedStoredValue && (
            <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
              Remove fields you do not want to share. Adding or rewriting
              values is blocked because only stored public job fields can be
              sent.
            </p>
          )}

          {includesSensitiveDetails && (
            <div className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:border-amber-900 dark:bg-amber-950 dark:text-amber-100">
              <p className="font-medium">These details include private data.</p>
              {allowSensitivePayloads ? (
                <label className="mt-2 flex items-start gap-2">
                  <input
                    type="checkbox"
                    checked={sensitiveConfirmed}
                    onChange={(event) =>
                      setSensitiveConfirmed(event.target.checked)
                    }
                    className="mt-1 rounded border-amber-300 text-amber-700 focus:ring-amber-600"
                  />
                  <span>
                    I reviewed these private details and choose to include them
                    in this one request.
                  </span>
                </label>
              ) : (
                <p className="mt-1">
                  Private details are off in Settings. Remove them here or turn
                  on private details after review.
                </p>
              )}
            </div>
          )}

          {submitError && (
            <p className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
              {submitError}
            </p>
          )}

          <ModalFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={onCancel}
            >
              Cancel
            </Button>
            <Button type="submit" loading={isSubmitting} disabled={sendDisabled}>
              Send Reviewed Details
            </Button>
          </ModalFooter>
        </form>
      )}
    </Modal>
  );
}
