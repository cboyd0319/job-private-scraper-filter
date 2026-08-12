import { describe, expect, it } from "vitest";
import {
  type ResumeData,
} from "./resumeBuilderData";
import {
  toJsonResumeData,
  toResumeAnalysisInput,
  toStructuredResume,
} from "./resumeBuilderTransforms";

function createResumeWithEvidenceSections(): ResumeData {
  return {
    id: 7,
    contact: {
      name: "Applicant Example",
      email: "applicant@example.test",
      phone: null,
      linkedin: null,
      github: null,
      location: null,
      website: null,
    },
    summary: "Operations coordinator focused on accessible services.",
    experience: [],
    education: [],
    skills: [],
    certifications: [
      {
        name: "Certified Community Health Worker",
        issuer: "State Health Board",
        date_obtained: "2024",
        expiration_date: null,
        credential_id: "CHW-123",
      },
    ],
    projects: [
      {
        name: "Clinic Intake Redesign",
        description: "Improved appointment intake for community clinic.",
        technologies: ["Scheduling", "Patient intake"],
        url: "https://example.test/project",
        start_date: null,
        end_date: null,
      },
    ],
    clearance: "User-confirmed current Secret clearance",
    military_info: "Army 25B with user-confirmed network operations duties",
    created_at: "2026-06-18T00:00:00Z",
    updated_at: "2026-06-18T00:00:00Z",
  };
}

describe("resume builder data conversion", () => {
  it("preserves credential, project, and military evidence in the shared payload", () => {
    const resume = createResumeWithEvidenceSections();
    const structured = toStructuredResume(resume);

    expect(structured.certifications).toEqual([
      {
        name: "Certified Community Health Worker",
        issuer: "State Health Board",
        date_obtained: "2024",
        expiration_date: null,
        credential_id: "CHW-123",
      },
    ]);
    expect(structured.projects).toEqual([
      {
        name: "Clinic Intake Redesign",
        description: "Improved appointment intake for community clinic.",
        technologies: ["Scheduling", "Patient intake"],
        url: "https://example.test/project",
        start_date: null,
        end_date: null,
      },
    ]);
    expect(structured.clearance).toBe("User-confirmed current Secret clearance");
    expect(structured.military_info).toBe(
      "Army 25B with user-confirmed network operations duties",
    );

    expect(toResumeAnalysisInput(resume)).toEqual({
      resume: structured,
      custom_sections: {},
      evidence_snapshot: {
        source_id: "resume-draft:7",
        revision: "2026-06-18T00:00:00Z",
      },
    });
  });

  it("exports builder data in JSON Resume shape", () => {
    const resume = {
      ...createResumeWithEvidenceSections(),
      contact: {
        ...createResumeWithEvidenceSections().contact,
        phone: "555-0100",
        linkedin: "https://linkedin.com/in/applicant",
        github: "https://github.com/applicant",
        location: "Portland, OR",
        website: "https://example.test",
      },
      experience: [
        {
          id: 1,
          company: "Neighborhood Clinic",
          title: "Operations Coordinator",
          location: "Portland, OR",
          start_date: "2022-01",
          end_date: null,
          achievements: ["Improved intake scheduling by 18%"],
        },
      ],
      education: [
        {
          id: 2,
          institution: "State University",
          degree: "Bachelor of Arts",
          graduation_date: "2020",
          gpa: "3.8",
          honors: ["Dean's List"],
        },
      ],
      skills: [
        {
          name: "Scheduling",
          category: "Operations",
          proficiency: "advanced",
        },
      ],
    } satisfies ResumeData;

    expect(toJsonResumeData(resume)).toMatchObject({
      basics: {
        name: "Applicant Example",
        email: "applicant@example.test",
        phone: "555-0100",
        url: "https://example.test",
        summary: "Operations coordinator focused on accessible services.",
        location: { address: "Portland, OR" },
        profiles: [
          {
            network: "LinkedIn",
            url: "https://linkedin.com/in/applicant",
          },
          {
            network: "GitHub",
            url: "https://github.com/applicant",
          },
        ],
      },
      work: [
        {
          name: "Neighborhood Clinic",
          position: "Operations Coordinator",
          startDate: "2022-01",
          endDate: "",
          highlights: ["Improved intake scheduling by 18%"],
        },
      ],
      education: [
        {
          institution: "State University",
          studyType: "Bachelor of Arts",
          area: "",
          endDate: "2020",
          score: "3.8",
          courses: ["Dean's List"],
        },
      ],
      certificates: [
        {
          name: "Certified Community Health Worker",
          issuer: "State Health Board",
          date: "2024",
        },
      ],
      skills: [
        {
          name: "Operations",
          level: "advanced",
          keywords: ["Scheduling"],
        },
      ],
      projects: [
        {
          name: "Clinic Intake Redesign",
          description: "Improved appointment intake for community clinic.",
          keywords: ["Scheduling", "Patient intake"],
          url: "https://example.test/project",
        },
      ],
    });
  });
});
