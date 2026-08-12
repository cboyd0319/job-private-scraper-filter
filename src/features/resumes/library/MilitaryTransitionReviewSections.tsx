import { Button } from "../../../ui/Button";
import type {
  SafeMilitarySuggestion,
  WordingMapping,
} from "./MilitaryTransitionReviewModal";

const emptyMapping = (): WordingMapping => ({
  militaryEvidence: "",
  civilianWording: "",
});

export function MappingList({
  title,
  kind,
  mappings,
  setMappings,
  updateMapping,
  disabled,
}: {
  title: string;
  kind: "Responsibility" | "Credential";
  mappings: WordingMapping[];
  setMappings: (next: WordingMapping[]) => void;
  updateMapping: (
    set: (next: WordingMapping[]) => void,
    values: WordingMapping[],
    index: number,
    field: keyof WordingMapping,
    value: string,
  ) => void;
  disabled: boolean;
}) {
  const singular = kind.toLowerCase();
  return (
    <section>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="font-medium text-surface-900 dark:text-white">
          {title}
        </h3>
        <Button
          size="sm"
          variant="secondary"
          disabled={disabled || mappings.length >= 16}
          onClick={() => setMappings([...mappings, emptyMapping()])}
        >
          Add {singular} mapping
        </Button>
      </div>
      <div className="mt-3 space-y-3">
        {mappings.map((mapping, index) => (
          <div
            key={index}
            className="grid gap-2 rounded-lg border border-surface-200 p-3 sm:grid-cols-2 dark:border-surface-700"
          >
            <label className="text-sm text-surface-700 dark:text-surface-200">
              Source phrase
              <input
                aria-label={`${kind} source phrase ${index + 1}`}
                value={mapping.militaryEvidence}
                disabled={disabled}
                onChange={(event) =>
                  updateMapping(
                    setMappings,
                    mappings,
                    index,
                    "militaryEvidence",
                    event.target.value,
                  )
                }
                maxLength={256}
                className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
              />
            </label>
            <label className="text-sm text-surface-700 dark:text-surface-200">
              Proposed civilian wording
              <input
                aria-label={`${kind} civilian wording ${index + 1}`}
                value={mapping.civilianWording}
                disabled={disabled}
                onChange={(event) =>
                  updateMapping(
                    setMappings,
                    mappings,
                    index,
                    "civilianWording",
                    event.target.value,
                  )
                }
                maxLength={256}
                className="mt-1 w-full rounded-md border border-surface-300 bg-white p-2 text-surface-900 dark:border-surface-600 dark:bg-surface-800 dark:text-white"
              />
            </label>
            <Button
              size="sm"
              variant="ghost"
              className="sm:col-span-2 w-fit"
              disabled={disabled}
              onClick={() =>
                setMappings(
                  mappings.filter((_, mappingIndex) => mappingIndex !== index),
                )
              }
            >
              Remove {singular} mapping {index + 1}
            </Button>
          </div>
        ))}
      </div>
    </section>
  );
}

export function FinalReview({
  civilianRole,
  responsibilityMappings,
  credentialMappings,
  currentClearance,
  confirming,
  onConfirm,
}: {
  civilianRole: string;
  responsibilityMappings: WordingMapping[];
  credentialMappings: WordingMapping[];
  currentClearance: string | null;
  confirming: boolean;
  onConfirm: () => void;
}) {
  return (
    <section className="space-y-4">
      <h3 className="font-medium text-surface-900 dark:text-white">
        Confirm the wording you entered
      </h3>
      <p className="text-sm text-surface-600 dark:text-surface-300">
        Proposed civilian role: {civilianRole}
      </p>
      <ReviewMappings
        title="Responsibility mappings"
        mappings={responsibilityMappings}
      />
      <ReviewMappings
        title="Credential mappings"
        mappings={credentialMappings}
      />
      {currentClearance && (
        <p className="text-sm text-surface-600 dark:text-surface-300">
          User-confirmed current clearance: {currentClearance}
        </p>
      )}
      <Button
        loading={confirming}
        loadingText="Confirming…"
        onClick={onConfirm}
      >
        Confirm suggestion
      </Button>
    </section>
  );
}

function ReviewMappings({
  title,
  mappings,
}: {
  title: string;
  mappings: WordingMapping[];
}) {
  if (mappings.length === 0) return null;
  return (
    <section>
      <h4 className="text-sm font-medium text-surface-700 dark:text-surface-200">
        {title}
      </h4>
      <ul className="mt-2 space-y-2">
        {mappings.map((mapping, index) => (
          <li
            key={index}
            className="rounded-md bg-surface-50 p-3 text-sm text-surface-800 dark:bg-surface-700 dark:text-surface-100"
          >
            <p>{mapping.militaryEvidence}</p>
            <p className="mt-1">{mapping.civilianWording}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function SafeSuggestion({
  suggestion,
}: {
  suggestion: SafeMilitarySuggestion;
}) {
  return (
    <section className="space-y-3">
      <h3 className="font-medium text-surface-900 dark:text-white">
        Confirmed suggestion
      </h3>
      <p className="text-sm text-surface-700 dark:text-surface-200">
        {suggestion.civilian_role}
      </p>
      <SafeList
        label="Responsibilities"
        values={suggestion.civilian_responsibilities}
      />
      <SafeList
        label="Credential wording"
        values={suggestion.credential_wording}
      />
      {suggestion.user_confirmed_current_clearance && (
        <p className="text-sm text-surface-700 dark:text-surface-200">
          User-confirmed current clearance:{" "}
          {suggestion.user_confirmed_current_clearance}
        </p>
      )}
      <p className="text-sm text-surface-600 dark:text-surface-300">
        Suggestion only. Clearance currentness and military/civilian equivalence
        are not verified.
      </p>
    </section>
  );
}

function SafeList({ label, values }: { label: string; values: string[] }) {
  if (values.length === 0) return null;
  return (
    <section>
      <h4 className="text-sm font-medium text-surface-700 dark:text-surface-200">
        {label}
      </h4>
      <ul className="mt-1 list-disc pl-5 text-sm text-surface-700 dark:text-surface-200">
        {values.map((value, index) => (
          <li key={`${index}-${value}`}>{value}</li>
        ))}
      </ul>
    </section>
  );
}
