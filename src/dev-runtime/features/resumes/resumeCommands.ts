import {
  addResumeEducation,
  addResumeExperience,
  addUserSkill,
  analyzeActiveResumeForJob,
  createMockResume,
  createMockResumeDraft,
  deleteResume,
  deleteResumeDraft,
  deleteResumeEducation,
  deleteResumeExperience,
  getMockActiveResume,
  getResumeDraft,
  getResumeTextPreview,
  matchResumeToJob,
  setActiveResume,
  setResumeSkills,
  updateResumeDraft,
  updateUserSkill,
  withSave,
  withoutSave,
} from "./resumeCommandHandlers";
import type {
  MockResumeCommandResult,
  MockResumeCommandState,
} from "./resumeCommandTypes";

export { getMockActiveResume } from "./resumeCommandHandlers";
import { ATS_POWER_WORDS } from "./resumeAnalysis";
import { improveMockBulletPoint } from "./resumeBulletPrompts";
import {
  analyzeMockResumeForJob,
  analyzeMockResumeFormat,
} from "./resumeAnalysisRunner";
import {
  exportMockResumeText,
  getResumeTemplates,
  normalizeBuilderContact,
  renderMockResumeHtml,
} from "./resumeBuilder";
import {
  getArg,
  getNumericArg,
  getResumeIdArg,
  getSkillIdArg,
  getStringArg,
} from "../../mocks/handlers/commandHelpers";
import { extractMockAtsKeywords } from "./resumeKeywordMatching";
import { toMockResumeSummary } from "./resumeSummaryViews";
import { parseMockMatchingProfile } from "./resumeMatchingProfile";
import { handleMockMilitaryTransitionCommand } from "./resumeMilitaryTransitionCommands";

const textEncoder = new TextEncoder();

export function handleMockResumeCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const militaryTransitionResult = handleMockMilitaryTransitionCommand(
    command,
    args,
    state,
    () => getSavedMatchDebuggerMatch(args, state),
  );
  if (militaryTransitionResult) return militaryTransitionResult;

  switch (command) {
    case "get_active_resume": {
      const activeResume = getMockActiveResume(state.resumes);
      return withoutSave(
        state,
        activeResume ? toMockResumeSummary(activeResume) : null,
      );
    }

    case "list_all_resumes":
      return withoutSave(state, state.resumes.map(toMockResumeSummary));

    case "get_resume_text_preview":
      return getResumeTextPreview(args, state);

    case "set_active_resume":
      return setActiveResume(args, state);

    case "select_and_upload_resume":
      return createMockResume(
        "Mock Resume",
        "app-owned://resume-uploads/mock-resume.pdf",
        state,
      );

    case "import_json_resume": {
      const name = getStringArg(args, "name") ?? "Imported Resume";
      return createMockResume(`${name}.json`, `${name}.json`, state);
    }

    case "select_and_import_json_resume":
      return createMockResume(
        "Imported Resume",
        "app-owned://resume-imports/imported-resume.json",
        state,
      );

    case "delete_resume":
      return deleteResume(args, state);

    case "get_user_skills": {
      const resumeId = getResumeIdArg(args);
      return withoutSave(
        state,
        state.userSkills.filter((skill) => skill.resume_id === resumeId),
      );
    }

    case "add_user_skill":
      return addUserSkill(args, state);

    case "update_user_skill":
      return updateUserSkill(args, state);

    case "delete_user_skill": {
      const skillId = getSkillIdArg(args);
      return withSave(
        {
          ...state,
          userSkills: state.userSkills.filter((skill) => skill.id !== skillId),
        },
        undefined,
      );
    }

    case "get_recent_matches": {
      const resumeId = getResumeIdArg(args);
      const limit = getNumericArg(args, "limit") ?? 10;
      return withoutSave(
        state,
        state.recentMatches
          .filter((match) => match.resume_id === resumeId)
          .slice(0, limit),
      );
    }

    case "set_resume_match_feedback": {
      const matchId = getNumericArg(args, "matchId");
      const payload = args?.payload as Record<string, unknown> | undefined;
      const rawLabel = Object.prototype.hasOwnProperty.call(args ?? {}, "label")
        ? args?.label
        : payload?.label;
      if (
        typeof matchId !== "number" ||
        (rawLabel !== null && rawLabel !== "useful" && rawLabel !== "not_relevant") ||
        !state.recentMatches.some((match) => match.id === matchId)
      ) {
        throw new Error("Invalid saved resume match feedback");
      }
      const label: "useful" | "not_relevant" | null = rawLabel;
      const feedback =
        label === null
          ? null
          : { match_id: matchId, label, recorded_at: new Date().toISOString() };
      return withSave(
        {
          ...state,
          recentMatches: state.recentMatches.map((match) =>
            match.id === matchId ? { ...match, feedback } : match,
          ),
        },
        feedback,
      );
    }

    case "get_saved_match_debugger": {
      const match = getSavedMatchDebuggerMatch(args, state);
      return withoutSave(state, mockSavedMatchDebugger(match, state.savedMatchEvidence));
    }

    case "confirm_saved_match_evidence": {
      const match = getSavedMatchDebuggerMatch(args, state);
      const debuggerView = mockSavedMatchDebugger(match, state.savedMatchEvidence);
      const evidenceId = args?.evidenceId;
      const evidence = debuggerView.requirements.flatMap((requirement) => requirement.evidence)
        .find((value) => value.evidence_id === evidenceId);
      if (args?.debuggerId !== debuggerView.debugger_id || !evidence) {
        throw new Error("The selected evidence is no longer available. Refresh the debugger and try again.");
      }
      const identity = savedMatchEvidenceIdentity(match);
      const savedEvidence = state.savedMatchEvidence[identity] ?? emptySavedMatchEvidence();
      return withSave(
        {
          ...state,
          savedMatchEvidence: {
            ...state.savedMatchEvidence,
            [identity]: {
              ...savedEvidence,
              confirmedEvidenceIds: [...new Set([...savedEvidence.confirmedEvidenceIds, evidence.evidence_id])],
            },
          },
        },
        true,
      );
    }

    case "list_saved_match_evidence_packets": {
      const match = getSavedMatchDebuggerMatch(args, state);
      return withoutSave(state, state.savedMatchEvidence[savedMatchEvidenceIdentity(match)]?.packetClaims ?? []);
    }

    case "save_saved_match_evidence_packet": {
      const match = getSavedMatchDebuggerMatch(args, state);
      const reviewedText = getStringArg(args, "reviewedText") ?? "";
      const evidenceIds = Array.isArray(args?.evidenceIds) ? args.evidenceIds : [];
      if (!isMockPacketInputValid(reviewedText, evidenceIds, match, state.savedMatchEvidence)) {
        throw new Error("Review a claim and choose current confirmed evidence before saving.");
      }
      const identity = savedMatchEvidenceIdentity(match);
      const savedEvidence = state.savedMatchEvidence[identity] ?? emptySavedMatchEvidence();
      const claims = savedEvidence.packetClaims;
      const claim: (typeof savedEvidence.packetClaims)[number] = {
        claim_id: `${claims.length + 1}`.padStart(64, "c"),
        reviewed_text: reviewedText.trim(),
        evidence_ids: [...evidenceIds],
        boundaries: [
          "clearance_currentness_unverified",
          "military_civilian_equivalence_unverified",
        ],
      };
      return withSave(
        {
          ...state,
          savedMatchEvidence: {
            ...state.savedMatchEvidence,
            [identity]: { ...savedEvidence, packetClaims: [...claims, claim] },
          },
        },
        claim,
      );
    }

    case "match_resume_to_job":
      return matchResumeToJob(args, state);

    case "create_resume_draft":
      return createMockResumeDraft(state);

    case "get_resume_draft":
      return withoutSave(state, getResumeDraft(args, state) ?? null);

    case "update_resume_contact": {
      const resumeId = getResumeIdArg(args);
      const contact = normalizeBuilderContact(getArg(args, "contact"));
      return updateResumeDraft(
        resumeId,
        (draft) => ({ ...draft, contact }),
        state,
      );
    }

    case "update_resume_summary": {
      const resumeId = getResumeIdArg(args);
      const summary = getStringArg(args, "summary") ?? "";
      return updateResumeDraft(
        resumeId,
        (draft) => ({ ...draft, summary }),
        state,
      );
    }

    case "add_resume_experience":
      return addResumeExperience(args, state);

    case "delete_resume_experience":
      return deleteResumeExperience(args, state);

    case "add_resume_education":
      return addResumeEducation(args, state);

    case "delete_resume_education":
      return deleteResumeEducation(args, state);

    case "set_resume_skills":
      return setResumeSkills(args, state);

    case "delete_resume_draft":
      return deleteResumeDraft(args, state);

    case "list_resume_templates":
      return withoutSave(state, getResumeTemplates());

    case "render_resume_html":
      return withoutSave(state, renderMockResumeHtml(getArg(args, "resume")));

    case "analyze_resume_format":
      return withoutSave(state, analyzeMockResumeFormat(getArg(args, "resume")));

    case "analyze_resume_for_job":
      return withoutSave(
        state,
        analyzeMockResumeForJob(
          getArg(args, "resume"),
          getStringArg(args, "jobDescription") ?? "",
          parseMockMatchingProfile(getArg(args, "matchingProfile")),
        ),
      );

    case "analyze_active_resume_for_job":
      return analyzeActiveResumeForJob(args, state);

    case "get_ats_power_words":
      return withoutSave(state, [...ATS_POWER_WORDS]);

    case "improve_bullet_point":
      return withoutSave(
        state,
        improveMockBulletPoint(args, extractMockAtsKeywords),
      );

    case "export_resume_docx":
      return withoutSave(state, [80, 75, 3, 4, 20, 0, 0, 0]);

    case "export_resume_html":
      return withoutSave(state, renderMockResumeHtml(getArg(args, "resume")));

    case "export_resume_text":
      return withoutSave(state, exportMockResumeText(getArg(args, "resume")));

    default:
      return {
        handled: false,
        shouldSave: false,
        state,
        value: undefined,
      };
  }
}

function getSavedMatchDebuggerMatch(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
) {
  const resumeId = getResumeIdArg(args);
  const jobHash = getStringArg(args, "jobHash");
  if (
    !jobHash ||
    textEncoder.encode(jobHash).byteLength > 128 ||
    /\p{Cc}/u.test(jobHash) ||
    typeof resumeId !== "number" ||
    resumeId <= 0
  ) {
    throw new Error("Choose a saved job and resume before inspecting its evidence.");
  }
  const match = state.recentMatches.find(
    (value) => value.resume_id === resumeId && value.job_hash === jobHash,
  );
  if (!match) throw new Error("Saved match evidence is unavailable.");
  return match;
}

function mockSavedMatchDebugger(
  match: MockResumeCommandState["recentMatches"][number],
  savedMatchEvidence: MockResumeCommandState["savedMatchEvidence"],
) {
  const matchedRequirement = match.matching_skills[0];
  const requirement = matchedRequirement ?? match.missing_skills[0] ?? "Saved match review";
  const hasEvidence = matchedRequirement !== undefined;
  const debuggerId = `${match.id}`.padStart(64, "a");
  const evidenceId = `${match.id}`.padStart(64, "b");
  const confirmedEvidence = new Set(
    savedMatchEvidence[savedMatchEvidenceIdentity(match)]?.confirmedEvidenceIds ?? [],
  );
  return {
    debugger_id: debuggerId,
    requirements: [
      {
        requirement,
        importance: "Required",
        match_state: hasEvidence ? "Direct" : "Missing",
        hard_constraint: !hasEvidence,
        evidence: hasEvidence
          ? [{ evidence_id: evidenceId, confirmed: confirmedEvidence.has(evidenceId) }]
          : [],
        why_not: hasEvidence ? null : "missing_evidence",
        blocking: !hasEvidence,
      },
    ],
  };
}

function isMockPacketInputValid(
  reviewedText: string,
  evidenceIds: unknown[],
  match: MockResumeCommandState["recentMatches"][number],
  savedMatchEvidence: MockResumeCommandState["savedMatchEvidence"],
) {
  if (
    !reviewedText ||
    !reviewedText.trim() ||
    textEncoder.encode(reviewedText).byteLength > 8_192 ||
    /\p{Cc}/u.test(reviewedText) ||
    evidenceIds.length === 0 ||
    evidenceIds.length > 32 ||
    new Set(evidenceIds).size !== evidenceIds.length ||
    evidenceIds.some((id) => typeof id !== "string" || !isMockOpaqueId(id))
  ) {
    return false;
  }
  const confirmedEvidence = new Set(
    savedMatchEvidence[savedMatchEvidenceIdentity(match)]?.confirmedEvidenceIds ?? [],
  );
  return evidenceIds.every((id) => confirmedEvidence.has(id as string));
}

function savedMatchEvidenceIdentity(match: MockResumeCommandState["recentMatches"][number]) {
  return `${match.id}:${match.resume_id}:${match.job_hash}`;
}

function emptySavedMatchEvidence() {
  return {
    confirmedEvidenceIds: [],
    confirmedMilitaryEvidenceKinds: [],
    packetClaims: [],
  };
}

function isMockOpaqueId(value: string) {
  return value.length === 64 && [...value].every((character) => /[a-f0-9]/.test(character));
}
