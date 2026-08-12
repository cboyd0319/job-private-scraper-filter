import screeningAliasTaxonomy from "../../resources/taxonomies/application-screening-aliases.json";

export interface CommonScreeningPattern {
  pattern: string;
  label: string;
  type: "text" | "yes_no" | "textarea";
}

export interface LegacyScreeningPattern {
  pattern: string;
  label: string;
  editablePattern: string;
}

export interface PlainScreeningPatternAlias {
  patterns: string[];
  label: string;
  editablePattern: string;
}

export const COMMON_SCREENING_PATTERNS: CommonScreeningPattern[] = [
  { pattern: "years of experience", label: "Years of experience", type: "text" },
  { pattern: "salary", label: "Salary expectation", type: "text" },
  { pattern: "start date", label: "Start date / Notice period", type: "text" },
  { pattern: "relocate", label: "Willingness to relocate", type: "yes_no" },
  { pattern: "travel", label: "Travel availability", type: "text" },
  { pattern: "reliable transportation", label: "Reliable transportation", type: "yes_no" },
  { pattern: "work authorization", label: "Work authorization", type: "yes_no" },
  { pattern: "US citizen", label: "Citizenship", type: "yes_no" },
  { pattern: "availability", label: "Schedule availability", type: "text" },
  { pattern: "overtime", label: "Overtime availability", type: "yes_no" },
  { pattern: "holiday", label: "Holiday availability", type: "yes_no" },
  { pattern: "managed a team", label: "Management experience", type: "text" },
  { pattern: "sponsorship", label: "Visa sponsorship", type: "yes_no" },
  { pattern: "remote", label: "Remote work preference", type: "text" },
  { pattern: "driver's license", label: "Driver's license", type: "yes_no" },
  { pattern: "security clearance", label: "Security clearance", type: "yes_no" },
  { pattern: "certification", label: "Certification or license", type: "text" },
  { pattern: "background check", label: "Background check", type: "text" },
  { pattern: "drug screen", label: "Drug screen", type: "text" },
  { pattern: "bilingual", label: "Language fluency", type: "text" },
  { pattern: "physical requirements", label: "Physical requirements", type: "text" },
  { pattern: "18 years of age", label: "Age requirement", type: "yes_no" },
  { pattern: "education", label: "Education level", type: "text" },
  { pattern: "cover letter", label: "Cover letter / Why this role", type: "textarea" },
];

export const LEGACY_SCREENING_PATTERNS: LegacyScreeningPattern[] =
  screeningAliasTaxonomy.legacyScreeningPatterns.map(
    ({ pattern, label, editablePattern }) => ({
      pattern,
      label,
      editablePattern,
    }),
  );

export const PLAIN_SCREENING_PATTERN_ALIASES: PlainScreeningPatternAlias[] =
  screeningAliasTaxonomy.plainScreeningPatternAliases;

export const REQUIRES_USER_ANSWER_PATTERNS =
  screeningAliasTaxonomy.requiresUserAnswerPatterns;

export function requiresUserAnswer(question: string): boolean {
  const normalizedQuestion = question
    .toLowerCase()
    .replace(/[^\p{L}\p{N}+#]+/gu, " ")
    .trim()
    .replace(/\s+/g, " ");
  return normalizedQuestion.length > 0 && REQUIRES_USER_ANSWER_PATTERNS.some(
    (pattern) => ` ${normalizedQuestion} `.includes(` ${pattern.toLowerCase()} `),
  );
}

export const LANGUAGE_SCREENING_PATTERNS = [
  /\b(?:bilingual|multilingual)\b/i,
  /\blanguage (?:fluency|proficiency)\b/i,
  /\b(?:fluent|fluency|proficient)\s+(?:in\s+)?(?:spanish|french|mandarin|cantonese|arabic|portuguese|german|japanese|korean)\b/i,
  /\b(?:spanish|french|mandarin|cantonese|arabic|portuguese|german|japanese|korean)[-\s]?(?:speaking|language|fluency|proficiency)\b/i,
] as const;

export const PHYSICAL_REQUIREMENT_PATTERNS = [
  /\blift(?:\s+up\s+to)?\s+\d+\s*(?:pounds?|lbs?)\b/i,
  /\b(?:stand|standing) for long periods?\b/i,
  /\bphysical (?:requirements?|demands?)\b/i,
  /\bable to (?:lift|stand)\b/i,
] as const;

export const MINIMUM_AGE_PATTERNS = [
  /\b(?:must be|at least|minimum age(?: is)?|age requirement(?: is)?)\s*\d{2}\s*(?:\+|years? (?:old|of age))\b/i,
  /\b\d{2}\s*\+?\s*(?:years? old|years? of age)\b/i,
] as const;

export const CITIZENSHIP_SCREENING_PATTERNS = [
  /\b(?:u\.?s\.?|united states)\s+citizens?\b/i,
  /\bcitizenship (?:required|requirement)\b/i,
  /\bmust be (?:a\s+)?(?:u\.?s\.?|united states)\s+citizens?\b/i,
] as const;

export const WORK_AUTHORIZATION_PATTERNS = [
  /\bwork authorization\b/i,
  /\bauthorized to work\b/i,
  /\blegally authorized\b/i,
  /\blegally able to work\b/i,
  /\beligible to work\b/i,
  /\bemployment authorization\b/i,
  /\bEAD\b/,
  /\bgreen card\b/i,
  /\bsponsorship\b/i,
  /\bvisa\b/i,
] as const;

export const LOCATION_SCREENING_PATTERNS = [
  /\brelocat(?:e|ion)\b/i,
  /\bremote\b/i,
  /\bhybrid\b/i,
  /\bon[-\s]?site\b/i,
  /\bcommut(?:e|ing)\b/i,
  /\btravel\b/i,
] as const;

export const TRANSPORTATION_SCREENING_PATTERNS = [
  /\breliable transportation\b/i,
  /\bown transportation\b/i,
  /\bown vehicle\b/i,
  /\bpersonal vehicle\b/i,
  /\baccess to (?:a|your own) vehicle\b/i,
  /\breliable vehicle\b/i,
] as const;

export const DRIVING_RECORD_OR_INSURANCE_PATTERNS = [
  /\bclean driving record\b/i,
  /\bacceptable driving record\b/i,
  /\bdriving record\b/i,
  /\bMVR\b/,
  /\bmotor vehicle record\b/i,
  /\bproof of auto insurance\b/i,
  /\bproof of insurance\b/i,
  /\bauto insurance\b/i,
  /\bcar insurance\b/i,
  /\bvehicle insurance\b/i,
  /\binsured vehicle\b/i,
] as const;

export const CREDENTIAL_SCREENING_PATTERNS = [
  /\blicen[cs]e\b/i,
  /\bcertif(?:ication|ied)\b/i,
  /\bclearance\b/i,
  /\bRN\b/,
  /\bCNA\b/,
  /\bCDL\b/,
  /\bPMP\b/,
  /\bSecurity\+\b/i,
] as const;

export const BACKGROUND_SCREENING_PATTERNS = [
  /\bbackground check\b/i,
  /\bbackground screening\b/i,
  /\bdrug screen\b/i,
  /\bdrug test\b/i,
  /\bpre[-\s]?employment screening\b/i,
] as const;

export const EDUCATION_SCREENING_PATTERNS = [
  /\beducation\b/i,
  /\bdegree\b/i,
  /\bbachelor'?s?\b/i,
  /\bmaster'?s?\b/i,
  /\bhigh school\b/i,
  /\bdiploma\b/i,
] as const;

export const EXPERIENCE_ANSWER_SCREENING_PATTERNS = [
  /\byears? of experience\b/i,
  /\bexperience\b/i,
] as const;

export const EXPERIENCE_REQUIREMENT_PATTERNS = [
  /\b\d+\+?\s*(?:years|yrs)\b(?!\s*(?:of age|old))/i,
  /\byears? of experience\b/i,
  /\bexperience required\b/i,
] as const;

export const MANAGEMENT_EXPERIENCE_PATTERNS = [
  /\bmanagement experience\b/i,
  /\bpeople management\b/i,
  /\bteam management\b/i,
  /\bteam supervision\b/i,
  /\bsupervis(?:or|ory|ion|ing|ed)\b/i,
  /\bmanaged\s+(?:a\s+)?team\b/i,
  /\bmanaged (?:staff|people|employees)\b/i,
  /\b(?:shift|crew) lead\b/i,
  /\blead worker\b/i,
  /\blead experience\b/i,
] as const;

export const SALARY_HISTORY_PATTERNS = [
  /\bsalary history\b/i,
  /\bcompensation history\b/i,
  /\bpay history\b/i,
  /\bcurrent (?:salary|compensation|pay(?: rate)?)\b/i,
  /\bprevious (?:salary|compensation|pay(?: rate)?)\b/i,
  /\bprior (?:salary|compensation|pay(?: rate)?)\b/i,
  /\bpast (?:salary|compensation|pay(?: rate)?)\b/i,
] as const;

export const SALARY_EXPECTATION_PATTERNS = [
  /\bsalary expectations?\b/i,
  /\bcompensation expectations?\b/i,
  /\bpay expectations?\b/i,
  /\bexpected (?:salary|compensation|pay)\b/i,
  /\bdesired (?:salary|compensation|pay)\b/i,
  /\btarget (?:salary|compensation|pay)\b/i,
] as const;

export const SAVED_SALARY_ANSWER_PATTERNS = [
  ...SALARY_EXPECTATION_PATTERNS,
  /\bsalary\b/i,
  /\bcompensation\b/i,
  /\bpay\b/i,
] as const;

export const AVAILABILITY_SCREENING_PATTERNS = [
  /\bavailability\b/i,
  /\bavailable\b/i,
  /\bstart date\b/i,
  /\bnotice period\b/i,
  /\bschedule\b/i,
  /\bshift\b/i,
  /\bweekend\b/i,
  /\bovernight\b/i,
  /\bovertime\b/i,
  /\bholiday\b/i,
] as const;

export const SCREENING_ANSWER_LABEL_RULES = [
  { label: "travel", patterns: [/\btravel\b/i] },
  { label: "relocation", patterns: [/\brelocat/i] },
  { label: "commute", patterns: [/\bcommut/i] },
  { label: "location", patterns: [/\bremote\b|\bhybrid\b|\bon[-\s]?site\b/i] },
  { label: "citizenship", patterns: CITIZENSHIP_SCREENING_PATTERNS },
  { label: "driving record or insurance", patterns: DRIVING_RECORD_OR_INSURANCE_PATTERNS },
  { label: "transportation", patterns: TRANSPORTATION_SCREENING_PATTERNS },
  { label: "credential", patterns: CREDENTIAL_SCREENING_PATTERNS },
  { label: "screening", patterns: BACKGROUND_SCREENING_PATTERNS },
  { label: "language", patterns: LANGUAGE_SCREENING_PATTERNS },
  { label: "physical requirement", patterns: PHYSICAL_REQUIREMENT_PATTERNS },
  { label: "age requirement", patterns: MINIMUM_AGE_PATTERNS },
  { label: "education", patterns: EDUCATION_SCREENING_PATTERNS },
  { label: "management experience", patterns: MANAGEMENT_EXPERIENCE_PATTERNS },
  { label: "experience", patterns: EXPERIENCE_ANSWER_SCREENING_PATTERNS },
  { label: "salary-history", patterns: SALARY_HISTORY_PATTERNS },
  { label: "salary", patterns: SAVED_SALARY_ANSWER_PATTERNS },
  { label: "availability", patterns: AVAILABILITY_SCREENING_PATTERNS },
] as const;

export const HARD_SCREENING_ANSWER_PATTERNS = [
  /\bcitizenship\b|\bUS citizen\b|\bU\.S\. citizen\b/i,
  /\bwork authorization\b|\bauthorized to work\b|\bsponsorship\b|\bvisa\b/i,
  /\bclean driving record\b|\bacceptable driving record\b|\bdriving record\b|\bMVR\b|\bmotor vehicle record\b|\bproof of auto insurance\b|\bproof of insurance\b|\bauto insurance\b|\bcar insurance\b|\bvehicle insurance\b|\binsured vehicle\b/i,
  /\breliable transportation\b|\btransportation\b|\bvehicle\b/i,
  /\brelocat(?:e|ion)\b|\btravel\b/i,
  /\beducation\b|\bdegree\b|\bdiploma\b|\bGED\b|\bbachelor'?s?\b/i,
  /\bsalary\b|\bcompensation\b|\bpay\b|\bstart date\b|\bnotice period\b/i,
  /\bavailability\b|\bschedule\b|\bshift\b|\bweekend\b|\bovertime\b|\bholiday\b/i,
  /\bmanaged a team\b|\bmanagement experience\b|\bsupervisory\b|\bsupervisor\b/i,
  /\bbilingual\b|\bmultilingual\b|\blanguage\b|\bfluenc(?:y|e|t)\b/i,
  /\bbackground check\b|\bbackground screening\b|\bdrug screen\b|\bdrug test\b|\bpre[-\s]?employment screening\b/i,
  /\bphysical requirements?\b|\blift\b|\bstanding\b|\bstand for\b/i,
  /\b18 years of age\b|\bminimum age\b|\bage requirement\b/i,
  /\bdriver'?s license\b|\blicen[cs]e\b|\bcertification\b|\bclearance\b/i,
] as const;
