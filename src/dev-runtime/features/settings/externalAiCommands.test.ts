import { describe, expect, it } from "vitest";
import { mockJobs } from "../../mocks/data";
import type { MockExternalAiState } from "../../mocks/runtimeState";
import { handleMockExternalAiCommand } from "./externalAiCommands";

describe("External AI mock commands", () => {
  const newState = (): MockExternalAiState & { jobs: typeof mockJobs } => ({
    pendingExternalAiApproval: null,
    jobs: mockJobs,
  });
  const validRequest = () => ({
    feature: "job-description-summary",
    sourceJobId: mockJobs[0].id,
    provider: "anthropic",
    labels: ["External AI optional", "Public-data only"],
    dataCategories: ["job_posting"],
    payload: {
      title: mockJobs[0].title,
      company: mockJobs[0].company,
    },
    previewShown: true,
    userApproved: true,
  });

  it("prepares a reviewed request with a synthetic approval", () => {
    const state = newState();
    const prepared = handleMockExternalAiCommand(
      "prepare_external_ai_request",
      { request: validRequest() },
      state,
    );

    expect(prepared.handled).toBe(true);
    expect(prepared.value).toMatchObject({
      approvalId: expect.stringMatching(/^outside-ai-approval:/),
      providerId: "anthropic",
      destination: expect.stringMatching(/^https:\/\//),
      model: expect.any(String),
      fieldCount: 2,
    });
  });

  it("rejects malformed, stale, unsupported, or rewritten public job requests", () => {
    const state = newState();

    for (const request of [
      { provider: "anthropic", payload: {} },
      { ...validRequest(), sourceJobId: 999 },
      { ...validRequest(), provider: "unsupported" },
      {
        ...validRequest(),
        payload: { title: "Rewritten role", company: mockJobs[0].company },
      },
    ]) {
      expect(() =>
        handleMockExternalAiCommand(
          "prepare_external_ai_request",
          { request },
          state,
        ),
      ).toThrow();
    }
    expect(state.pendingExternalAiApproval).toBeNull();
  });

  it("sends only the current exact prepared request once", () => {
    const state = newState();
    const request = validRequest();
    const exactRequest = validRequest();
    const approvalId = (
      handleMockExternalAiCommand(
        "prepare_external_ai_request",
        { request },
        state,
      ).value as { approvalId: string }
    ).approvalId;

    expect(() =>
      handleMockExternalAiCommand(
        "send_external_ai_request",
        {
          approvalId,
          request: { ...request, payload: { title: "Different role" } },
        },
        state,
      ),
    ).toThrow("no current matching review");

    request.payload.title = "Changed after review";
    expect(() =>
      handleMockExternalAiCommand(
        "send_external_ai_request",
        { approvalId, request },
        state,
      ),
    ).toThrow("no current matching review");

    expect(
      handleMockExternalAiCommand(
        "send_external_ai_request",
        { approvalId, request: exactRequest },
        state,
      ).value,
    ).toMatchObject({
      text: expect.any(String),
      provider: "anthropic",
      model: expect.any(String),
    });

    expect(() =>
      handleMockExternalAiCommand(
        "send_external_ai_request",
        { approvalId, request },
        state,
      ),
    ).toThrow("no current matching review");
  });

  it("cancels only the current exact approval", () => {
    const state = newState();
    const staleApprovalId = (
      handleMockExternalAiCommand(
        "prepare_external_ai_request",
        { request: validRequest() },
        state,
      ).value as { approvalId: string }
    ).approvalId;
    const approvalId = (
      handleMockExternalAiCommand(
        "prepare_external_ai_request",
        { request: validRequest() },
        state,
      ).value as { approvalId: string }
    ).approvalId;

    expect(() =>
      handleMockExternalAiCommand(
        "cancel_external_ai_request",
        { approvalId: staleApprovalId },
        state,
      ),
    ).toThrow("could not be verified");

    expect(() =>
      handleMockExternalAiCommand(
        "cancel_external_ai_request",
        { approvalId: "outside-ai-approval:unknown" },
        state,
      ),
    ).toThrow("could not be verified");

    expect(
      handleMockExternalAiCommand(
        "cancel_external_ai_request",
        { approvalId },
        state,
      ).value,
    ).toEqual({ outcome: "cancelled" });

    expect(() =>
      handleMockExternalAiCommand(
        "cancel_external_ai_request",
        { approvalId },
        state,
      ),
    ).toThrow("could not be verified");
  });

  it("lists durable activity entries the settings history accepts", () => {
    const activity = handleMockExternalAiCommand(
      "list_external_ai_activity",
      undefined,
      newState(),
    ).value as Array<Record<string, unknown>>;

    expect(activity.length).toBeGreaterThan(0);
    for (const entry of activity) {
      expect(entry.providerId).toEqual(expect.any(String));
      expect(entry.destination).toMatch(/^https:\/\//);
      expect(Number.isNaN(Date.parse(entry.createdAt as string))).toBe(false);
      expect(
        entry.completedAt === null ||
          !Number.isNaN(Date.parse(entry.completedAt as string)),
      ).toBe(true);
    }
  });

  it("rejects commands owned by another feature", () => {
    expect(
      handleMockExternalAiCommand("get_config", undefined, newState()),
    ).toMatchObject({ handled: false });
  });
});
