import {
  SKILL_PROFICIENCY_LABELS,
  SKILL_PROFICIENCY_VALUES,
  type SkillProficiency,
} from "../shared/resumeSkillUiTaxonomy";

export interface Resume {
  id: number;
  name: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface UserSkill {
  id: number;
  resume_id: number;
  skill_name: string;
  skill_category: string | null;
  confidence_score: number;
  years_experience: number | null;
  proficiency_level: string | null;
  source: string;
}

export interface ATSAnalysis {
  format_score: number;
  issues: string[];
  recommendations: string[];
}

export interface ContactInfo {
  name: string;
  email: string;
  phone: string | null;
  linkedin: string | null;
  github: string | null;
  location: string | null;
  website: string | null;
}

export interface Experience {
  id: number;
  title: string;
  company: string;
  location: string | null;
  start_date: string;
  end_date: string | null;
  achievements: string[];
}

export interface Education {
  id: number;
  degree: string;
  institution: string;
  location: string | null;
  graduation_date: string | null;
  gpa: string | null;
  honors: string[];
}

export interface SkillEntry {
  name: string;
  category: string;
  proficiency: SkillProficiency | null;
  years_experience?: number | null;
}

export interface Certification {
  name: string;
  issuer: string;
  date_obtained: string | null;
  expiration_date: string | null;
  credential_id: string | null;
}

export interface Project {
  name: string;
  description: string;
  technologies: string[];
  url: string | null;
  start_date: string | null;
  end_date: string | null;
}

export interface ResumeData {
  id: number;
  contact: ContactInfo;
  summary: string;
  experience: Experience[];
  education: Education[];
  skills: SkillEntry[];
  certifications: Certification[];
  projects: Project[];
  clearance?: string | null;
  military_info?: string | null;
  created_at: string;
  updated_at: string;
}

export type TemplateId =
  | "Classic"
  | "Modern"
  | "Technical"
  | "Executive"
  | "Military";

export interface Template {
  id: TemplateId;
  name: string;
  description: string;
  preview_image: string;
}

export interface StructuredSkill {
  name: string;
  proficiency: string | null;
  years_experience: number | null;
}

export interface StructuredSkillCategory {
  name: string;
  skills: StructuredSkill[];
}

export interface StructuredResume {
  personal: ContactInfo;
  summary: string | null;
  experience: Array<Omit<Experience, "id"> & { is_current: boolean }>;
  education: Array<
    Omit<Education, "id"> & { field_of_study: string | null }
  >;
  skills: StructuredSkillCategory[];
  certifications: Certification[];
  projects: Project[];
  clearance: string | null;
  military_info: string | null;
}

export interface ResumeAnalysisInput {
  resume: StructuredResume;
  custom_sections: Record<string, string[]>;
  evidence_snapshot?: {
    source_id: string;
    revision: string;
  };
}

export interface JsonResumeData {
  basics: {
    name: string;
    label: string;
    image: string;
    email: string;
    phone: string;
    url: string;
    summary: string;
    location: {
      address: string;
      postalCode: string;
      city: string;
      countryCode: string;
      region: string;
    };
    profiles: Array<{
      network: string;
      username: string;
      url: string;
    }>;
  };
  work: Array<{
    name: string;
    position: string;
    url: string;
    startDate: string;
    endDate: string;
    summary: string;
    highlights: string[];
  }>;
  education: Array<{
    institution: string;
    url: string;
    area: string;
    studyType: string;
    startDate: string;
    endDate: string;
    score: string;
    courses: string[];
  }>;
  certificates: Array<{
    name: string;
    date: string;
    issuer: string;
    url: string;
  }>;
  skills: Array<{
    name: string;
    level: string;
    keywords: string[];
  }>;
  projects: Array<{
    name: string;
    description: string;
    highlights: string[];
    keywords: string[];
    startDate: string;
    endDate: string;
    url: string;
    roles: string[];
    entity: string;
    type: string;
  }>;
}

export interface BackendATSAnalysis {
  format_score: number;
  issues?: string[];
  recommendations?: string[];
  format_issues?: Array<{ issue: string }>;
  suggestions?: Array<{ suggestion: string }>;
}

export const STEPS = [
  { id: 1, name: "Contact", description: "Personal information" },
  { id: 2, name: "Summary", description: "Professional summary" },
  { id: 3, name: "Experience", description: "Work history" },
  { id: 4, name: "Education", description: "Academic background" },
  { id: 5, name: "Skills", description: "Role and people skills" },
  { id: 6, name: "Preview", description: "Choose template" },
  { id: 7, name: "Export", description: "Download resume" },
];

export const SKILL_STRENGTH_VALUES = SKILL_PROFICIENCY_VALUES;

export const SKILL_STRENGTH_LABELS: Record<
  (typeof SKILL_STRENGTH_VALUES)[number],
  string
> = SKILL_PROFICIENCY_LABELS;

export function getSkillStrengthLabel(
  proficiency: SkillEntry["proficiency"],
) {
  return proficiency ? SKILL_STRENGTH_LABELS[proficiency] : "";
}
