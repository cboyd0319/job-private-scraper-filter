import type {
  ATSAnalysis,
  BackendATSAnalysis,
  ContactInfo,
  JsonResumeData,
  ResumeAnalysisInput,
  ResumeData,
  SkillEntry,
  StructuredResume,
  StructuredSkillCategory,
} from "./resumeBuilderData";

function groupSkills(skills: SkillEntry[]): StructuredSkillCategory[] {
  const grouped = skills.reduce<Record<string, SkillEntry[]>>((acc, skill) => {
    const category = skill.category || "General";
    acc[category] = [...(acc[category] ?? []), skill];
    return acc;
  }, {});

  return Object.entries(grouped).map(([name, values]) => ({
    name,
    skills: values.map((skill) => ({
      name: skill.name,
      proficiency: skill.proficiency,
      years_experience: skill.years_experience ?? null,
    })),
  }));
}

function jsonResumeProfiles(contact: ContactInfo): JsonResumeData["basics"]["profiles"] {
  return [
    contact.linkedin
      ? {
          network: "LinkedIn",
          username: "",
          url: contact.linkedin,
        }
      : null,
    contact.github
      ? {
          network: "GitHub",
          username: "",
          url: contact.github,
        }
      : null,
  ].filter((profile): profile is JsonResumeData["basics"]["profiles"][number] =>
    Boolean(profile),
  );
}

function jsonResumeSkillLevel(skills: SkillEntry[]) {
  const strengthRank: Record<NonNullable<SkillEntry["proficiency"]>, number> = {
    beginner: 1,
    intermediate: 2,
    advanced: 3,
    expert: 4,
  };
  const strongest = skills.reduce<NonNullable<SkillEntry["proficiency"]> | null>(
    (best, skill) => {
      if (!skill.proficiency) return best;
      if (!best) return skill.proficiency;

      return strengthRank[skill.proficiency] > strengthRank[best]
        ? skill.proficiency
        : best;
    },
    null,
  );

  return strongest ?? "intermediate";
}

function toJsonResumeSkillGroups(skills: SkillEntry[]): JsonResumeData["skills"] {
  const grouped = skills.reduce<Record<string, SkillEntry[]>>((acc, skill) => {
    const category = skill.category || "General";
    acc[category] = [...(acc[category] ?? []), skill];
    return acc;
  }, {});

  return Object.entries(grouped).map(([name, values]) => ({
    name,
    level: jsonResumeSkillLevel(values),
    keywords: values.map((skill) => skill.name),
  }));
}

export function toStructuredResume(resume: ResumeData): StructuredResume {
  return {
    personal: resume.contact,
    summary: resume.summary || null,
    experience: resume.experience.map((experience) => ({
      title: experience.title,
      company: experience.company,
      location: experience.location,
      start_date: experience.start_date,
      end_date: experience.end_date,
      is_current: !experience.end_date,
      achievements: experience.achievements,
    })),
    education: resume.education.map(({ id: _id, ...education }) => ({
      ...education,
      field_of_study: null,
    })),
    skills: groupSkills(resume.skills),
    certifications: resume.certifications,
    projects: resume.projects,
    clearance: resume.clearance ?? null,
    military_info: resume.military_info ?? null,
  };
}

export function toResumeAnalysisInput(
  resume: ResumeData,
  revision = resume.updated_at,
): ResumeAnalysisInput {
  return {
    resume: toStructuredResume(resume),
    custom_sections: {},
    evidence_snapshot: {
      source_id: `resume-draft:${resume.id}`,
      revision,
    },
  };
}

export function toJsonResumeData(resume: ResumeData): JsonResumeData {
  return {
    basics: {
      name: resume.contact.name,
      label: "",
      image: "",
      email: resume.contact.email,
      phone: resume.contact.phone ?? "",
      url: resume.contact.website ?? "",
      summary: resume.summary,
      location: {
        address: resume.contact.location ?? "",
        postalCode: "",
        city: "",
        countryCode: "",
        region: "",
      },
      profiles: jsonResumeProfiles(resume.contact),
    },
    work: resume.experience.map((experience) => ({
      name: experience.company,
      position: experience.title,
      url: "",
      startDate: experience.start_date,
      endDate: experience.end_date ?? "",
      summary: "",
      highlights: experience.achievements,
    })),
    education: resume.education.map((education) => ({
      institution: education.institution,
      url: "",
      area: "",
      studyType: education.degree,
      startDate: "",
      endDate: education.graduation_date ?? "",
      score: education.gpa ?? "",
      courses: education.honors,
    })),
    certificates: resume.certifications.map((certification) => ({
      name: certification.name,
      date: certification.date_obtained ?? "",
      issuer: certification.issuer,
      url: "",
    })),
    skills: toJsonResumeSkillGroups(resume.skills),
    projects: resume.projects.map((project) => ({
      name: project.name,
      description: project.description,
      highlights: [],
      keywords: project.technologies,
      startDate: project.start_date ?? "",
      endDate: project.end_date ?? "",
      url: project.url ?? "",
      roles: [],
      entity: "",
      type: "",
    })),
  };
}

export function normalizeAtsAnalysis(
  analysis: BackendATSAnalysis,
): ATSAnalysis {
  return {
    format_score: analysis.format_score,
    issues:
      analysis.issues ??
      analysis.format_issues?.map((issue) => issue.issue) ??
      [],
    recommendations:
      analysis.recommendations ??
      analysis.suggestions?.map((suggestion) => suggestion.suggestion) ??
      [],
  };
}
