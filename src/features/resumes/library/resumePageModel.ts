import {
  DEFAULT_SKILL_STRENGTH,
  RESUME_SKILL_CATEGORIES,
  SKILL_STRENGTH_COLORS as SHARED_SKILL_STRENGTH_COLORS,
  SKILL_STRENGTH_LABELS as SHARED_SKILL_STRENGTH_LABELS,
  SKILL_STRENGTH_OPTIONS,
} from "../shared/resumeSkillUiTaxonomy";

type BadgeVariant = "sentinel" | "alert" | "surface" | "success" | "danger";

export type { BadgeVariant };

export { DEFAULT_SKILL_STRENGTH, SKILL_STRENGTH_OPTIONS };

export const SKILL_STRENGTH_COLORS: Record<string, BadgeVariant> =
  SHARED_SKILL_STRENGTH_COLORS;
export const SKILL_STRENGTH_LABELS: Record<string, string> =
  SHARED_SKILL_STRENGTH_LABELS;
export const SKILL_CATEGORIES = [...RESUME_SKILL_CATEGORIES];

export interface ResumeData {
  id: number;
  name: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  format_label?: string;
  has_readable_text?: boolean;
  readable_text_chars?: number;
}

export interface ResumeTextPreview {
  resume_id: number;
  name: string;
  has_text: boolean;
  text_preview: string;
  text_chars: number;
  is_truncated: boolean;
}

export interface ReadablePreviewCheck {
  label: string;
  detail: string;
  variant: BadgeVariant;
}

export interface ResumeMatchingPreference {
  enabled: boolean;
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

export interface SkillUpdate {
  skill_name?: string;
  skill_category?: string | null;
  proficiency_level?: string | null;
  years_experience?: number | null;
}

export interface NewSkill {
  skill_name: string;
  skill_category?: string;
  proficiency_level?: string;
  years_experience?: number;
}

export interface MatchResult {
  id: number;
  resume_id: number;
  job_hash: string;
  job_title: string;
  company: string;
  overall_match_score: number;
  skills_match_score?: number | null;
  experience_match_score?: number | null;
  education_match_score?: number | null;
  matching_skills: string[];
  missing_skills: string[];
  gap_analysis: string | null;
  feedback: ResumeMatchFeedback | null;
  created_at: string;
}

export type ResumeMatchFeedbackLabel = "useful" | "not_relevant";

export interface ResumeMatchFeedback {
  match_id: number;
  label: ResumeMatchFeedbackLabel;
  recorded_at: string;
}

export function normalizeSkillStrength(value: string | null | undefined) {
  return value?.trim().toLowerCase() ?? "";
}

export function getSkillStrengthLabel(value: string | null | undefined) {
  const normalized = normalizeSkillStrength(value);
  if (!normalized) {
    return "Not set";
  }
  return SKILL_STRENGTH_LABELS[normalized] ?? value?.trim() ?? "Not set";
}

export const getSkillStrengthColor = (strength: string | null): BadgeVariant =>
  SKILL_STRENGTH_COLORS[normalizeSkillStrength(strength)] ?? "surface";

export function getSkillSourceLabel(source: string) {
  const normalized = source.trim().toLowerCase();
  if (normalized === "manual") {
    return "Added by you";
  }
  if (normalized === "resume" || normalized === "import") {
    return "Found in resume";
  }
  return "Saved skill";
}

export function isScoreFraction(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function optionalTrimmedText(value: string | null | undefined): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}

export function optionalYearsValue(value: number | null | undefined): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function isResumeMatchingEnabled(
  preference: ResumeMatchingPreference | null | undefined,
) {
  return preference?.enabled === true;
}

export function getResumeFormatLabel(resume: ResumeData) {
  return resume.format_label?.trim() || "Resume file";
}

export function isPdfFormatLabel(formatLabel: string | null | undefined) {
  return formatLabel?.trim().toLowerCase() === "pdf";
}

export function getReadableTextLabel(resume: ResumeData) {
  if (resume.has_readable_text === true) {
    return "Readable text ready";
  }
  if (resume.has_readable_text === false) {
    return "No readable text found";
  }
  return "Readable text not checked";
}

export function getReadableTextDescription(resume: ResumeData) {
  if (resume.has_readable_text === true) {
    const count = typeof resume.readable_text_chars === "number"
      ? resume.readable_text_chars
      : 0;
    return `${count.toLocaleString()} characters available for local review.`;
  }
  if (resume.has_readable_text === false) {
    if (isPdfFormatLabel(resume.format_label)) {
      return [
        "Follow employer file instructions first.",
        "This PDF may be scanned or image-only, so JobSentinel could not find selectable text.",
        "If no format is named, export a readable PDF, DOCX, TXT, Markdown, or HTML resume.",
      ].join(" ");
    }

    return [
      "Follow employer file instructions first.",
      "If no format is named, add a PDF, DOCX, TXT, Markdown, or HTML resume with readable text.",
    ].join(" ");
  }
  return "Open the readable-text preview to check what JobSentinel can read.";
}

export function getEmptyReadablePreviewMessage(resume: ResumeData | null) {
  if (isPdfFormatLabel(resume?.format_label)) {
    return [
      "No selectable text found in this PDF.",
      "Follow employer file instructions first.",
      "If no format is named, try exporting a readable PDF, DOCX, TXT, Markdown, or HTML resume, or use a resume app export.",
    ].join(" ");
  }

  return [
    "No readable text found.",
    "Follow employer file instructions first.",
    "If no format is named, try a readable PDF, DOCX, TXT, Markdown, or HTML resume, or use a resume app export.",
  ].join(" ");
}

export function getReadableTextBadgeVariant(resume: ResumeData): BadgeVariant {
  if (resume.has_readable_text === true) return "success";
  if (resume.has_readable_text === false) return "danger";
  return "surface";
}

export function getReadablePreviewChecklist(
  preview: ResumeTextPreview | null,
): ReadablePreviewCheck[] {
  if (!preview) return [];

  const hasText = preview.has_text && preview.text_chars > 0;
  return [
    {
      label: hasText ? "Text found" : "Needs readable text",
      detail: hasText
        ? `${preview.text_chars.toLocaleString()} readable characters available.`
        : "Try a readable PDF, DOCX, TXT, Markdown, or HTML resume, or use a resume app export.",
      variant: hasText ? "success" : "danger",
    },
    {
      label: "Employer format first",
      detail: "Follow the employer's requested file type before choosing another readable format.",
      variant: "surface",
    },
    {
      label: "Important details need text",
      detail:
        "Keep contact details, roles, skills, and credentials as selectable text. Photos, logos, or image-only sections may not be read consistently.",
      variant: "surface",
    },
    {
      label: hasText
        ? preview.is_truncated
          ? "Preview clipped"
          : "Preview complete"
        : "Preview not available",
      detail: hasText
        ? preview.is_truncated
          ? "Only the first part is shown here; local review still uses saved readable text."
          : "The visible preview includes all readable text JobSentinel received."
        : "No readable text is available for preview.",
      variant: hasText ? (preview.is_truncated ? "alert" : "success") : "danger",
    },
  ];
}

const LEGACY_MATCH_PREFIX = "\u2713";
const LEGACY_MISSING_PREFIX = "\u2717";

export function parseGapAnalysisLine(line: string) {
  const trimmed = line.trim();
  const matchPattern = /^(?:Matching:|Matching Skills\b)/i;
  const missingPattern = /^(?:Missing:|Missing Skills\b)/i;

  if (trimmed.startsWith(LEGACY_MATCH_PREFIX) || matchPattern.test(trimmed)) {
    return {
      text: trimmed
        .replace(LEGACY_MATCH_PREFIX, "")
        .replace(matchPattern, "")
        .trim(),
      status: "match" as const,
    };
  }

  if (trimmed.startsWith(LEGACY_MISSING_PREFIX) || missingPattern.test(trimmed)) {
    return {
      text: trimmed
        .replace(LEGACY_MISSING_PREFIX, "")
        .replace(missingPattern, "")
        .trim(),
      status: "missing" as const,
    };
  }

  return { text: trimmed, status: "neutral" as const };
}
