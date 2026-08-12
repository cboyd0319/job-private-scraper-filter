import { analyzeMockResumeForJob } from "./resumeAnalysisRunner";
import {
  getEmptyBuilderContact,
  normalizeBuilderEducation,
  normalizeBuilderExperience,
  normalizeBuilderSkill,
} from "./resumeBuilder";
import {
  getArg,
  getNextId,
  getNumericArg,
  getResumeIdArg,
  getSkillIdArg,
  getStringArg,
  hasOwnInputKey,
  normalizeSkillInput,
  skillYearsOrNull,
  toScoreFraction,
  trimmedStringOrNull,
} from "../../mocks/handlers/commandHelpers";
import { toMockResumeTextPreview } from "./resumeSummaryViews";
import type {
  MockBuilderSkill,
  MockMatchResult,
  MockResumeData,
  MockResumeDraft,
  MockUserSkill,
} from "../../mocks/handlers/types";
import type {
  MockResumeCommandResult,
  MockResumeCommandState,
} from "./resumeCommandTypes";
import { parseMockMatchingProfile } from "./resumeMatchingProfile";

export function getMockActiveResume(
  resumes: MockResumeData[],
): MockResumeData | null {
  return resumes.find((resume) => resume.is_active) ?? null;
}

export function getResumeTextPreview(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const selectedResume = state.resumes.find((resume) => resume.id === resumeId);
  if (!selectedResume) {
    throw new Error("Resume not found");
  }
  return withoutSave(state, toMockResumeTextPreview(selectedResume));
}

export function setActiveResume(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  if (
    typeof resumeId !== "number" ||
    !state.resumes.some((resume) => resume.id === resumeId)
  ) {
    throw new Error("Resume not found");
  }

  return withSave(
    {
      ...state,
      resumes: state.resumes.map((resume) => ({
        ...resume,
        is_active: resume.id === resumeId,
      })),
    },
    undefined,
  );
}

export function createMockResume(
  name: string,
  filePath: string,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const now = new Date().toISOString();
  const id = getNextId(state.resumes);
  const parsedText = [
    name,
    "Care coordinator supporting intake, scheduling, and case management.",
    "Skills: patient scheduling, community outreach, documentation.",
  ].join("\n");

  return withSave(
    {
      ...state,
      resumes: [
        ...state.resumes.map((resume) => ({ ...resume, is_active: false })),
        {
          id,
          name,
          file_path: filePath,
          parsed_text: parsedText,
          is_active: true,
          created_at: now,
          updated_at: now,
        },
      ],
    },
    id,
  );
}

export function deleteResume(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  if (typeof resumeId !== "number") {
    return withoutSave(state, undefined);
  }

  const wasActive = state.resumes.some(
    (resume) => resume.id === resumeId && resume.is_active,
  );
  let resumes = state.resumes.filter((resume) => resume.id !== resumeId);
  if (wasActive && resumes.length > 0) {
    resumes = resumes.map((resume, index) => ({
      ...resume,
      is_active: index === 0,
    }));
  }

  return withSave(
    {
      ...state,
      resumes,
      userSkills: state.userSkills.filter(
        (skill) => skill.resume_id !== resumeId,
      ),
      recentMatches: state.recentMatches.filter(
        (match) => match.resume_id !== resumeId,
      ),
    },
    undefined,
  );
}

export function addUserSkill(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const skill = normalizeSkillInput(getArg(args, "skill"));
  const skillName = trimmedStringOrNull(skill.skill_name);
  if (typeof resumeId !== "number" || !skillName) {
    return withoutSave(state, undefined);
  }

  const newSkill: MockUserSkill = {
    id: getNextId(state.userSkills),
    resume_id: resumeId,
    skill_name: skillName,
    skill_category: trimmedStringOrNull(skill.skill_category),
    confidence_score: 1,
    years_experience: skillYearsOrNull(skill.years_experience),
    proficiency_level: trimmedStringOrNull(skill.proficiency_level),
    source: "manual",
  };

  return withSave(
    { ...state, userSkills: [...state.userSkills, newSkill] },
    newSkill.id,
  );
}

export function updateUserSkill(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const skillId = getSkillIdArg(args);
  const updates = normalizeSkillInput(getArg(args, "updates"));
  if (
    typeof skillId !== "number" ||
    !state.userSkills.some((skill) => skill.id === skillId)
  ) {
    throw new Error("Skill not found");
  }

  return withSave(
    {
      ...state,
      userSkills: state.userSkills.map((skill) =>
        skill.id === skillId
          ? {
              ...skill,
              skill_name:
                trimmedStringOrNull(updates.skill_name) ?? skill.skill_name,
              skill_category: hasOwnInputKey(updates, "skill_category")
                ? trimmedStringOrNull(updates.skill_category)
                : skill.skill_category,
              proficiency_level: hasOwnInputKey(updates, "proficiency_level")
                ? trimmedStringOrNull(updates.proficiency_level)
                : skill.proficiency_level,
              years_experience: hasOwnInputKey(updates, "years_experience")
                ? skillYearsOrNull(updates.years_experience)
                : skill.years_experience,
              source: "manual",
            }
          : skill,
      ),
    },
    undefined,
  );
}

export function matchResumeToJob(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const jobHash = getStringArg(args, "jobHash") ?? getStringArg(args, "job_hash");
  const job = state.jobs.find((item) => item.hash === jobHash);
  if (typeof resumeId !== "number" || !jobHash || !job) {
    return withoutSave(state, undefined);
  }

  const skills = state.userSkills
    .filter((skill) => skill.resume_id === resumeId)
    .map((skill) => skill.skill_name);
  const matchScore = toScoreFraction(job.score);
  const match: MockMatchResult = {
    id: getNextId(state.recentMatches),
    resume_id: resumeId,
    job_hash: jobHash,
    job_title: job.title,
    company: job.company,
    overall_match_score: matchScore,
    skills_match_score: matchScore,
    experience_match_score: Math.max(0, Number((matchScore - 0.05).toFixed(2))),
    education_match_score: null,
    matching_skills: skills.slice(0, 3),
    missing_skills: ["Role-specific evidence"],
    gap_analysis: "Matching: Existing skills align\nMissing: Add one role-specific example",
    feedback: null,
    created_at: new Date().toISOString(),
  };

  return withSave(
    {
      ...state,
      recentMatches: [
        match,
        ...state.recentMatches.filter((item) => item.job_hash !== jobHash),
      ],
    },
    match,
  );
}

export function createMockResumeDraft(
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const now = new Date().toISOString();
  const id = getNextId(state.resumeDrafts);

  return withSave(
    {
      ...state,
      resumeDrafts: [
        ...state.resumeDrafts,
        {
          id,
          contact: getEmptyBuilderContact(),
          summary: "",
          experience: [],
          education: [],
          skills: [],
          certifications: [],
          projects: [],
          created_at: now,
          updated_at: now,
        },
      ],
    },
    id,
  );
}

export function getResumeDraft(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeDraft | undefined {
  const resumeId = getResumeIdArg(args);
  return state.resumeDrafts.find((draft) => draft.id === resumeId);
}

export function updateResumeDraft(
  resumeId: number | undefined,
  updater: (draft: MockResumeDraft) => MockResumeDraft,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  if (typeof resumeId !== "number") throw new Error("Resume draft not found");

  const found = state.resumeDrafts.some((draft) => draft.id === resumeId);
  if (!found) throw new Error("Resume draft not found");

  return withSave(
    {
      ...state,
      resumeDrafts: state.resumeDrafts.map((draft) =>
        draft.id === resumeId
          ? updater({ ...draft, updated_at: new Date().toISOString() })
          : draft,
      ),
    },
    undefined,
  );
}

export function addResumeExperience(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const draft = getResumeDraft(args, state);
  const newId = getNextId(draft?.experience ?? []);
  const experience = normalizeBuilderExperience(getArg(args, "experience"), newId);
  const result = updateResumeDraft(
    resumeId,
    (current) => ({
      ...current,
      experience: [...current.experience, { ...experience, id: newId }],
    }),
    state,
  );
  return { ...result, value: newId };
}

export function deleteResumeExperience(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const experienceId =
    getNumericArg(args, "experienceId") ?? getNumericArg(args, "experience_id");
  const draft = getResumeDraft(args, state);
  if (!draft || !draft.experience.some((experience) => experience.id === experienceId)) {
    throw new Error("Experience entry not found");
  }
  return updateResumeDraft(
    resumeId,
    (draft) => ({
      ...draft,
      experience: draft.experience.filter(
        (experience) => experience.id !== experienceId,
      ),
    }),
    state,
  );
}

export function addResumeEducation(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const draft = getResumeDraft(args, state);
  const newId = getNextId(draft?.education ?? []);
  const education = normalizeBuilderEducation(getArg(args, "education"), newId);
  const result = updateResumeDraft(
    resumeId,
    (current) => ({
      ...current,
      education: [...current.education, { ...education, id: newId }],
    }),
    state,
  );
  return { ...result, value: newId };
}

export function deleteResumeEducation(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const educationId =
    getNumericArg(args, "educationId") ?? getNumericArg(args, "education_id");
  const draft = getResumeDraft(args, state);
  if (!draft || !draft.education.some((education) => education.id === educationId)) {
    throw new Error("Education entry not found");
  }
  return updateResumeDraft(
    resumeId,
    (draft) => ({
      ...draft,
      education: draft.education.filter((education) => education.id !== educationId),
    }),
    state,
  );
}

export function setResumeSkills(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  const rawSkills = getArg(args, "skills");
  const skills = Array.isArray(rawSkills)
    ? rawSkills
        .map(normalizeBuilderSkill)
        .filter((skill): skill is MockBuilderSkill => !!skill)
    : [];
  return updateResumeDraft(
    resumeId,
    (draft) => ({ ...draft, skills }),
    state,
  );
}

export function deleteResumeDraft(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const resumeId = getResumeIdArg(args);
  if (
    typeof resumeId !== "number" ||
    !state.resumeDrafts.some((draft) => draft.id === resumeId)
  ) {
    throw new Error("Resume draft not found");
  }

  return withSave(
    {
      ...state,
      resumeDrafts: state.resumeDrafts.filter((draft) => draft.id !== resumeId),
    },
    undefined,
  );
}

export function analyzeActiveResumeForJob(
  args: Record<string, unknown> | undefined,
  state: MockResumeCommandState,
): MockResumeCommandResult {
  const activeResume = getMockActiveResume(state.resumes);
  if (!activeResume) {
    throw new Error("Choose or add a resume before reviewing job fit.");
  }

  const readableText = (activeResume.parsed_text ?? "").trim();
  if (!readableText) {
    throw new Error("JobSentinel could not find readable text in the active resume.");
  }

  return withoutSave(
    state,
    analyzeMockResumeForJob(
      {
        summary: readableText,
        experience: [],
        skills: [],
        education: [],
        certifications: [],
        projects: [],
        custom_sections: {},
      },
      getStringArg(args, "jobDescription") ?? "",
      parseMockMatchingProfile(getArg(args, "matchingProfile")),
    ),
  );
}

export function withoutSave(
  state: MockResumeCommandState,
  value: unknown,
): MockResumeCommandResult {
  return {
    handled: true,
    shouldSave: false,
    state,
    value,
  };
}

export function withSave(
  state: MockResumeCommandState,
  value: unknown,
): MockResumeCommandResult {
  return {
    handled: true,
    shouldSave: true,
    state,
    value,
  };
}
