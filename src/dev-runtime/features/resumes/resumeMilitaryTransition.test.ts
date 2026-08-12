import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke, resetMockData } from "../../mocks/handlers";
import { loadMockState, mockRuntimeState } from "../../mocks/runtimeState";
import { handleMockResumeCommand } from "./resumeCommands";
import type { MockResumeCommandState } from "./resumeCommandTypes";
import { setupResumeRuntimeMocks } from "./resumeRuntimeTestSupport";

type Match = { id: number; resume_id: number; job_hash: string };

type MilitaryProjection = {
  civilian_role: string;
  civilian_responsibilities: string[];
  credential_wording: string[];
  user_confirmed_current_clearance: string | null;
  boundary: "suggestion_only";
  clearance_currentness: "not_verified";
  military_civilian_equivalence: "not_verified";
};

const wording = {
  occupationCode: "25B",
  civilianRole: "Network support specialist",
  responsibilityMappings: [
    {
      militaryEvidence: "Configured tactical networks",
      civilianWording: "Configured and supported network infrastructure",
    },
  ],
  credentialMappings: [
    {
      militaryEvidence: "CompTIA Security+",
      civilianWording: "CompTIA Security+ certification",
    },
  ],
  currentClearance: "Secret clearance",
};

const canonicalMilitarySource = [
  "Army 25B.",
  "Configured tactical networks.",
  "CompTIA Security+.",
  "Secret clearance.",
].join("\n");

async function savedMatch(): Promise<Match> {
  const [job] = await mockInvoke<Array<{ hash: string }>>("get_jobs", {});
  const resumeId = await mockInvoke<number>("select_and_upload_resume");
  const match = await mockInvoke<Match>("match_resume_to_job", {
    resumeId,
    jobHash: job?.hash,
  });
  const resume = mockRuntimeState.resumes.find(
    (value) => value.id === resumeId,
  );
  if (resume) resume.parsed_text = canonicalMilitarySource;
  return match;
}

async function confirmMilitaryEvidence(match: Match, includeClearance = true) {
  await mockInvoke("confirm_saved_match_military_evidence", {
    jobHash: match.job_hash,
    resumeId: match.resume_id,
    kind: "military_service",
  });
  if (includeClearance) {
    await mockInvoke("confirm_saved_match_military_evidence", {
      jobHash: match.job_hash,
      resumeId: match.resume_id,
      kind: "current_clearance",
    });
  }
}

describe("mock saved-match military transition commands", () => {
  beforeEach(setupResumeRuntimeMocks);

  it("requires explicit closed evidence before returning a one-use safe projection", async () => {
    const match = await savedMatch();
    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording,
      }),
    ).rejects.toThrow("Confirm military evidence");

    await confirmMilitaryEvidence(match);
    const token = await mockInvoke<string>(
      "prepare_saved_match_military_transition_review",
      {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording,
      },
    );

    expect(token).toMatch(
      /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/,
    );
    const projection = await mockInvoke<MilitaryProjection>(
      "confirm_saved_match_military_transition_review",
      { token },
    );
    expect(projection).toEqual({
      civilian_role: wording.civilianRole,
      civilian_responsibilities: wording.responsibilityMappings.map(
        ({ civilianWording }) => civilianWording,
      ),
      credential_wording: wording.credentialMappings.map(
        ({ civilianWording }) => civilianWording,
      ),
      user_confirmed_current_clearance: wording.currentClearance,
      boundary: "suggestion_only",
      clearance_currentness: "not_verified",
      military_civilian_equivalence: "not_verified",
    });
    await expect(
      mockInvoke("confirm_saved_match_military_transition_review", { token }),
    ).rejects.toThrow("no longer available");
  });

  it("keeps draft wording memory-only, rejects stale replacements, and clears it on reset", async () => {
    const match = await savedMatch();
    await confirmMilitaryEvidence(match);
    const first = await mockInvoke<string>(
      "prepare_saved_match_military_transition_review",
      {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording,
      },
    );
    const replacement = await mockInvoke<string>(
      "prepare_saved_match_military_transition_review",
      {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording: { ...wording, civilianRole: "IT support specialist" },
      },
    );

    expect(replacement).not.toBe(first);
    await expect(
      mockInvoke("confirm_saved_match_military_transition_review", {
        token: first,
      }),
    ).rejects.toThrow("no longer available");
    expect(mockRuntimeState.pendingMilitaryTransitionReviews).toHaveLength(1);

    const persisted =
      window.localStorage.getItem("jobsentinel.mockState.v1") ?? "";
    expect(persisted).not.toContain("pendingMilitaryTransitionReviews");
    expect(persisted).not.toContain("IT support specialist");
    mockRuntimeState.pendingMilitaryTransitionReviews = [];
    loadMockState();
    expect(mockRuntimeState.pendingMilitaryTransitionReviews).toEqual([]);
    await mockInvoke("prepare_saved_match_military_transition_review", {
      jobHash: match.job_hash,
      resumeId: match.resume_id,
      branch: "army",
      wording: { ...wording, currentClearance: null },
    });
    expect(mockRuntimeState.pendingMilitaryTransitionReviews).toHaveLength(1);
    resetMockData();
    expect(mockRuntimeState.pendingMilitaryTransitionReviews).toEqual([]);
  });

  it("fails closed for invalid wording, branch, and unconfirmed clearance", async () => {
    const match = await savedMatch();
    await confirmMilitaryEvidence(match, false);
    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording,
      }),
    ).rejects.toThrow("current clearance");
    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army ",
        wording: { ...wording, currentClearance: null },
      }),
    ).rejects.toThrow("military transition");
    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording: {
          ...wording,
          currentClearance: null,
          occupationCode: "e".repeat(33),
        },
      }),
    ).rejects.toThrow("military transition");
    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording: { ...wording, currentClearance: null, unsupported: "no" },
      }),
    ).rejects.toThrow("military transition");
  });

  it("requires source phrases to match canonical saved-match evidence at a word boundary", async () => {
    const match = await savedMatch();
    await confirmMilitaryEvidence(match);

    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording: {
          ...wording,
          responsibilityMappings: [
            {
              ...wording.responsibilityMappings[0],
              militaryEvidence: "Configured tactical networksX",
            },
          ],
        },
      }),
    ).rejects.toThrow("source evidence");
  });

  it("rejects JavaScript-only whitespace and boundary matches that Rust keeps adjacent", async () => {
    for (const sourceText of [
      canonicalMilitarySource.replace(
        "Configured tactical networks.",
        "Configured\uFEFFtactical networks.",
      ),
      canonicalMilitarySource.replace(
        "Configured tactical networks.",
        "Configured tactical networks\u0345.",
      ),
    ]) {
      const match = await savedMatch();
      const resume = mockRuntimeState.resumes.find(
        (value) => value.id === match.resume_id,
      );
      if (resume) resume.parsed_text = sourceText;
      await confirmMilitaryEvidence(match);

      await expect(
        mockInvoke("prepare_saved_match_military_transition_review", {
          jobHash: match.job_hash,
          resumeId: match.resume_id,
          branch: "army",
          wording,
        }),
      ).rejects.toThrow("source evidence");
    }
  });

  it("matches Rust scalar-by-scalar Unicode lowercasing", async () => {
    const match = await savedMatch();
    const resume = mockRuntimeState.resumes.find(
      (value) => value.id === match.resume_id,
    );
    if (resume) resume.parsed_text = canonicalMilitarySource.replace("25B", "ΟΣ");
    await confirmMilitaryEvidence(match);

    await expect(
      mockInvoke("prepare_saved_match_military_transition_review", {
        jobHash: match.job_hash,
        resumeId: match.resume_id,
        branch: "army",
        wording: { ...wording, occupationCode: "Ος" },
      }),
    ).rejects.toThrow("source evidence");
  });

  it("bounds memory-only drafts at twenty and evicts the oldest token", async () => {
    const match = await savedMatch();
    const template = mockRuntimeState.recentMatches.find(
      (value) => value.id === match.id,
    );
    expect(template).toBeDefined();
    let state: MockResumeCommandState = {
      jobs: mockRuntimeState.jobs,
      resumes: mockRuntimeState.resumes,
      userSkills: mockRuntimeState.userSkills,
      resumeDrafts: mockRuntimeState.resumeDrafts,
      recentMatches: [...mockRuntimeState.recentMatches],
      savedMatchEvidence: mockRuntimeState.savedMatchEvidence,
      pendingMilitaryTransitionReviews: [],
    };
    const tokens: string[] = [];
    for (let index = 0; index <= 20; index += 1) {
      const candidate = {
        ...template!,
        id: 100 + index,
        job_hash: `saved-military-${index}`,
      };
      state = { ...state, recentMatches: [...state.recentMatches, candidate] };
      state = handleMockResumeCommand(
        "confirm_saved_match_military_evidence",
        {
          jobHash: candidate.job_hash,
          resumeId: candidate.resume_id,
          kind: "military_service",
        },
        state,
      ).state;
      const result = handleMockResumeCommand(
        "prepare_saved_match_military_transition_review",
        {
          jobHash: candidate.job_hash,
          resumeId: candidate.resume_id,
          branch: "army",
          wording: { ...wording, currentClearance: null },
        },
        state,
      );
      state = result.state;
      tokens.push(result.value as string);
    }

    expect(state.pendingMilitaryTransitionReviews).toHaveLength(20);
    expect(() =>
      handleMockResumeCommand(
        "confirm_saved_match_military_transition_review",
        { token: tokens[0] },
        state,
      ),
    ).toThrow("no longer available");
  });
});
