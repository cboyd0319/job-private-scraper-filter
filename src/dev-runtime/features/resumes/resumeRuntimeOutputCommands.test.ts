import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke, resetMockData } from "../../mocks/handlers";
import { loadMockState, mockRuntimeState } from "../../mocks/runtimeState";
import { setupResumeRuntimeMocks } from "./resumeRuntimeTestSupport";

type MockJobSummary = {
  hash: string;
  score: number;
};

type MockMatchResult = {
  id: number;
  overall_match_score: number;
  skills_match_score: number | null;
  experience_match_score: number | null;
  education_match_score: number | null;
};

type MockMatchFeedback = {
  match_id: number;
  label: "useful" | "not_relevant";
  recorded_at: string;
};

type MockSavedMatchDebugger = {
  debugger_id: string;
  requirements: Array<{
    evidence: Array<{
      evidence_id: string;
      confirmed: boolean;
    }>;
  }>;
};

type MockSavedMatchPacket = {
  reviewed_text: string;
  boundaries: string[];
};

type ResumeTextPreview = {
  resume_id: number;
  name: string;
  has_text: boolean;
  text_preview: string;
  text_chars: number;
  is_truncated: boolean;
};

describe("mock resume runtime output commands", () => {
  beforeEach(setupResumeRuntimeMocks);

  it("returns match scores as backend-compatible fractions", async () => {
    const [job] = await mockInvoke<MockJobSummary[]>("get_jobs", {});
    const resumeId = await mockInvoke<number>("select_and_upload_resume");
    const match = await mockInvoke<MockMatchResult>("match_resume_to_job", {
      resumeId,
      jobHash: job.hash,
    });

    expect(match.overall_match_score).toBe(job.score);
    expect(match.skills_match_score).toBe(job.score);
    expect(match.experience_match_score).toBe(Number((job.score - 0.05).toFixed(2)));
    expect(match.education_match_score).toBeNull();
    expect(match.overall_match_score).toBeGreaterThanOrEqual(0);
    expect(match.overall_match_score).toBeLessThanOrEqual(1);
    expect(match.skills_match_score).toBeGreaterThanOrEqual(0);
    expect(match.skills_match_score).toBeLessThanOrEqual(1);
    expect(match.experience_match_score).toBeGreaterThanOrEqual(0);
    expect(match.experience_match_score).toBeLessThanOrEqual(1);
  });

  it("returns a readable preview without path details", async () => {
    const resumeId = await mockInvoke<number>("select_and_upload_resume");
    const summary = await mockInvoke<Record<string, unknown>>("get_active_resume");
    const preview = await mockInvoke<ResumeTextPreview>("get_resume_text_preview", { resumeId });

    expect(preview).toMatchObject({
      resume_id: resumeId,
      name: "Mock Resume",
      has_text: true,
      is_truncated: false,
    });
    expect(preview.text_preview).toContain("Mock Resume");
    expect(preview.text_chars).toBe(preview.text_preview.length);
    expect(JSON.stringify(summary)).not.toContain("app-owned://");
    expect(JSON.stringify(preview)).not.toContain("app-owned://");
    expect(JSON.stringify(preview)).not.toContain("file_path");
  });

  it("stores and clears only a closed local match label", async () => {
    const [job] = await mockInvoke<MockJobSummary[]>("get_jobs", {});
    const resumeId = await mockInvoke<number>("select_and_upload_resume");
    const match = await mockInvoke<MockMatchResult>("match_resume_to_job", {
      resumeId,
      jobHash: job.hash,
    });

    const feedback = await mockInvoke<MockMatchFeedback>("set_resume_match_feedback", {
      matchId: match.id,
      label: "not_relevant",
    });

    expect(feedback).toMatchObject({
      match_id: match.id,
      label: "not_relevant",
    });
    expect(Object.keys(feedback).sort()).toEqual(["label", "match_id", "recorded_at"]);
    await expect(
      mockInvoke("set_resume_match_feedback", {
        matchId: match.id,
        label: "maybe",
      }),
    ).rejects.toThrow("Invalid saved resume match feedback");
    await expect(
      mockInvoke("set_resume_match_feedback", {
        matchId: match.id,
        label: null,
      }),
    ).resolves.toBeNull();
  });

  it("mirrors saved-match evidence commands with exact opaque references", async () => {
    const [job] = await mockInvoke<MockJobSummary[]>("get_jobs", {});
    const resumeId = await mockInvoke<number>("select_and_upload_resume");
    await mockInvoke("add_user_skill", {
      resumeId,
      skill: { skill_name: "Patient scheduling" },
    });
    const match = await mockInvoke<MockMatchResult>("match_resume_to_job", { resumeId, jobHash: job.hash });

    const debuggerView = await mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
      jobHash: job.hash,
      resumeId,
    });
    const evidenceId = debuggerView.requirements[0]?.evidence[0]?.evidence_id;

    expect(debuggerView.debugger_id).toMatch(/^[a-f0-9]{64}$/);
    expect(evidenceId).toMatch(/^[a-f0-9]{64}$/);
    await expect(
      mockInvoke("confirm_saved_match_evidence", {
        jobHash: job.hash,
        resumeId,
        debuggerId: debuggerView.debugger_id,
        evidenceId,
      }),
    ).resolves.toBe(true);
    await expect(
      mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
        jobHash: job.hash,
        resumeId,
      }),
    ).resolves.toMatchObject({
      requirements: [{ evidence: [{ evidence_id: evidenceId, confirmed: true }] }],
    });
    await expect(
      mockInvoke<MockSavedMatchPacket>("save_saved_match_evidence_packet", {
        jobHash: job.hash,
        resumeId,
        reviewedText: "I reviewed the scheduling evidence.",
        evidenceIds: [evidenceId],
      }),
    ).resolves.toMatchObject({
      reviewed_text: "I reviewed the scheduling evidence.",
      boundaries: [
        "clearance_currentness_unverified",
        "military_civilian_equivalence_unverified",
      ],
    });
    await expect(
      mockInvoke<MockSavedMatchPacket[]>("list_saved_match_evidence_packets", {
        jobHash: job.hash,
        resumeId,
      }),
    ).resolves.toMatchObject([{ reviewed_text: "I reviewed the scheduling evidence." }]);
    const savedMatchEvidence = structuredClone(mockRuntimeState.savedMatchEvidence);
    mockRuntimeState.savedMatchEvidence = {};
    loadMockState();
    expect(mockRuntimeState.savedMatchEvidence).toEqual(savedMatchEvidence);
    await mockInvoke("set_resume_match_feedback", { matchId: match.id, label: "useful" });
    await expect(
      mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
        jobHash: job.hash,
        resumeId,
      }),
    ).resolves.toMatchObject({
      requirements: [{ evidence: [{ evidence_id: evidenceId, confirmed: true }] }],
    });
    await expect(
      mockInvoke("confirm_saved_match_evidence", {
        jobHash: job.hash,
        resumeId,
        debuggerId: debuggerView.debugger_id,
        evidenceId: "f".repeat(64),
      }),
    ).rejects.toThrow("selected evidence");
  });

  it("shows a hard blocker when a saved match has no mock evidence", async () => {
    mockRuntimeState.recentMatches.push({
      id: 99,
      resume_id: 1,
      job_hash: "no-evidence",
      job_title: "Support Lead",
      company: "Example",
      overall_match_score: 0,
      skills_match_score: 0,
      experience_match_score: 0,
      education_match_score: null,
      matching_skills: [],
      missing_skills: ["Role-specific evidence"],
      gap_analysis: null,
      feedback: null,
      created_at: "2026-07-21T00:00:00Z",
    });

    await expect(
      mockInvoke<MockSavedMatchDebugger & { requirements: Array<Record<string, unknown>> }>(
        "get_saved_match_debugger",
        { jobHash: "no-evidence", resumeId: 1 },
      ),
    ).resolves.toMatchObject({
      requirements: [{ evidence: [], hard_constraint: true, why_not: "missing_evidence", blocking: true }],
    });
  });

  it("uses UTF-8 byte limits and rejects C1 controls for saved-match packet input", async () => {
    const [job] = await mockInvoke<MockJobSummary[]>("get_jobs", {});
    const resumeId = await mockInvoke<number>("select_and_upload_resume");
    await mockInvoke("add_user_skill", { resumeId, skill: { skill_name: "Patient scheduling" } });
    await mockInvoke("match_resume_to_job", { resumeId, jobHash: job.hash });
    const debuggerView = await mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
      jobHash: job.hash,
      resumeId,
    });
    const evidenceId = debuggerView.requirements[0]?.evidence[0]?.evidence_id ?? "";
    await mockInvoke("confirm_saved_match_evidence", {
      jobHash: job.hash,
      resumeId,
      debuggerId: debuggerView.debugger_id,
      evidenceId,
    });

    await expect(mockInvoke("get_saved_match_debugger", {
      jobHash: "é".repeat(65),
      resumeId,
    })).rejects.toThrow("Choose a saved job");
    await expect(mockInvoke("save_saved_match_evidence_packet", {
      jobHash: job.hash,
      resumeId,
      reviewedText: "é".repeat(4097),
      evidenceIds: [evidenceId],
    })).rejects.toThrow("Review a claim");
    await expect(mockInvoke("save_saved_match_evidence_packet", {
      jobHash: job.hash,
      resumeId,
      reviewedText: "Reviewed\u0085claim",
      evidenceIds: [evidenceId],
    })).rejects.toThrow("Review a claim");
  });

  it("clears saved match evidence on reset even when a match id is reused", async () => {
    const match = {
      id: 77,
      resume_id: 1,
      job_hash: "reused-match",
      job_title: "Support Lead",
      company: "Example",
      overall_match_score: 0.5,
      skills_match_score: 0.5,
      experience_match_score: 0.5,
      education_match_score: null,
      matching_skills: ["Customer support"],
      missing_skills: [],
      gap_analysis: null,
      feedback: null,
      created_at: "2026-07-21T00:00:00Z",
    };
    mockRuntimeState.recentMatches.push(match);
    const debuggerView = await mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
      jobHash: match.job_hash,
      resumeId: match.resume_id,
    });
    const evidenceId = debuggerView.requirements[0]?.evidence[0]?.evidence_id ?? "";
    await mockInvoke("confirm_saved_match_evidence", {
      jobHash: match.job_hash,
      resumeId: match.resume_id,
      debuggerId: debuggerView.debugger_id,
      evidenceId,
    });

    resetMockData();
    mockRuntimeState.recentMatches.push(match);
    await expect(mockInvoke<MockSavedMatchDebugger>("get_saved_match_debugger", {
      jobHash: match.job_hash,
      resumeId: match.resume_id,
    })).resolves.toMatchObject({ requirements: [{ evidence: [{ confirmed: false }] }] });
  });
});
