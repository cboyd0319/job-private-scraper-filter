/** Displays exact saved-match evidence and its reviewed local Evidence Reviewer action. */

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { safeInvoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import { Modal, ModalFooter } from "../../../ui/Modal";
import type { MatchResult } from "./resumePageModel";
import { PackEvidenceReviewer } from "./PackEvidenceReviewer";

interface SavedMatchDebuggerRequirement {
  requirement: string;
  importance: string;
  match_state: string;
  hard_constraint: boolean;
  evidence: Array<{
    evidence_id: string;
    confirmed: boolean;
  }>;
  why_not: string | null;
  blocking: boolean;
}

interface SavedMatchDebugger {
  debugger_id: string;
  requirements: SavedMatchDebuggerRequirement[];
}

interface SavedMatchEvidencePacket {
  claim_id: string;
  reviewed_text: string;
}

interface MatchDebuggerInteraction {
  id: number;
  matchKey: string | null;
}

interface MatchDebuggerModalProps {
  isOpen: boolean;
  match: MatchResult | null;
  onClose: () => void;
}

const labelFor = (value: string) =>
  value.replace(/_/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();

const sentenceLabelFor = (value: string) => {
  const label = labelFor(value);
  return `${label[0]?.toUpperCase() ?? ""}${label.slice(1)}`;
};

export function MatchDebuggerModal({ isOpen, match, onClose }: MatchDebuggerModalProps) {
  const [debuggerView, setDebuggerView] = useState<SavedMatchDebugger | null>(null);
  const [loading, setLoading] = useState(false);
  const [confirmingEvidenceId, setConfirmingEvidenceId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [packetClaims, setPacketClaims] = useState<SavedMatchEvidencePacket[]>([]);
  const [packetLoading, setPacketLoading] = useState(false);
  const [packetSaving, setPacketSaving] = useState(false);
  const [packetError, setPacketError] = useState<string | null>(null);
  const [reviewedText, setReviewedText] = useState("");
  const [selectedEvidenceIds, setSelectedEvidenceIds] = useState<string[]>([]);
  const requestId = useRef(0);
  const packetRequestId = useRef(0);
  const matchKey = match ? `${match.resume_id}:${match.job_hash}` : null;
  const activeInteraction = useRef<MatchDebuggerInteraction>({ id: 0, matchKey: null });
  const isCurrentInteraction = useCallback(
    (interaction: MatchDebuggerInteraction) =>
      activeInteraction.current.id === interaction.id && activeInteraction.current.matchKey === interaction.matchKey,
    [],
  );

  useLayoutEffect(() => {
    activeInteraction.current = {
      id: activeInteraction.current.id + 1,
      matchKey: isOpen ? matchKey : null,
    };
    requestId.current += 1;
    packetRequestId.current += 1;
    setDebuggerView(null);
    setLoading(false);
    setConfirmingEvidenceId(null);
    setError(null);
    setPacketClaims([]);
    setPacketLoading(false);
    setPacketSaving(false);
    setPacketError(null);
    setReviewedText("");
    setSelectedEvidenceIds([]);
  }, [isOpen, matchKey]);

  const loadDebugger = useCallback(async (interaction = activeInteraction.current) => {
    if (!match || !isCurrentInteraction(interaction)) return;
    const request = ++requestId.current;
    const isCurrent = () => requestId.current === request && isCurrentInteraction(interaction);
    setLoading(true);
    setError(null);
    try {
      const debuggerResponse = await safeInvoke<SavedMatchDebugger>(
        "get_saved_match_debugger",
        { jobHash: match.job_hash, resumeId: match.resume_id },
        { logContext: "Load saved match debugger" },
      );
      if (isCurrent()) setDebuggerView(debuggerResponse);
      if (isCurrent()) {
        const confirmedEvidenceIds = debuggerResponse.requirements
          .flatMap((requirement) => requirement.evidence)
          .filter((evidence) => evidence.confirmed)
          .map((evidence) => evidence.evidence_id);
        setSelectedEvidenceIds((selected) =>
          selected.filter((evidenceId) => confirmedEvidenceIds.includes(evidenceId)),
        );
      }
    } catch {
      if (isCurrent()) {
        setDebuggerView(null);
        setError("Could not load saved match evidence. Close this review and try again.");
      }
    } finally {
      if (isCurrent()) setLoading(false);
    }
  }, [isCurrentInteraction, match]);

  const loadPacketClaims = useCallback(async (interaction = activeInteraction.current) => {
    if (!match || !isCurrentInteraction(interaction)) return;
    const request = ++packetRequestId.current;
    const isCurrent = () => packetRequestId.current === request && isCurrentInteraction(interaction);
    setPacketLoading(true);
    setPacketError(null);
    try {
      const claims = await safeInvoke<SavedMatchEvidencePacket[]>(
        "list_saved_match_evidence_packets",
        { jobHash: match.job_hash, resumeId: match.resume_id },
        { logContext: "Load saved match reviewed claims" },
      );
      if (isCurrent()) setPacketClaims(claims);
    } catch {
      if (isCurrent()) {
        setPacketClaims([]);
        setPacketError("Could not load reviewed claims. Try opening this review again.");
      }
    } finally {
      if (isCurrent()) setPacketLoading(false);
    }
  }, [isCurrentInteraction, match]);

  useEffect(() => {
    if (!isOpen) return;
    const interaction = activeInteraction.current;
    void loadDebugger(interaction);
    void loadPacketClaims(interaction);
  }, [isOpen, loadDebugger, loadPacketClaims]);

  useEffect(() => () => {
    activeInteraction.current = { id: activeInteraction.current.id + 1, matchKey: null };
    requestId.current += 1;
    packetRequestId.current += 1;
  }, []);

  const confirmEvidence = async (evidenceId: string) => {
    const interaction = activeInteraction.current;
    const currentMatch = match;
    if (!currentMatch || !debuggerView || !isCurrentInteraction(interaction)) return;
    setConfirmingEvidenceId(evidenceId);
    setError(null);
    try {
      await safeInvoke<boolean>(
        "confirm_saved_match_evidence",
        {
          jobHash: currentMatch.job_hash,
          resumeId: currentMatch.resume_id,
          debuggerId: debuggerView.debugger_id,
          evidenceId,
        },
        { logContext: "Confirm saved match evidence" },
      );
      if (!isCurrentInteraction(interaction)) return;
      await loadDebugger(interaction);
    } catch {
      if (isCurrentInteraction(interaction)) {
        setError("Could not confirm that evidence. Refresh this review and try again.");
      }
    } finally {
      if (isCurrentInteraction(interaction)) setConfirmingEvidenceId(null);
    }
  };

  const toggleEvidence = (evidenceId: string) => {
    setSelectedEvidenceIds((selected) =>
      selected.includes(evidenceId)
        ? selected.filter((value) => value !== evidenceId)
        : [...selected, evidenceId],
    );
  };

  const savePacketClaim = async () => {
    const interaction = activeInteraction.current;
    const currentMatch = match;
    const currentReviewedText = reviewedText.trim();
    const currentEvidenceIds = [...selectedEvidenceIds];
    if (!currentMatch || !currentReviewedText || currentEvidenceIds.length === 0 || !isCurrentInteraction(interaction)) return;
    setPacketSaving(true);
    setPacketError(null);
    try {
      await safeInvoke<SavedMatchEvidencePacket>(
        "save_saved_match_evidence_packet",
        {
          jobHash: currentMatch.job_hash,
          resumeId: currentMatch.resume_id,
          reviewedText: currentReviewedText,
          evidenceIds: currentEvidenceIds,
        },
        { logContext: "Save saved match reviewed claim" },
      );
      if (!isCurrentInteraction(interaction)) return;
      setReviewedText("");
      setSelectedEvidenceIds([]);
      await loadPacketClaims(interaction);
    } catch {
      if (isCurrentInteraction(interaction)) {
        setPacketError("Could not save this reviewed claim. Confirm current evidence and try again.");
      }
    } finally {
      if (isCurrentInteraction(interaction)) setPacketSaving(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Match Debugger"
      description="Review the saved match requirement by requirement. Evidence stays on this device."
      size="lg"
    >
      {loading && <p className="text-sm text-surface-600 dark:text-surface-300">Loading saved evidence…</p>}
      {error && <p role="alert" className="text-sm text-red-700 dark:text-red-300">{error}</p>}
      {!loading && !error && debuggerView?.requirements.length === 0 && (
        <p className="text-sm text-surface-600 dark:text-surface-300">
          No requirement evidence is available for this saved match.
        </p>
      )}
      {!loading && debuggerView && (
        <div className="space-y-3">
          {debuggerView.requirements.map((requirement, index) => {
            return (
              <section
                key={`${requirement.requirement}-${index}`}
                className="rounded-lg border border-surface-200 p-3 dark:border-surface-700"
              >
                <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                  <div className="min-w-0">
                    <h3 className="break-words font-medium text-surface-900 dark:text-white">
                      {requirement.requirement}
                    </h3>
                    <p className="mt-1 text-sm text-surface-600 dark:text-surface-300">
                      <span>{requirement.importance} importance</span>
                      <span aria-hidden="true"> · </span>
                      <span>{sentenceLabelFor(requirement.match_state)} evidence</span>
                    </p>
                  </div>
                  {requirement.blocking && (
                    <span className="w-fit rounded bg-red-50 px-2 py-1 text-xs font-medium text-red-800 dark:bg-red-900/30 dark:text-red-200">
                      Hard blocker
                    </span>
                  )}
                </div>
                {requirement.why_not && (
                  <p className="mt-2 text-sm text-surface-700 dark:text-surface-200">
                    Why not: {labelFor(requirement.why_not)}
                  </p>
                )}
                {requirement.evidence.length === 0 ? (
                  <p className="mt-3 text-sm text-surface-600 dark:text-surface-300">
                    No supporting evidence available
                  </p>
                ) : (
                  <div className="mt-3 space-y-2">
                    {requirement.evidence.map((evidence, evidenceIndex) => {
                      const confirming = confirmingEvidenceId === evidence.evidence_id;
                      const anotherEvidenceIsConfirming = confirmingEvidenceId !== null && !confirming;
                      return (
                        <div
                          key={evidence.evidence_id}
                          className="flex flex-wrap items-center justify-between gap-2"
                        >
                          <span className="text-sm text-surface-600 dark:text-surface-300">
                            Evidence {evidenceIndex + 1}
                          </span>
                          {evidence.confirmed ? (
                            <label className="flex items-center gap-2 text-sm font-medium text-green-700 dark:text-green-300">
                              <input
                                type="checkbox"
                                checked={selectedEvidenceIds.includes(evidence.evidence_id)}
                                onChange={() => toggleEvidence(evidence.evidence_id)}
                                aria-label={`Select Evidence ${evidenceIndex + 1} for ${requirement.requirement} reviewed claim`}
                              />
                              Confirmed
                            </label>
                          ) : anotherEvidenceIsConfirming ? (
                            <span className="text-sm text-surface-500 dark:text-surface-400">
                              Confirming another evidence…
                            </span>
                          ) : (
                            <Button
                              size="sm"
                              variant="secondary"
                              loading={confirming}
                              loadingText="Confirming…"
                              onClick={() => void confirmEvidence(evidence.evidence_id)}
                            >
                              Confirm evidence
                            </Button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </section>
            );
          })}
        </div>
      )}
      <section className="mt-6 border-t border-surface-200 pt-4 dark:border-surface-700">
        <h3 className="font-medium text-surface-900 dark:text-white">Reviewed claims</h3>
        <p className="mt-1 text-sm text-surface-600 dark:text-surface-300">
          Save only text you have reviewed against selected confirmed evidence.
        </p>
        <label htmlFor="saved-match-reviewed-claim" className="mt-3 block text-sm font-medium text-surface-700 dark:text-surface-200">
          Reviewed claim
        </label>
        <input
          type="text"
          id="saved-match-reviewed-claim"
          value={reviewedText}
          onChange={(event) => setReviewedText(event.target.value)}
          maxLength={8192}
          className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-sm text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
        />
        <p className="mt-2 text-xs text-surface-600 dark:text-surface-300">
          Clearance currentness is not verified. Military/civilian equivalence is not verified.
        </p>
        <Button
          className="mt-3"
          variant="secondary"
          disabled={!reviewedText.trim() || selectedEvidenceIds.length === 0}
          loading={packetSaving}
          loadingText="Saving…"
          onClick={() => void savePacketClaim()}
        >
          Save reviewed claim
        </Button>
        {packetError && <p role="alert" className="mt-3 text-sm text-red-700 dark:text-red-300">{packetError}</p>}
        {packetLoading ? (
          <p className="mt-3 text-sm text-surface-600 dark:text-surface-300">Loading reviewed claims…</p>
        ) : packetClaims.length === 0 ? (
          <p className="mt-3 text-sm text-surface-600 dark:text-surface-300">No reviewed claims saved yet.</p>
        ) : (
          <ul className="mt-3 space-y-2">
            {packetClaims.map((claim) => (
              <li key={claim.claim_id} className="rounded-md bg-surface-50 p-3 text-sm text-surface-800 dark:bg-surface-700 dark:text-surface-100">
                {claim.reviewed_text}
              </li>
            ))}
          </ul>
        )}
      </section>
      {match ? <PackEvidenceReviewer match={match} /> : null}
      <ModalFooter>
        <Button variant="secondary" onClick={onClose}>Close</Button>
      </ModalFooter>
    </Modal>
  );
}
