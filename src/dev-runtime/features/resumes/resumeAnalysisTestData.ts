export type AtsAnalysisResult = {
  overall_score: number;
  keyword_score: number;
  format_score: number;
  completeness_score: number;
  keyword_matches: Array<{
    keyword: string;
    found_in: string[];
    frequency: number;
    importance: "Required" | "Preferred" | "Industry";
  }>;
  missing_keywords: string[];
  missing_keyword_details: Array<{
    keyword: string;
    importance: "Required" | "Preferred" | "Industry";
  }>;
  requirement_reviews: Array<{
    keyword: string;
    importance: "Required" | "Preferred" | "Industry";
    match_state: "Direct" | "Strong" | "Partial" | "Implied" | "Missing";
    evidence_sections: string[];
    hard_constraint: boolean;
    profile_preferred_section?: boolean;
    recommendation: string;
  }>;
  hard_constraint_risks: Array<{
    requirement: string;
    category:
      | "WorkAuthorization"
      | "Citizenship"
      | "SecurityClearance"
      | "LicenseOrCertification"
      | "Education"
      | "Experience"
      | "Language"
      | "BackgroundScreening"
      | "PhysicalRequirement"
      | "Location";
    score_cap: number;
    reason: string;
    action: string;
  }>;
  format_issues: Array<{
    severity: "Critical" | "Warning" | "Info";
    issue: string;
    fix: string;
  }>;
  suggestions: Array<{
    category: "AddKeyword" | "RewordBullet" | "AddSection" | "ReorderContent" | "FormatFix";
    suggestion: string;
    impact: string;
  }>;
  matching_profile?: {
    profession:
      | "technical"
      | "content"
      | "operations"
      | "healthcare"
      | "service"
      | "trades"
      | "education"
      | "sales"
      | "early_career";
    region: "us" | "uk" | "eu" | "india";
  };
};

export const atsResume = {
  contact_info: {
    name: "Casey Smith",
    email: "casey@example.com",
    phone: "",
    location: "Denver, CO",
    linkedin: null,
    github: null,
    website: null,
  },
  summary: "Care coordinator supporting intake, scheduling, and case management.",
  experience: [
    {
      title: "Care Coordinator",
      company: "Community Health Partners",
      location: "Remote",
      start_date: "2021-01",
      end_date: "Present",
      achievements: ["Coordinated client intake", "Improved scheduling follow-up by 40%"],
      current: true,
    },
  ],
  skills: [
    { name: "Scheduling", category: "Operations", proficiency: "advanced" },
    { name: "Case Management", category: "Client Support", proficiency: "advanced" },
  ],
  education: [],
  certifications: [],
  projects: [],
  custom_sections: {},
};
