import { requiresUserAnswer } from "../../shared/applicationScreeningTaxonomy";

export interface MockApplicationProfile {
  id: number;
  fullName: string;
  email: string;
  phone: string | null;
  linkedinUrl: string | null;
  githubUrl: string | null;
  portfolioUrl: string | null;
  websiteUrl: string | null;
  defaultResumeId: number | null;
  hasResumeFile: boolean;
  resumeFileName: string | null;
  defaultCoverLetterTemplate: string | null;
  usWorkAuthorized: boolean;
  requiresSponsorship: boolean;
  maxApplicationsPerDay: number;
  requireManualApproval: boolean;
  createdAt: string;
  updatedAt: string;
}

export type MockApplicationProfilePreview = Pick<
  MockApplicationProfile,
  | "fullName"
  | "email"
  | "phone"
  | "linkedinUrl"
  | "githubUrl"
  | "portfolioUrl"
  | "websiteUrl"
  | "usWorkAuthorized"
  | "requiresSponsorship"
>;

export type MockApplicationProfileEdit = Pick<
  MockApplicationProfile,
  | "fullName"
  | "email"
  | "phone"
  | "linkedinUrl"
  | "githubUrl"
  | "portfolioUrl"
  | "websiteUrl"
  | "hasResumeFile"
  | "resumeFileName"
  | "usWorkAuthorized"
  | "requiresSponsorship"
  | "maxApplicationsPerDay"
  | "requireManualApproval"
>;

export interface MockScreeningAnswer {
  id: number;
  questionPattern: string;
  answer: string;
  answerType: string | null;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
  timesUsed?: number;
  timesModified?: number;
  confidenceScore?: number;
  lastUsedAt?: string | null;
}

export interface MockAnswerSuggestion {
  answer: string;
  confidence: number;
  source: {
    type: "manual";
    answerId: number;
  };
  timesUsed: number;
  timesModified: number;
  lastUsedDaysAgo: number | null;
  modificationRate: number;
}

const LEGACY_SCREENING_PATTERN_ALIASES: Record<string, string[]> = {
  "(?i)authorized.*work.*(united states|us|usa)": [
    "authorized to work",
    "authorized work",
    "work authorization",
  ],
  "(?i)require.*sponsor.*work": [
    "require sponsorship to work",
    "need sponsorship to work",
    "sponsorship",
  ],
  "(?i)require.*sponsor.*(now|future)": [
    "require sponsorship",
    "need sponsorship",
    "visa sponsorship",
  ],
  "(?i)18.*years.*age": ["18 years of age", "18 years age"],
  "(?i)drug.*test": ["drug test", "drug screen"],
  "(?i)background.*check": ["background check"],
  "(?i)security.*clearance": ["security clearance"],
  "(?i)willing.*relocate": ["willing to relocate", "willing relocate", "relocate"],
  "(?i)notice.*period": ["notice period"],
  "(?i)salary.*expectation": ["salary expectation", "expected salary"],
};

export function getDefaultMockApplicationProfile(): MockApplicationProfile {
  const now = "2026-05-19T16:00:00.000Z";
  return {
    id: 1,
    fullName: "Jordan Lee",
    email: "jordan@example.com",
    phone: "+1 (555) 123-4567",
    linkedinUrl: "https://linkedin.com/in/jordanlee",
    githubUrl: null,
    portfolioUrl: "https://jordanlee.example.com/work",
    websiteUrl: "https://jordanlee.example.com",
    defaultResumeId: null,
    hasResumeFile: false,
    resumeFileName: null,
    defaultCoverLetterTemplate: null,
    usWorkAuthorized: true,
    requiresSponsorship: false,
    maxApplicationsPerDay: 10,
    requireManualApproval: true,
    createdAt: now,
    updatedAt: now,
  };
}

export function getDefaultMockScreeningAnswers(): MockScreeningAnswer[] {
  const now = "2026-05-19T16:00:00.000Z";
  return [
    {
      id: 1,
      questionPattern: "work authorized",
      answer: "Yes",
      answerType: "yes_no",
      notes: "US work authorization",
      timesUsed: 4,
      timesModified: 0,
      confidenceScore: 0.92,
      lastUsedAt: now,
      createdAt: now,
      updatedAt: now,
    },
  ];
}

export function getMockApplicationProfilePreview(
  profile: MockApplicationProfile | null,
): MockApplicationProfilePreview | null {
  if (!profile) return null;

  return {
    fullName: profile.fullName,
    email: profile.email,
    phone: profile.phone,
    linkedinUrl: profile.linkedinUrl,
    githubUrl: profile.githubUrl,
    portfolioUrl: profile.portfolioUrl,
    websiteUrl: profile.websiteUrl,
    usWorkAuthorized: profile.usWorkAuthorized,
    requiresSponsorship: profile.requiresSponsorship,
  };
}

export function getMockApplicationProfileEdit(
  profile: MockApplicationProfile | null,
): MockApplicationProfileEdit | null {
  if (!profile) return null;

  return {
    fullName: profile.fullName,
    email: profile.email,
    phone: profile.phone,
    linkedinUrl: profile.linkedinUrl,
    githubUrl: profile.githubUrl,
    portfolioUrl: profile.portfolioUrl,
    websiteUrl: profile.websiteUrl,
    hasResumeFile: profile.hasResumeFile,
    resumeFileName: profile.resumeFileName,
    usWorkAuthorized: profile.usWorkAuthorized,
    requiresSponsorship: profile.requiresSponsorship,
    maxApplicationsPerDay: profile.maxApplicationsPerDay,
    requireManualApproval: profile.requireManualApproval,
  };
}

export function normalizeMockApplicationProfile(
  value: Partial<MockApplicationProfile>,
): MockApplicationProfile {
  const defaults = getDefaultMockApplicationProfile();
  return {
    ...defaults,
    ...value,
    id: typeof value.id === "number" ? value.id : defaults.id,
    fullName: typeof value.fullName === "string" ? value.fullName : defaults.fullName,
    email: typeof value.email === "string" ? value.email : defaults.email,
    phone: nullableString(value.phone),
    linkedinUrl: nullableString(value.linkedinUrl),
    githubUrl: nullableString(value.githubUrl),
    portfolioUrl: nullableString(value.portfolioUrl),
    websiteUrl: nullableString(value.websiteUrl),
    defaultResumeId: nullableNumber(value.defaultResumeId),
    hasResumeFile: typeof value.hasResumeFile === "boolean" ? value.hasResumeFile : defaults.hasResumeFile,
    resumeFileName: nullableString(value.resumeFileName),
    defaultCoverLetterTemplate: nullableString(value.defaultCoverLetterTemplate),
    usWorkAuthorized: typeof value.usWorkAuthorized === "boolean" ? value.usWorkAuthorized : defaults.usWorkAuthorized,
    requiresSponsorship: typeof value.requiresSponsorship === "boolean" ? value.requiresSponsorship : defaults.requiresSponsorship,
    maxApplicationsPerDay: typeof value.maxApplicationsPerDay === "number" ? value.maxApplicationsPerDay : defaults.maxApplicationsPerDay,
    requireManualApproval: typeof value.requireManualApproval === "boolean" ? value.requireManualApproval : defaults.requireManualApproval,
    createdAt: typeof value.createdAt === "string" ? value.createdAt : defaults.createdAt,
    updatedAt: typeof value.updatedAt === "string" ? value.updatedAt : defaults.updatedAt,
  };
}

export function buildMockApplicationProfileFromInput(
  input: Record<string, unknown>,
  existingProfile: MockApplicationProfile | null,
): MockApplicationProfile {
  const existing = existingProfile ?? getDefaultMockApplicationProfile();
  const now = new Date().toISOString();
  const selectedResumeFileName = displayFileNameFromResumeToken(input.resume_file_token);
  const clearResumeFile = booleanValue(input.clear_resume_file, false);

  return {
    id: existing.id,
    fullName: String(input.full_name ?? ""),
    email: String(input.email ?? ""),
    phone: nullableString(input.phone),
    linkedinUrl: nullableString(input.linkedin_url),
    githubUrl: nullableString(input.github_url),
    portfolioUrl: nullableString(input.portfolio_url),
    websiteUrl: nullableString(input.website_url),
    defaultResumeId: nullableNumber(input.default_resume_id),
    hasResumeFile: clearResumeFile
      ? false
      : selectedResumeFileName !== null || existing.hasResumeFile,
    resumeFileName: clearResumeFile
      ? null
      : selectedResumeFileName ?? existing.resumeFileName,
    defaultCoverLetterTemplate: nullableString(input.default_cover_letter_template),
    usWorkAuthorized: booleanValue(input.us_work_authorized, true),
    requiresSponsorship: booleanValue(input.requires_sponsorship, false),
    maxApplicationsPerDay: numberValue(input.max_applications_per_day, 10),
    requireManualApproval: booleanValue(input.require_manual_approval, true),
    createdAt: existing.createdAt,
    updatedAt: now,
  };
}

export function normalizeMockScreeningAnswer(
  value: Partial<MockScreeningAnswer>,
): MockScreeningAnswer {
  const now = new Date().toISOString();
  return {
    id: typeof value.id === "number" ? value.id : 1,
    questionPattern: typeof value.questionPattern === "string" ? value.questionPattern : "",
    answer: typeof value.answer === "string" ? value.answer : "",
    answerType: nullableString(value.answerType),
    notes: nullableString(value.notes),
    timesUsed: typeof value.timesUsed === "number" ? value.timesUsed : undefined,
    timesModified: typeof value.timesModified === "number" ? value.timesModified : undefined,
    confidenceScore: typeof value.confidenceScore === "number" ? value.confidenceScore : undefined,
    lastUsedAt: nullableString(value.lastUsedAt),
    createdAt: typeof value.createdAt === "string" ? value.createdAt : now,
    updatedAt: typeof value.updatedAt === "string" ? value.updatedAt : now,
  };
}

export function upsertMockScreeningAnswer(
  args: Record<string, unknown> | undefined,
  screeningAnswers: MockScreeningAnswer[],
): MockScreeningAnswer[] {
  const questionPattern =
    getStringArg(args, "questionPattern") ?? getStringArg(args, "question_pattern") ?? "";
  const answer = getStringArg(args, "answer") ?? "";
  const answerType =
    getStringArg(args, "answerType") ?? getStringArg(args, "answer_type") ?? "text";
  const notes = nullableString(getArg(args, "notes"));
  const existing = screeningAnswers.find(
    (screeningAnswer) => screeningAnswer.questionPattern === questionPattern,
  );
  const now = new Date().toISOString();

  if (existing) {
    return screeningAnswers.map((screeningAnswer) =>
      screeningAnswer.id === existing.id
        ? {
            ...screeningAnswer,
            answer,
            answerType,
            notes,
            updatedAt: now,
          }
        : screeningAnswer,
    );
  }

  return [
    ...screeningAnswers,
    {
      id: getNextId(screeningAnswers),
      questionPattern,
      answer,
      answerType,
      notes,
      timesUsed: 0,
      timesModified: 0,
      confidenceScore: 1,
      lastUsedAt: null,
      createdAt: now,
      updatedAt: now,
    },
  ];
}

export function getMockSuggestedAnswers(
  args: Record<string, unknown> | undefined,
  screeningAnswers: MockScreeningAnswer[],
): MockAnswerSuggestion[] {
  const question = getStringArg(args, "question") ?? "";
  const limit = getNumericArg(args, "limit") ?? 5;

  if (requiresUserAnswer(question)) return [];

  return screeningAnswers
    .filter((answer) => !requiresUserAnswer(answer.questionPattern))
    .filter((answer) => screeningPatternMatchesQuestion(answer.questionPattern, question))
    .slice(0, limit)
    .map((answer) => ({
      answer: answer.answer,
      confidence: answer.confidenceScore ?? 0.8,
      source: {
        type: "manual",
        answerId: answer.id,
      },
      timesUsed: answer.timesUsed ?? 0,
      timesModified: answer.timesModified ?? 0,
      lastUsedDaysAgo: answer.lastUsedAt ? 1 : null,
      modificationRate: answer.timesUsed && answer.timesUsed > 0
        ? (answer.timesModified ?? 0) / answer.timesUsed
        : 0,
    }));
}

function displayFileNameFromPath(value: unknown): string | null {
  if (typeof value !== "string" || value.trim().length === 0) {
    return null;
  }

  return value.trim().split(/[\\/]/).filter(Boolean).pop() ?? "Selected resume";
}

function displayFileNameFromResumeToken(value: unknown): string | null {
  if (typeof value !== "string" || value.trim().length === 0) {
    return null;
  }

  const token = value.trim();
  const tokenParts = token.split("--");
  const displayName = tokenParts.slice(1).join("--").trim();
  if (tokenParts.length >= 2 && displayName.length > 0) {
    return displayName;
  }

  return displayFileNameFromPath(token);
}

function screeningPatternMatchesQuestion(savedWording: string, question: string): boolean {
  const normalizedQuestion = normalizeScreeningMatchText(question);
  if (!normalizedQuestion) return false;

  const questionTokens = new Set(normalizedQuestion.split(/\s+/));

  return getScreeningMatchCandidates(savedWording).some((candidate) => {
    const normalizedCandidate = normalizeScreeningMatchText(candidate);
    if (!normalizedCandidate) return false;
    if (normalizedQuestion.includes(normalizedCandidate)) return true;

    const candidateTokens = normalizedCandidate.split(/\s+/);
    return candidateTokens.every((token) => questionTokens.has(token));
  });
}

function getScreeningMatchCandidates(savedWording: string): string[] {
  const trimmed = savedWording.trim();
  if (!trimmed) return [];

  const candidates = [
    trimmed,
    ...(LEGACY_SCREENING_PATTERN_ALIASES[trimmed] ?? []),
  ];

  if (looksLikeLegacyScreeningPattern(trimmed)) {
    const simplified = simplifyLegacyScreeningPattern(trimmed);
    if (simplified) {
      candidates.push(simplified);
      candidates.push(
        ...simplified
          .split("|")
          .map((candidate) => candidate.trim())
          .filter(Boolean),
      );
    }
  }

  return [...new Set(candidates)];
}

function normalizeScreeningMatchText(value: string): string {
  return value
    .toLowerCase()
    .replace(/\bu\s*\.\s*s\s*\.?\b/g, "us")
    .replace(/[\u2019']/g, "")
    .replace(/[^a-z0-9+#]+/g, " ")
    .trim()
    .replace(/\s+/g, " ");
}

function looksLikeLegacyScreeningPattern(savedWording: string): boolean {
  const lower = savedWording.toLowerCase();
  return lower.startsWith("(?i)") ||
    lower.includes(".*") ||
    lower.includes(".+") ||
    lower.includes("\\s") ||
    lower.includes("|") ||
    lower.includes("\\b");
}

function simplifyLegacyScreeningPattern(savedWording: string): string {
  const withoutInlineFlag = savedWording.slice(0, 4).toLowerCase() === "(?i)"
    ? savedWording.slice(4)
    : savedWording;

  return withoutInlineFlag
    .replace(/\\s[+*]/g, " ")
    .replace(/\\b/g, " ")
    .replace(/\.\*/g, " ")
    .replace(/\.\+/g, " ")
    .replace(/[()[\]{}^$?*\\]/g, " ")
    .trim()
    .replace(/\s+/g, " ");
}

function getNextId(items: Array<{ id: number }>): number {
  return Math.max(0, ...items.map((item) => item.id)) + 1;
}

function getNumericArg(args: Record<string, unknown> | undefined, key: string): number | undefined {
  const value = getArg(args, key);
  return typeof value === "number" ? value : undefined;
}

function getStringArg(
  args: Record<string, unknown> | undefined,
  key: string,
): string | undefined {
  const value = getArg(args, key);
  return typeof value === "string" ? value : undefined;
}

function getArg(
  args: Record<string, unknown> | undefined,
  key: string,
): unknown {
  return args ? args[key] : undefined;
}

function booleanValue(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function nullableNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function numberValue(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}
