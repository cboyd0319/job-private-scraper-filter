import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke, resetMockData } from "../../mocks/handlers";

type AnswerSuggestion = {
  answer: string;
  source: { type: "manual"; answerId: number };
};

type ScreeningAnswerPreview = {
  questionPattern: string;
  answer: string;
};

type FillResult = {
  filledFields: string[];
  manualReviewTopics?: string[];
};

describe("application assist mock runtime commands", () => {
  let localStore: Record<string, string>;

  beforeEach(() => {
    localStore = {};
    vi.mocked(window.localStorage.getItem).mockImplementation(
      (key) => localStore[key] ?? null,
    );
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      localStore[key] = value;
    });
    vi.mocked(window.localStorage.removeItem).mockImplementation((key) => {
      delete localStore[key];
    });
    resetMockData();
  });

  it("treats saved screening-answer symbols as literal text", async () => {
    await mockInvoke<void>("upsert_screening_answer", {
      questionPattern: "Security+",
      answer: "Yes",
      answerType: "yes_no",
      notes: null,
    });

    await expect(mockInvoke<AnswerSuggestion[]>("get_suggested_answers", {
      question: "Do you have a Security+ certification?",
      limit: 3,
    })).resolves.toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          answer: "Yes",
          source: expect.objectContaining({ type: "manual" }),
        }),
      ]),
    );

    await expect(mockInvoke<AnswerSuggestion[]>("get_suggested_answers", {
      question: "Do you have a security clearance?",
      limit: 3,
    })).resolves.not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          answer: "Yes",
          source: expect.objectContaining({ type: "manual" }),
        }),
      ]),
    );
  });

  it("keeps user-controlled personal answers out of application previews and suggestions", async () => {
    await mockInvoke<void>("upsert_screening_answer", {
      questionPattern: "protected veteran status",
      answer: "Decline to answer",
      answerType: "select",
      notes: "private note",
    });
    await mockInvoke<void>("upsert_screening_answer", {
      questionPattern: "work authorization",
      answer: "Yes",
      answerType: "yes_no",
      notes: null,
    });

    const previews = await mockInvoke<ScreeningAnswerPreview[]>(
      "get_application_screening_answer_previews",
    );
    expect(previews).toEqual(
      expect.arrayContaining([{ questionPattern: "work authorization", answer: "Yes" }]),
    );
    expect(previews).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({ questionPattern: "protected veteran status" }),
      ]),
    );
    await expect(
      mockInvoke<AnswerSuggestion[]>("get_suggested_answers", {
        question: "What is your protected veteran status?",
        limit: 3,
      }),
    ).resolves.toEqual([]);
    const fill = await mockInvoke<FillResult>("fill_application_form", {
      jobUrl: "https://boards.greenhouse.io/example/jobs/123",
      jobHash: "job-123",
    });
    expect(fill.manualReviewTopics).toEqual([]);
  });

  it("models manual review from detected questions rather than saved answers", async () => {
    const fill = await mockInvoke<FillResult>("fill_application_form", {
      jobUrl: "https://boards.greenhouse.io/example/jobs/123",
      jobHash: "job-123",
      detectedQuestions: ["What are your pronouns?"],
    });

    expect(fill.manualReviewTopics).toEqual([
      "voluntary or sensitive personal questions",
    ]);
  });
});
