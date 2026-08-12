import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import {
  mockInvoke,
  mockJob,
  mockProfile,
  setupApplicationPreviewMocks,
} from "./ApplicationPreview.testSupport";
import { ApplicationPreview } from "./ApplicationPreview";

interface SavedAnswerFixture {
  questionPattern: string;
  answer: string;
  answerType?: string;
}

function savedAnswer({ questionPattern, answer, answerType = "text" }: SavedAnswerFixture) {
  return {
    id: 1,
    questionPattern,
    answer,
    answerType,
    notes: null,
    timesUsed: 0,
    timesModified: 0,
    confidenceScore: 0,
    lastUsedAt: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

function mockProfileAndAnswers(answers: SavedAnswerFixture[]) {
  mockInvoke.mockImplementation((command: string) => {
    if (command === "get_application_profile_preview") {
      return Promise.resolve(mockProfile);
    }

    if (command === "get_application_screening_answer_previews") {
      return Promise.resolve(answers.map(savedAnswer));
    }

    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

function renderPreview(description: string) {
  render(
    <ApplicationPreview
      job={{
        ...mockJob,
        description,
      }}
      atsPlatform="greenhouse"
    />,
  );
}

async function expectSavedGuidance(
  description: string,
  answer: SavedAnswerFixture,
  expectedGuidance: string,
) {
  mockProfileAndAnswers([answer]);
  renderPreview(description);

  expect(await screen.findByText("Hard Question Review")).toBeInTheDocument();
  expect(screen.getByText(expectedGuidance)).toBeInTheDocument();
  expect(mockInvoke).toHaveBeenCalledWith("get_application_screening_answer_previews");
}

describe("ApplicationPreview", () => {
  beforeEach(setupApplicationPreviewMocks);

  describe("saved hard-screening answer review", () => {
    it("shows saved work-authorization answers when the job asks about sponsorship", async () => {
      mockInvoke.mockImplementation((command: string) => {
        if (command === "get_application_profile_preview") {
          return Promise.resolve({
            ...mockProfile,
            requiresSponsorship: true,
          });
        }

        if (command === "get_application_screening_answer_previews") {
          return Promise.resolve([]);
        }

        return Promise.reject(new Error(`Unexpected command: ${command}`));
      });

      renderPreview("Applicants must be authorized to work without visa sponsorship.");

      expect(await screen.findByText("Hard Question Review")).toBeInTheDocument();
      expect(
        screen.getByText(
          "Saved profile says sponsorship is needed. Check the employer's sponsorship question and resume evidence before continuing.",
        ),
      ).toBeInTheDocument();
    });

    it("shows saved travel answers when the job asks about travel", async () => {
      await expectSavedGuidance(
        "This role requires travel to client sites up to 25%.",
        {
          questionPattern: "travel",
          answer: "I can travel up to 25% for client visits.",
        },
        "Saved travel answer says: I can travel up to 25% for client visits. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved credential answers when the job asks about certifications", async () => {
      await expectSavedGuidance(
        "PMP certification required for this role.",
        {
          questionPattern: "certification",
          answer: "I hold an active PMP certification.",
        },
        "Saved credential answer says: I hold an active PMP certification. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved education answers when the job asks about a degree", async () => {
      await expectSavedGuidance(
        "Bachelor's degree required, or equivalent education.",
        {
          questionPattern: "education",
          answer: "I have a bachelor's degree in business administration.",
        },
        "Saved education answer says: I have a bachelor's degree in business administration. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved education answers when the job asks about education", async () => {
      await expectSavedGuidance(
        "Education required: approved training program accepted.",
        {
          questionPattern: "education",
          answer: "I completed an approved training program.",
        },
        "Saved education answer says: I completed an approved training program. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved experience answers when the job asks about years of experience", async () => {
      await expectSavedGuidance(
        "5+ years of experience required.",
        {
          questionPattern: "years of experience",
          answer: "I have 6 years of customer support experience.",
        },
        "Saved experience answer says: I have 6 years of customer support experience. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved salary answers when the job asks about compensation", async () => {
      await expectSavedGuidance(
        "Please include salary or compensation expectations.",
        {
          questionPattern: "salary",
          answer: "My target salary range is $85,000 to $95,000.",
        },
        "Saved salary answer says: My target salary range is $85,000 to $95,000. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved salary-history answers with target-pay guidance", async () => {
      await expectSavedGuidance(
        "Please provide your current compensation and salary history.",
        {
          questionPattern: "salary history",
          answer: "My target range is $85,000 to $95,000 based on this role.",
        },
        "Saved salary-history answer says: My target range is $85,000 to $95,000 based on this role. Confirm it answers the exact question with role range or target pay, not unsupported past pay.",
      );
    });

    it("shows saved availability answers when the job asks about weekend availability", async () => {
      await expectSavedGuidance(
        "Weekend availability is required for this role.",
        {
          questionPattern: "availability",
          answer: "I am available for weekend shifts with two weeks of notice.",
        },
        "Saved availability answer says: I am available for weekend shifts with two weeks of notice. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved schedule answers when the job asks about schedule", async () => {
      await expectSavedGuidance(
        "Schedule requirement: rotating weekends with notice.",
        {
          questionPattern: "schedule",
          answer: "I can work a rotating schedule with advance notice.",
        },
        "Saved availability answer says: I can work a rotating schedule with advance notice. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved screening answers when the job asks about a background check", async () => {
      await expectSavedGuidance(
        "Background check is required after offer.",
        {
          questionPattern: "background check",
          answer: "I can complete the required background check.",
        },
        "Saved screening answer says: I can complete the required background check. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved language answers when the job asks about bilingual fluency", async () => {
      await expectSavedGuidance(
        "This role requires bilingual Spanish fluency.",
        {
          questionPattern: "bilingual Spanish",
          answer: "I am fluent in Spanish for customer calls.",
        },
        "Saved language answer says: I am fluent in Spanish for customer calls. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved physical-demand answers when the job asks about lifting", async () => {
      await expectSavedGuidance(
        "Must be able to lift 50 lbs throughout the shift.",
        {
          questionPattern: "lift 50 pounds",
          answer: "I can lift 50 pounds safely.",
        },
        "Saved physical requirement answer says: I can lift 50 pounds safely. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved availability answers when the job asks about overtime availability", async () => {
      await expectSavedGuidance(
        "Overtime is required during peak weeks.",
        {
          questionPattern: "overtime",
          answer: "I am available for overtime during peak weeks.",
        },
        "Saved availability answer says: I am available for overtime during peak weeks. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved availability answers when the job asks about holiday availability", async () => {
      await expectSavedGuidance(
        "Holiday work is required during peak weeks.",
        {
          questionPattern: "holiday",
          answer: "I am available for holiday shifts during peak weeks.",
        },
        "Saved availability answer says: I am available for holiday shifts during peak weeks. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved management answers when the job asks about managed-team experience", async () => {
      await expectSavedGuidance(
        "Required: managed a team for client intake coverage.",
        {
          questionPattern: "managed a team",
          answer: "I managed staff schedules for client intake coverage.",
        },
        "Saved management experience answer says: I managed staff schedules for client intake coverage. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved transportation answers when the job asks about reliable transportation", async () => {
      await expectSavedGuidance(
        "Required: reliable transportation for visits to client sites.",
        {
          questionPattern: "reliable transportation",
          answer: "Yes, I have reliable transportation for client-site visits.",
          answerType: "yes_no",
        },
        "Saved transportation answer says: Yes, I have reliable transportation for client-site visits. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved driving-record or insurance answers when the job asks about MVR", async () => {
      await expectSavedGuidance(
        "Required: MVR review and proof of auto insurance for field visits.",
        {
          questionPattern: "proof of auto insurance",
          answer: "I have current auto insurance for field visits.",
          answerType: "yes_no",
        },
        "Saved driving record or insurance answer says: I have current auto insurance for field visits. Confirm it matches the employer's wording and resume evidence before continuing.",
      );
    });

    it("shows saved citizenship answers when the job asks about citizenship", async () => {
      await expectSavedGuidance(
        "Applicants must be U.S. citizens for this contract.",
        {
          questionPattern: "US citizen",
          answer: "Yes, I am a U.S. citizen.",
          answerType: "yes_no",
        },
        "Saved citizenship answer says: Yes, I am a U.S. citizen. Confirm it matches the employer's wording before continuing.",
      );
    });

    it("shows saved age answers when the job asks about minimum age", async () => {
      await expectSavedGuidance(
        "Applicants must be 18 years of age before start.",
        {
          questionPattern: "18 years of age",
          answer: "Yes, I meet the 18 years of age requirement.",
        },
        "Saved age requirement answer says: Yes, I meet the 18 years of age requirement. Confirm it matches the employer's wording before continuing.",
      );
    });

    it("keeps hard question review when saved screening answers cannot load", async () => {
      mockInvoke.mockImplementation((command: string) => {
        if (command === "get_application_profile_preview") {
          return Promise.resolve(mockProfile);
        }

        if (command === "get_application_screening_answer_previews") {
          return Promise.reject(new Error("saved answers unavailable"));
        }

        return Promise.reject(new Error(`Unexpected command: ${command}`));
      });

      renderPreview("This role requires travel to client sites.");

      expect(await screen.findByText("Hard Question Review")).toBeInTheDocument();
      expect(
        screen.getByText(
          "Confirm location, commute, relocation, remote, hybrid, travel, and shift constraints.",
        ),
      ).toBeInTheDocument();
      expect(screen.queryByText(/saved answers unavailable/i)).not.toBeInTheDocument();
    });
  });
});
