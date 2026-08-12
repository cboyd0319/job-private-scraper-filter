import type { AtsAnalysisResult } from "./AtsLiveScorePanel";

export const mockResumeData = {
  id: 7,
  contact: {
    name: "Jordan Lee",
    email: "jordan@example.com",
    phone: "555-1234",
    linkedin: "linkedin.com/in/jordanlee",
    github: null,
    location: "Chicago, IL",
    website: null,
  },
  summary: "Customer support lead with 5+ years helping clients...",
  experience: [
    {
      id: 1,
      title: "Customer Support Lead",
      company: "CareBridge Services",
      location: "Chicago",
      start_date: "2020-01",
      end_date: null,
      achievements: ["Coached team of 5", "Improved response time by 40%"],
    },
  ],
  education: [
    {
      id: 1,
      degree: "BA Communications",
      institution: "State University",
      location: "Chicago",
      graduation_date: "2018-05",
      gpa: "3.8",
      honors: ["Magna Cum Laude"],
    },
  ],
  skills: [
    { name: "Customer service", category: "Service", proficiency: "expert" as const },
    { name: "Scheduling tools", category: "Tools", proficiency: "advanced" as const },
  ],
  certifications: [],
  projects: [],
  created_at: "2026-07-19T12:00:00Z",
  updated_at: "2026-07-19T13:00:00Z",
};

export const mockAnalysis: AtsAnalysisResult = {
  overall_score: 75,
  keyword_score: 80,
  format_score: 70,
  completeness_score: 75,
  keyword_matches: [
    {
      keyword: "Customer service",
      importance: "Required",
      found_in: ["skills"],
      frequency: 1,
    },
    {
      keyword: "Scheduling",
      importance: "Preferred",
      found_in: ["skills"],
      frequency: 1,
    },
  ],
  missing_keywords: ["Spanish", "Zendesk"],
  missing_keyword_details: [
    {
      keyword: "Spanish",
      importance: "Required",
    },
    {
      keyword: "Zendesk",
      importance: "Preferred",
    },
  ],
  format_issues: [
    {
      severity: "Warning",
      issue: "Summary could be longer",
      fix: "Add more details about your experience",
    },
  ],
  suggestions: [
    {
      category: "AddKeyword",
      suggestion: "Review whether 'Spanish' is true for your background and worth making visible",
      impact: "Required job-post language is easier to compare when real evidence is visible.",
    },
  ],
};
