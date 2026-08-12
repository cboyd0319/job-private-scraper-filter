import type {
  MockJob,
  MockMatchResult,
  MockResumeData,
  MockResumeDraft,
  MockSavedMatchEvidenceState,
  MockUserSkill,
} from "../../mocks/handlers/types";

export interface MockResumeCommandState {
  jobs: MockJob[];
  resumes: MockResumeData[];
  userSkills: MockUserSkill[];
  resumeDrafts: MockResumeDraft[];
  recentMatches: MockMatchResult[];
  savedMatchEvidence: Record<string, MockSavedMatchEvidenceState>;
  pendingMilitaryTransitionReviews: MockPendingMilitaryTransitionReview[];
}

export interface MockPendingMilitaryTransitionReview {
  token: string;
  savedMatchIdentity: string;
  wording: {
    occupation_code: string;
    civilian_role: string;
    responsibility_mappings: Array<{
      military_evidence: string;
      civilian_wording: string;
    }>;
    credential_mappings: Array<{
      military_evidence: string;
      civilian_wording: string;
    }>;
    current_clearance: string | null;
  };
}

export interface MockResumeCommandResult {
  handled: boolean;
  shouldSave: boolean;
  state: MockResumeCommandState;
  value: unknown;
}
