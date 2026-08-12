import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke, resetMockData } from "../../mocks/handlers";
import type { AtsAnalysisResult } from "./resumeAnalysisTestData";

describe("mock resume matching profiles", () => {
  beforeEach(() => {
    resetMockData();
  });

  it("applies only an explicit local role and regional profile", async () => {
    const resume = {
      summary: "",
      experience: [
        {
          title: "Program Analyst",
          achievements: ["Led program evaluation."],
        },
      ],
      education: [],
      skills: [],
      certifications: [],
      projects: [],
      custom_sections: {},
    };
    const profiled = await mockInvoke<AtsAnalysisResult>(
      "analyze_resume_for_job",
      {
        resume,
        jobDescription: "Required: programme evaluation",
        matchingProfile: { profession: "operations", region: "uk" },
      },
    );
    const unprofiled = await mockInvoke<AtsAnalysisResult>(
      "analyze_resume_for_job",
      {
        resume,
        jobDescription: "Required: programme evaluation",
      },
    );
    await mockInvoke<number>("select_and_upload_resume");
    const active = await mockInvoke<AtsAnalysisResult>(
      "analyze_active_resume_for_job",
      {
        jobDescription: "Required: programme evaluation",
        matchingProfile: { profession: "operations", region: "uk" },
      },
    );

    expect(profiled.matching_profile).toEqual({
      profession: "operations",
      region: "uk",
    });
    expect(profiled.keyword_matches).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ keyword: "program evaluation" }),
      ]),
    );
    expect(profiled.requirement_reviews[0]).toMatchObject({
      profile_preferred_section: true,
    });
    expect(active.matching_profile).toEqual(profiled.matching_profile);
    expect(unprofiled.matching_profile).toBeUndefined();
  });

  it("rejects a malformed profile instead of inferring a fallback", async () => {
    await expect(
      mockInvoke("analyze_resume_for_job", {
        resume: {},
        jobDescription: "Required: program evaluation",
        matchingProfile: { profession: "operations", region: "global" },
      }),
    ).rejects.toThrow("Invalid resume matching profile");
  });
});
