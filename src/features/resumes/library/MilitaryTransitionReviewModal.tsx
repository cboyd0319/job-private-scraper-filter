import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { safeInvoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import { Modal, ModalFooter } from "../../../ui/Modal";
import type { MatchResult } from "./resumePageModel";
import {
  FinalReview,
  MappingList,
  SafeSuggestion,
} from "./MilitaryTransitionReviewSections";

type MilitaryBranch =
  | "army"
  | "marine_corps"
  | "navy"
  | "air_force"
  | "space_force"
  | "coast_guard";

export type WordingMapping = {
  militaryEvidence: string;
  civilianWording: string;
};

export type SafeMilitarySuggestion = {
  civilian_role: string;
  civilian_responsibilities: string[];
  credential_wording: string[];
  user_confirmed_current_clearance: string | null;
  boundary: "suggestion_only";
  clearance_currentness: "not_verified";
  military_civilian_equivalence: "not_verified";
};

interface MilitaryTransitionReviewModalProps {
  isOpen: boolean;
  match: MatchResult | null;
  onClose: () => void;
}

const branchOptions: Array<{ value: MilitaryBranch; label: string }> = [
  { value: "army", label: "Army" },
  { value: "marine_corps", label: "Marine Corps" },
  { value: "navy", label: "Navy" },
  { value: "air_force", label: "Air Force" },
  { value: "space_force", label: "Space Force" },
  { value: "coast_guard", label: "Coast Guard" },
];

export function MilitaryTransitionReviewModal({
  isOpen,
  match,
  onClose,
}: MilitaryTransitionReviewModalProps) {
  const [branch, setBranch] = useState<MilitaryBranch | "">("");
  const [occupationCode, setOccupationCode] = useState("");
  const [civilianRole, setCivilianRole] = useState("");
  const [responsibilityMappings, setResponsibilityMappings] = useState<
    WordingMapping[]
  >([]);
  const [credentialMappings, setCredentialMappings] = useState<
    WordingMapping[]
  >([]);
  const [currentClearance, setCurrentClearance] = useState("");
  const [militaryEvidenceConfirmed, setMilitaryEvidenceConfirmed] =
    useState(false);
  const [clearanceEvidenceConfirmed, setClearanceEvidenceConfirmed] =
    useState(false);
  const [confirmingKind, setConfirmingKind] = useState<
    "military_service" | "current_clearance" | null
  >(null);
  const [preparing, setPreparing] = useState(false);
  const [reviewToken, setReviewToken] = useState<string | null>(null);
  const [confirmingSuggestion, setConfirmingSuggestion] = useState(false);
  const [suggestion, setSuggestion] = useState<SafeMilitarySuggestion | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const interaction = useRef(0);
  const matchKey = match
    ? `${match.id}:${match.resume_id}:${match.job_hash}`
    : null;

  const isCurrent = useCallback(
    (request: number, requestMatchKey: string | null) =>
      interaction.current === request && matchKey === requestMatchKey,
    [matchKey],
  );

  useLayoutEffect(() => {
    interaction.current += 1;
    setBranch("");
    setOccupationCode("");
    setCivilianRole("");
    setResponsibilityMappings([]);
    setCredentialMappings([]);
    setCurrentClearance("");
    setMilitaryEvidenceConfirmed(false);
    setClearanceEvidenceConfirmed(false);
    setConfirmingKind(null);
    setPreparing(false);
    setReviewToken(null);
    setConfirmingSuggestion(false);
    setSuggestion(null);
    setError(null);
  }, [isOpen, matchKey]);

  const closeReview = useCallback(() => {
    interaction.current += 1;
    onClose();
  }, [onClose]);

  const confirmEvidence = async (
    kind: "military_service" | "current_clearance",
  ) => {
    if (!match || confirmingKind) return;
    const request = interaction.current;
    const requestMatchKey = matchKey;
    setConfirmingKind(kind);
    setError(null);
    try {
      await safeInvoke<boolean>(
        "confirm_saved_match_military_evidence",
        { jobHash: match.job_hash, resumeId: match.resume_id, kind },
        { logContext: "Confirm military transition evidence" },
      );
      if (!isCurrent(request, requestMatchKey)) return;
      if (kind === "military_service") setMilitaryEvidenceConfirmed(true);
      else setClearanceEvidenceConfirmed(true);
    } catch {
      if (isCurrent(request, requestMatchKey)) {
        setError(
          "Could not confirm this evidence. Close the review and try again.",
        );
      }
    } finally {
      if (isCurrent(request, requestMatchKey)) setConfirmingKind(null);
    }
  };

  const prepareReview = async () => {
    if (!match || !branch || !militaryEvidenceConfirmed || preparing) return;
    const request = interaction.current;
    const requestMatchKey = matchKey;
    setPreparing(true);
    setError(null);
    try {
      const token = await safeInvoke<string>(
        "prepare_saved_match_military_transition_review",
        {
          jobHash: match.job_hash,
          resumeId: match.resume_id,
          branch,
          wording: {
            occupationCode,
            civilianRole,
            responsibilityMappings,
            credentialMappings,
            currentClearance: currentClearance.trim() || null,
          },
        },
        { logContext: "Prepare military transition review" },
      );
      if (isCurrent(request, requestMatchKey)) setReviewToken(token);
    } catch {
      if (isCurrent(request, requestMatchKey)) {
        setError(
          "Could not prepare this review. Check the confirmed evidence and wording, then try again.",
        );
      }
    } finally {
      if (isCurrent(request, requestMatchKey)) setPreparing(false);
    }
  };

  const confirmSuggestion = async () => {
    if (!reviewToken || confirmingSuggestion) return;
    const request = interaction.current;
    const requestMatchKey = matchKey;
    setConfirmingSuggestion(true);
    setError(null);
    try {
      const result = await safeInvoke<SafeMilitarySuggestion>(
        "confirm_saved_match_military_transition_review",
        { token: reviewToken },
        { logContext: "Confirm military transition review" },
      );
      if (isCurrent(request, requestMatchKey)) {
        setSuggestion(result);
        setReviewToken(null);
      }
    } catch {
      if (isCurrent(request, requestMatchKey)) {
        setError(
          "This review is no longer available. Start a new review before confirming.",
        );
        setReviewToken(null);
      }
    } finally {
      if (isCurrent(request, requestMatchKey)) setConfirmingSuggestion(false);
    }
  };

  const updateMapping = (
    setMappings: (next: WordingMapping[]) => void,
    mappings: WordingMapping[],
    index: number,
    field: keyof WordingMapping,
    value: string,
  ) =>
    setMappings(
      mappings.map((mapping, mappingIndex) =>
        mappingIndex === index ? { ...mapping, [field]: value } : mapping,
      ),
    );

  const readyToPrepare = Boolean(
    branch &&
    occupationCode.trim() &&
    civilianRole.trim() &&
    militaryEvidenceConfirmed &&
    responsibilityMappings.every(
      (mapping) =>
        mapping.militaryEvidence.trim() && mapping.civilianWording.trim(),
    ) &&
    credentialMappings.every(
      (mapping) =>
        mapping.militaryEvidence.trim() && mapping.civilianWording.trim(),
    ) &&
    (!currentClearance.trim() || clearanceEvidenceConfirmed),
  );

  return (
    <Modal
      isOpen={isOpen && match !== null}
      onClose={closeReview}
      title="Military transition wording review"
      description="Review your wording locally before it becomes a suggestion."
      size="lg"
      closeButtonLabel="Close military transition review"
    >
      <div className="space-y-4">
        <section className="rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm text-amber-950 dark:border-amber-700 dark:bg-amber-950/30 dark:text-amber-100">
          <p>Manual review required. This is suggestion-only wording.</p>
          <p className="mt-1">
            Military/civilian equivalence is not verified. Clearance currentness
            is not verified.
          </p>
          <p className="mt-1">
            O*NET and DoD COOL do not author or verify wording, eligibility,
            clearance status, or military-to-civilian equivalence.
          </p>
        </section>

        {suggestion ? (
          <SafeSuggestion suggestion={suggestion} />
        ) : reviewToken ? (
          <FinalReview
            civilianRole={civilianRole}
            responsibilityMappings={responsibilityMappings}
            credentialMappings={credentialMappings}
            currentClearance={currentClearance.trim() || null}
            confirming={confirmingSuggestion}
            onConfirm={() => void confirmSuggestion()}
          />
        ) : (
          <div className="space-y-5">
            <section>
              <h3 className="font-medium text-surface-900 dark:text-white">
                Confirm saved evidence
              </h3>
              <p className="mt-1 text-sm text-surface-600 dark:text-surface-300">
                Confirm only evidence you entered and reviewed in this saved
                match.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                <Button
                  variant="secondary"
                  loading={confirmingKind === "military_service"}
                  loadingText="Confirming…"
                  disabled={
                    preparing ||
                    militaryEvidenceConfirmed ||
                    confirmingKind !== null
                  }
                  onClick={() => void confirmEvidence("military_service")}
                >
                  {militaryEvidenceConfirmed
                    ? "Military service evidence confirmed"
                    : "Confirm military service evidence"}
                </Button>
                {currentClearance.trim() && (
                  <Button
                    variant="secondary"
                    loading={confirmingKind === "current_clearance"}
                    loadingText="Confirming…"
                    disabled={
                      preparing ||
                      clearanceEvidenceConfirmed ||
                      confirmingKind !== null
                    }
                    onClick={() => void confirmEvidence("current_clearance")}
                  >
                    {clearanceEvidenceConfirmed
                      ? "Current clearance evidence confirmed"
                      : "Confirm current clearance evidence"}
                  </Button>
                )}
              </div>
            </section>

            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-200">
                Military branch
                <select
                  aria-label="Military branch"
                  value={branch}
                  disabled={preparing}
                  onChange={(event) =>
                    setBranch(event.target.value as MilitaryBranch | "")
                  }
                  className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
                >
                  <option value="">Choose branch</option>
                  {branchOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-200">
                Occupation code
                <input
                  aria-label="Occupation code"
                  value={occupationCode}
                  disabled={preparing}
                  onChange={(event) => setOccupationCode(event.target.value)}
                  maxLength={32}
                  className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
                />
              </label>
            </div>
            <label className="block text-sm font-medium text-surface-700 dark:text-surface-200">
              Proposed civilian role
              <input
                aria-label="Proposed civilian role"
                value={civilianRole}
                disabled={preparing}
                onChange={(event) => setCivilianRole(event.target.value)}
                maxLength={256}
                className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
              />
            </label>
            <label className="block text-sm font-medium text-surface-700 dark:text-surface-200">
              Optional user-confirmed current clearance
              <input
                aria-label="Optional user-confirmed current clearance"
                value={currentClearance}
                disabled={preparing}
                onChange={(event) => {
                  setCurrentClearance(event.target.value);
                  setClearanceEvidenceConfirmed(false);
                }}
                maxLength={128}
                className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
              />
            </label>
            <MappingList
              title="Responsibility mappings"
              kind="Responsibility"
              mappings={responsibilityMappings}
              setMappings={setResponsibilityMappings}
              updateMapping={updateMapping}
              disabled={preparing}
            />
            <MappingList
              title="Credential mappings"
              kind="Credential"
              mappings={credentialMappings}
              setMappings={setCredentialMappings}
              updateMapping={updateMapping}
              disabled={preparing}
            />
            <Button
              disabled={!readyToPrepare}
              loading={preparing}
              loadingText="Preparing review…"
              onClick={() => void prepareReview()}
            >
              Prepare final review
            </Button>
          </div>
        )}
        {error && (
          <p role="alert" className="text-sm text-red-700 dark:text-red-300">
            {error}
          </p>
        )}
      </div>
      <ModalFooter>
        <Button variant="secondary" onClick={closeReview}>
          Close
        </Button>
      </ModalFooter>
    </Modal>
  );
}
