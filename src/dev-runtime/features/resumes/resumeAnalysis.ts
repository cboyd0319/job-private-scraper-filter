import resumeKeywordTaxonomy from "../../../../resources/taxonomies/resume-keywords.json";
import { getMockCredentialCatalogTerms } from "./resumeCredentialTaxonomy";
import type { ResumeMatchingProfile } from "../../../features/resumes/shared/atsAnalysisContracts";

export type MockKeywordImportance = "Required" | "Preferred" | "Industry";
export type MockIssueSeverity = "Critical" | "Warning" | "Info";
export type MockRequirementMatchState = "Direct" | "Strong" | "Partial" | "Implied" | "Missing";
export type MockHardConstraintCategory =
  | "WorkAuthorization"
  | "Citizenship"
  | "SecurityClearance"
  | "LicenseOrCertification"
  | "Education"
  | "Experience"
  | "Language"
  | "Age"
  | "BackgroundScreening"
  | "PhysicalRequirement"
  | "Location";
export type MockSuggestionCategory =
  | "AddKeyword"
  | "RewordBullet"
  | "AddSection"
  | "ReorderContent"
  | "FormatFix";

export interface MockKeywordMatch {
  keyword: string;
  found_in: string[];
  frequency: number;
  importance: MockKeywordImportance;
}

export interface MockFormatIssue {
  severity: MockIssueSeverity;
  issue: string;
  fix: string;
}

export interface MockRequirementReview {
  keyword: string;
  importance: MockKeywordImportance;
  match_state: MockRequirementMatchState;
  evidence_sections: string[];
  hard_constraint: boolean;
  profile_preferred_section?: boolean;
  recommendation: string;
}

export interface MockHardConstraintRisk {
  requirement: string;
  category: MockHardConstraintCategory;
  score_cap: number;
  reason: string;
  action: string;
}

export interface MockAtsSuggestion {
  category: MockSuggestionCategory;
  suggestion: string;
  impact: string;
}

export interface MockAtsAnalysisResult {
  overall_score: number;
  keyword_score: number;
  format_score: number;
  completeness_score: number;
  keyword_matches: MockKeywordMatch[];
  missing_keywords: string[];
  missing_keyword_details: MockAtsKeyword[];
  requirement_reviews: MockRequirementReview[];
  hard_constraint_risks: MockHardConstraintRisk[];
  format_issues: MockFormatIssue[];
  suggestions: MockAtsSuggestion[];
  matching_profile?: ResumeMatchingProfile;
}

export interface MockAtsKeyword {
  keyword: string;
  importance: MockKeywordImportance;
}

export const MOCK_HUMAN_LANGUAGES = resumeKeywordTaxonomy.humanLanguages;

export const ATS_POWER_WORDS = resumeKeywordTaxonomy.bulletPowerWords;

type SupplementalKeywordGroup = {
  canonical: string;
  catalogTerms?: string[];
};

function getSupplementalKeywordCatalogTerms(): string[] {
  return [
    ...new Set(
      (resumeKeywordTaxonomy.supplementalKeywordGroups as SupplementalKeywordGroup[])
        .flatMap((group) =>
          group.catalogTerms && group.catalogTerms.length > 0
            ? group.catalogTerms
            : [group.canonical]
        ),
    ),
  ];
}

export const ATS_KNOWN_KEYWORDS: readonly string[] = [
  ...getSupplementalKeywordCatalogTerms(),
  ...getMockCredentialCatalogTerms(),
  "forecasting",
  "workflow improvement",
  "quality assurance",
  "qa",
  "customer service",
  "guest service",
  "guest services",
  "front desk",
  "front-desk",
  "reception",
  "receptionist",
  "crm",
  "salesforce",
  "leadership",
  "scheduling",
  "calendar management",
  "appointment setting",
  "case management",
  "case coordination",
  "client intake",
  "onboarding",
  "new hire orientation",
  "employee orientation",
  "customer support",
  "project coordination",
  "bookkeeping",
  "bookkeeper",
  "quickbooks",
  "qbo",
  "accounts payable",
  "accounts receivable",
  "a/p",
  "a/r",
  "inventory",
  "inventory control",
  "inventory management",
  "stock control",
  "stock management",
  "stockroom",
  "logistics",
  "shipping",
  "receiving",
  "procurement",
  "purchasing",
  "vendor management",
  "supplier management",
  "budgeting",
  "budget tracking",
  "program evaluation",
  "data entry",
  "bilingual",
  "data analysis",
  "data analytics",
  "analytics",
  "training",
  "patient-care",
  "security clearance",
  "work authorization",
  "bachelor's degree",
  "high school diploma",
  "ged",
  "high school equivalency",
  "degree",
  "patient care",
  "medication administration",
  "medication-administration",
  "vital sign",
  "vital-sign",
  "vital signs",
  "vital-signs",
  "care plan",
  "care-plan",
  "care plans",
  "care-plans",
  "medical record",
  "medical-record",
  "medical records",
  "medical-records",
  "charting",
  "lesson planning",
  "classroom management",
  "curriculum",
  "iep",
  "student support",
  "student services",
  "parent communication",
  "family communication",
  "guardian communication",
  "forklift",
  "welding",
  "equipment maintenance",
  "safety inspections",
  "cash handling",
  "cashier",
  "point of sale",
  "pos system",
  "pos systems",
  "public benefits",
  "reconciliation",
  "billing",
  "invoicing",
  "financial reporting",
];
