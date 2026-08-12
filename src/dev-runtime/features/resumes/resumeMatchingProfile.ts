import {
  isResumeMatchingProfile,
  type ProfessionMatchingProfile,
  type RegionalMatchingProfile,
  type ResumeMatchingProfile,
} from "../../../features/resumes/shared/atsAnalysisContracts";
import type { MockAtsResumeSections } from "./resumeAnalysisSections";

export function parseMockMatchingProfile(
  value: unknown,
): ResumeMatchingProfile | undefined {
  if (value === undefined) return undefined;
  if (!isResumeMatchingProfile(value)) {
    throw new Error("Invalid resume matching profile");
  }
  return value;
}

export function normalizeMockJobDescription(
  jobDescription: string,
  region: RegionalMatchingProfile,
): string {
  return normalizeRegionalText(jobDescription, region);
}

export function normalizeMockResumeSections(
  sections: MockAtsResumeSections,
  region: RegionalMatchingProfile,
): MockAtsResumeSections {
  const normalize = (text: string) => normalizeRegionalText(text, region);
  return {
    summary: normalize(sections.summary),
    experience: sections.experience.map(normalize),
    currentExperience: sections.currentExperience.map(normalize),
    recentExperience: sections.recentExperience.map(normalize),
    pastExperience: sections.pastExperience.map(normalize),
    skills: sections.skills.map(normalize),
    education: sections.education.map(normalize),
    certifications: sections.certifications.map(normalize),
    projects: sections.projects.map(normalize),
    allText: normalize(sections.allText),
  };
}

function normalizeRegionalText(
  text: string,
  region: RegionalMatchingProfile,
): string {
  if (region === "us") return text;
  return text
    .replace(/\bprogramme evaluation\b/gi, "program evaluation")
    .replace(/\bdriver's licence\b/gi, "driver's license")
    .replace(/\bdrivers licence\b/gi, "drivers license")
    .replace(/\bdriver licence\b/gi, "driver license");
}

export function mockProfilePrefersSection(
  profession: ProfessionMatchingProfile,
  sections: string[],
): boolean {
  return sections.some((section) => {
    switch (profession) {
      case "technical":
        return [
          "experience",
          "current experience",
          "recent experience",
          "projects",
          "certifications",
        ].includes(section);
      case "content":
        return [
          "experience",
          "current experience",
          "recent experience",
          "projects",
          "publications",
        ].includes(section);
      case "operations":
      case "service":
      case "sales":
        return [
          "experience",
          "current experience",
          "recent experience",
          "projects",
        ].includes(section);
      case "healthcare":
      case "trades":
      case "education":
        return [
          "experience",
          "current experience",
          "recent experience",
          "education",
          "certifications",
          "licenses",
        ].includes(section);
      case "early_career":
        return ["projects", "education", "certifications"].includes(section);
    }
  });
}
