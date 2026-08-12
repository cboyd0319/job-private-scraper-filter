import { withSave, withoutSave } from "./resumeCommandHandlers";
import type {
  MockResumeCommandResult,
  MockResumeCommandState,
} from "./resumeCommandTypes";

const MILITARY_BRANCHES = new Set([
  "army",
  "marine_corps",
  "navy",
  "air_force",
  "space_force",
  "coast_guard",
]);
const MAX_PENDING_MILITARY_TRANSITION_REVIEWS = 20;
let nextMilitaryTransitionReviewToken = 1;

export function handleMockMilitaryTransitionCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
  getMatch: () => MockResumeCommandState["recentMatches"][number],
): MockResumeCommandResult | null {
  switch (command) {
    case "confirm_saved_match_military_evidence": {
      const match = getMatch();
      const kind = args?.kind;
      if (kind !== "military_service" && kind !== "current_clearance") {
        throw new Error("Choose a supported military evidence confirmation.");
      }
      const militaryEvidenceKind: "military_service" | "current_clearance" =
        kind;
      const identity = savedMatchEvidenceIdentity(match);
      const savedEvidence =
        state.savedMatchEvidence[identity] ?? emptySavedMatchEvidence();
      const confirmedMilitaryEvidenceKinds =
        confirmedMilitaryEvidenceKindsFor(savedEvidence);
      return withSave(
        {
          ...state,
          savedMatchEvidence: {
            ...state.savedMatchEvidence,
            [identity]: {
              ...savedEvidence,
              confirmedMilitaryEvidenceKinds: [
                ...new Set([
                  ...confirmedMilitaryEvidenceKinds,
                  militaryEvidenceKind,
                ]),
              ],
            },
          },
        },
        true,
      );
    }

    case "prepare_saved_match_military_transition_review": {
      const match = getMatch();
      const branch = args?.branch;
      const wording = parseMilitaryTransitionWording(args?.wording);
      if (
        typeof branch !== "string" ||
        !MILITARY_BRANCHES.has(branch) ||
        !wording
      ) {
        throw new Error(
          "Enter supported military transition wording before review.",
        );
      }
      const confirmed = new Set(
        confirmedMilitaryEvidenceKindsFor(
          state.savedMatchEvidence[savedMatchEvidenceIdentity(match)],
        ),
      );
      if (!confirmed.has("military_service")) {
        throw new Error(
          "Confirm military evidence before preparing this review.",
        );
      }
      if (wording.current_clearance && !confirmed.has("current_clearance")) {
        throw new Error(
          "Confirm current clearance evidence before preparing this review.",
        );
      }
      if (!hasCanonicalSavedMatchEvidence(state, match, wording)) {
        throw new Error(
          "Military source evidence no longer matches this saved match.",
        );
      }
      const savedMatchIdentity = savedMatchEvidenceIdentity(match);
      const pendingMilitaryTransitionReviews = [
        ...state.pendingMilitaryTransitionReviews.filter(
          (review) => review.savedMatchIdentity !== savedMatchIdentity,
        ),
        {
          token: nextMockMilitaryTransitionReviewToken(),
          savedMatchIdentity,
          wording,
        },
      ].slice(-MAX_PENDING_MILITARY_TRANSITION_REVIEWS);
      return withoutSave(
        { ...state, pendingMilitaryTransitionReviews },
        pendingMilitaryTransitionReviews[
          pendingMilitaryTransitionReviews.length - 1
        ]?.token,
      );
    }

    case "confirm_saved_match_military_transition_review": {
      const token = args?.token;
      if (
        typeof token !== "string" ||
        !isMockPendingMilitaryTransitionToken(token)
      ) {
        throw new Error(
          "This military transition review is no longer available.",
        );
      }
      const reviewIndex = state.pendingMilitaryTransitionReviews.findIndex(
        (review) => review.token === token,
      );
      const review = state.pendingMilitaryTransitionReviews[reviewIndex];
      if (!review)
        throw new Error(
          "This military transition review is no longer available.",
        );
      return withoutSave(
        {
          ...state,
          pendingMilitaryTransitionReviews:
            state.pendingMilitaryTransitionReviews.filter(
              (_, index) => index !== reviewIndex,
            ),
        },
        {
          civilian_role: review.wording.civilian_role,
          civilian_responsibilities: review.wording.responsibility_mappings.map(
            ({ civilian_wording }) => civilian_wording,
          ),
          credential_wording: review.wording.credential_mappings.map(
            ({ civilian_wording }) => civilian_wording,
          ),
          user_confirmed_current_clearance: review.wording.current_clearance,
          boundary: "suggestion_only",
          clearance_currentness: "not_verified",
          military_civilian_equivalence: "not_verified",
        },
      );
    }

    default:
      return null;
  }
}

function savedMatchEvidenceIdentity(
  match: MockResumeCommandState["recentMatches"][number],
) {
  return `${match.id}:${match.resume_id}:${match.job_hash}`;
}

function emptySavedMatchEvidence(): MockResumeCommandState["savedMatchEvidence"][string] {
  return {
    confirmedEvidenceIds: [],
    confirmedMilitaryEvidenceKinds: [],
    packetClaims: [],
  };
}

function confirmedMilitaryEvidenceKindsFor(
  evidence: MockResumeCommandState["savedMatchEvidence"][string] | undefined,
) {
  return (evidence?.confirmedMilitaryEvidenceKinds ?? []).filter(
    (kind): kind is "military_service" | "current_clearance" =>
      kind === "military_service" || kind === "current_clearance",
  );
}

function parseMilitaryTransitionWording(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const wording = value as Record<string, unknown>;
  if (
    !hasOnlyKeys(wording, [
      "occupationCode",
      "civilianRole",
      "responsibilityMappings",
      "credentialMappings",
      "currentClearance",
    ])
  )
    return null;
  const occupationCode = wording.occupationCode;
  const civilianRole = wording.civilianRole;
  const responsibilityMappings = parseMilitaryWordingMappings(
    wording.responsibilityMappings,
  );
  const credentialMappings = parseMilitaryWordingMappings(
    wording.credentialMappings,
  );
  const currentClearance = wording.currentClearance;
  if (
    !isValidMilitaryWordingText(occupationCode, 32) ||
    !isValidMilitaryWordingText(civilianRole, 256) ||
    !responsibilityMappings ||
    !credentialMappings ||
    (currentClearance !== null &&
      currentClearance !== undefined &&
      !isValidMilitaryWordingText(currentClearance, 128))
  )
    return null;
  return {
    occupation_code: occupationCode,
    civilian_role: civilianRole,
    responsibility_mappings: responsibilityMappings,
    credential_mappings: credentialMappings,
    current_clearance: currentClearance ?? null,
  };
}

function parseMilitaryWordingMappings(value: unknown) {
  if (!Array.isArray(value) || value.length > 16) return null;
  const mappings = value.map((mapping) => {
    if (!mapping || typeof mapping !== "object" || Array.isArray(mapping))
      return null;
    const values = mapping as Record<string, unknown>;
    if (
      !hasOnlyKeys(values, ["militaryEvidence", "civilianWording"]) ||
      !isValidMilitaryWordingText(values.militaryEvidence, 256) ||
      !isValidMilitaryWordingText(values.civilianWording, 256)
    )
      return null;
    return {
      military_evidence: values.militaryEvidence,
      civilian_wording: values.civilianWording,
    };
  });
  return mappings.every((mapping) => mapping !== null)
    ? (mappings as Array<{
        military_evidence: string;
        civilian_wording: string;
      }>)
    : null;
}

function hasCanonicalSavedMatchEvidence(
  state: MockResumeCommandState,
  match: MockResumeCommandState["recentMatches"][number],
  wording: {
    occupation_code: string;
    responsibility_mappings: Array<{ military_evidence: string }>;
    credential_mappings: Array<{ military_evidence: string }>;
    current_clearance: string | null;
  },
) {
  const sourceText = state.resumes.find(
    (resume) => resume.id === match.resume_id,
  )?.parsed_text;
  if (typeof sourceText !== "string") return false;
  const phrases = [
    wording.occupation_code,
    ...wording.responsibility_mappings.map(
      (mapping) => mapping.military_evidence,
    ),
    ...wording.credential_mappings.map((mapping) => mapping.military_evidence),
    ...(wording.current_clearance ? [wording.current_clearance] : []),
  ];
  return phrases.every((phrase) =>
    containsExactSourceEvidence(sourceText, phrase),
  );
}

function containsExactSourceEvidence(sourceText: string, phrase: string) {
  const source = [...normalizeSourceEvidence(sourceText)];
  const candidate = [...normalizeSourceEvidence(phrase)];
  if (candidate.length === 0 || candidate.length > source.length) return false;
  for (let start = 0; start <= source.length - candidate.length; start += 1) {
    if (
      !candidate.every(
        (character, index) => source[start + index] === character,
      )
    )
      continue;
    const before = source[start - 1];
    const after = source[start + candidate.length];
    if (
      (before === undefined ||
        !/^(?:\p{Alphabetic}|\p{Number})$/u.test(before)) &&
      (after === undefined || !/^(?:\p{Alphabetic}|\p{Number})$/u.test(after))
    )
      return true;
  }
  return false;
}

function normalizeSourceEvidence(value: string) {
  return [
    ...value.split(/\p{White_Space}+/u).filter(Boolean).join(" "),
  ]
    .map((character) => character.toLowerCase())
    .join("");
}

function isValidMilitaryWordingText(
  value: unknown,
  maximumCodePoints: number,
): value is string {
  return (
    typeof value === "string" &&
    value === value.trim() &&
    value.length > 0 &&
    [...value].length <= maximumCodePoints &&
    !/\p{Cc}/u.test(value)
  );
}

function nextMockMilitaryTransitionReviewToken() {
  const token = globalThis.crypto?.randomUUID?.();
  if (token && isMockPendingMilitaryTransitionToken(token)) return token;
  const sequence = (nextMilitaryTransitionReviewToken++)
    .toString(16)
    .padStart(12, "0");
  return `00000000-0000-4000-8000-${sequence}`;
}

function isMockPendingMilitaryTransitionToken(value: string) {
  return /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(
    value,
  );
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: string[]) {
  return Object.keys(value).every((key) => allowed.includes(key));
}
