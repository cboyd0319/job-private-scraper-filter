/** Verifies the browser-development command facade and deterministic state. */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { registeredMockCommands } from "./commandRegistry";
import { mockJobs } from "./data";
import { mockInvoke, resetMockData } from "./handlers";
import { loadMockState, mockRuntimeState } from "./runtimeState";

type MockJobSummary = { hash: string };
type SalaryBenchmark = {
  job_title: string;
  location: string;
  seniority_level: string;
  p25_salary: number;
  median_salary: number;
  p75_salary: number;
  max_salary: number;
  sample_size: number;
};
type AtsDetectionResponse = {
  platform: string;
  commonFields: string[];
  automationNotes: string | null;
};
type FillResultWithAttempt = {
  filledFields: string[];
  unfilledFields: string[];
  captchaDetected: boolean;
  readyForReview: boolean;
  errorMessage: string | null;
  attemptId: number | null;
  durationMs: number;
  atsPlatform: string;
};
type AnswerSuggestion = {
  answer: string;
  source: { type: "manual"; answerId: number };
  timesUsed: number;
  lastUsedDaysAgo: number | null;
};

const MOCK_INVOKE_CONTROLS_KEY = "jobsentinel.mockInvokeControls.v1";
const MOCK_STATE_KEY = "jobsentinel.mockState.v1";

function outsideAiRequest() {
  return {
    feature: "job-description-summary",
    sourceJobId: mockJobs[0].id,
    provider: "anthropic",
    labels: ["External AI optional", "Public-data only"],
    dataCategories: ["job_posting"],
    payload: { title: mockJobs[0].title, company: mockJobs[0].company },
    previewShown: true,
    userApproved: true,
  };
}

describe("mock Tauri command facade", () => {
  let localStore: Record<string, string>;

  beforeEach(() => {
    vi.useRealTimers();
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

  it("registers each development command once", () => {
    expect(new Set(registeredMockCommands).size).toBe(registeredMockCommands.length);
    expect(registeredMockCommands).toEqual(
      expect.arrayContaining([
        "get_jobs",
        "get_config",
        "get_active_resume",
        "fill_application_form",
        "list_pack_management",
        "disable_pack",
        "uninstall_pack",
        "retry_pack_cleanup",
        "record_linkedin_workbench_event",
      ]),
    );
  });

  it("mirrors pack lifecycle response generations", async () => {
    const identity = {
      publisherKeyId: "jobsentinel-release-v1",
      packId: "jobsentinel.sources.us",
      expectedGeneration: 4,
    };

    await expect(mockInvoke("list_pack_management")).resolves.toEqual([
      expect.objectContaining({
        publisherKeyId: identity.publisherKeyId,
        packId: identity.packId,
        state: "ready",
        generation: 4,
      }),
    ]);
    await expect(mockInvoke("disable_pack", identity)).resolves.toEqual({
      state: "disabled",
      generation: 5,
    });
    await expect(mockInvoke("list_pack_management")).resolves.toEqual([
      expect.objectContaining({ state: "disabled", generation: 5 }),
    ]);
    await expect(mockInvoke("disable_pack", identity)).rejects.toThrow(
      "pack changed",
    );
    await expect(mockInvoke("retry_pack_cleanup", {
      ...identity,
      expectedGeneration: 5,
    })).resolves.toEqual({
      generation: 5,
      cleanupPending: false,
    });
    await expect(mockInvoke("uninstall_pack", {
      ...identity,
      expectedGeneration: 5,
    })).resolves.toEqual({
      generation: 6,
      cleanupPending: false,
    });
    await expect(mockInvoke("list_pack_management")).resolves.toEqual([
      expect.objectContaining({ state: "removed", generation: 6 }),
    ]);

    resetMockData();
    await expect(mockInvoke("list_pack_management")).resolves.toEqual([
      expect.objectContaining({ state: "ready", generation: 4 }),
    ]);
  });

  it("supports forced command failures", async () => {
    window.localStorage.setItem(
      MOCK_INVOKE_CONTROLS_KEY,
      JSON.stringify({ delayMs: 0, failures: { get_jobs: "Forced test failure" } }),
    );
    await expect(mockInvoke<unknown>("get_jobs")).rejects.toThrow("Forced test failure");
  });

  it("supports command-specific delays", async () => {
    vi.useFakeTimers();
    window.localStorage.setItem(
      MOCK_INVOKE_CONTROLS_KEY,
      JSON.stringify({ delayMs: 0, delays: { get_jobs: 500 } }),
    );

    let settled = false;
    const result = mockInvoke<MockJobSummary[]>("get_jobs").finally(() => {
      settled = true;
    });
    await vi.advanceTimersByTimeAsync(499);
    expect(settled).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    await expect(result).resolves.toEqual(
      expect.arrayContaining([expect.objectContaining({ hash: expect.any(String) })]),
    );
    expect(settled).toBe(true);
  });

  it("supports response overrides", async () => {
    window.localStorage.setItem(
      MOCK_INVOKE_CONTROLS_KEY,
      JSON.stringify({
        delayMs: 0,
        responses: { get_jobs: [], is_first_run: true },
      }),
    );
    await expect(mockInvoke<MockJobSummary[]>("get_jobs")).resolves.toEqual([]);
    await expect(mockInvoke<boolean>("is_first_run")).resolves.toBe(true);
  });

  it("clears invoke controls when mock data resets", () => {
    window.localStorage.setItem(
      MOCK_INVOKE_CONTROLS_KEY,
      JSON.stringify({ failures: { get_jobs: "Forced test failure" } }),
    );
    resetMockData();
    expect(window.localStorage.getItem(MOCK_INVOKE_CONTROLS_KEY)).toBeNull();
  });

  it("routes reviewed Outside AI and staged recovery commands through the registry", async () => {
    const request = outsideAiRequest();
    const prepared = await mockInvoke<{ approvalId: string }>(
      "prepare_external_ai_request",
      { request },
    );

    await expect(
      mockInvoke("send_external_ai_request", {
        approvalId: prepared.approvalId,
        request,
      }),
    ).resolves.toMatchObject({
      provider: "anthropic",
      text: expect.any(String),
    });

    await expect(
      mockInvoke("stage_portable_restore", { passphrase: "sixteen-letters!" }),
    ).resolves.toMatchObject({ outcome: "staged" });
    await expect(mockInvoke("get_staged_restore_status")).resolves.toBe("ready");
    await expect(mockInvoke("cancel_staged_restore")).resolves.toMatchObject({
      outcome: "cancelled",
    });
    await expect(mockInvoke("get_staged_restore_status")).resolves.toBe("none");
  });

  it("reloads only the staged restore status marker and rejects invalid persisted values", async () => {
    await mockInvoke("stage_portable_restore", {
      passphrase: "sixteen-letters!",
    });
    const persisted = window.localStorage.getItem(MOCK_STATE_KEY) ?? "";
    expect(persisted).toContain('"stagedRestoreStatus":"ready"');
    expect(persisted).not.toContain("sixteen-letters!");
    expect(JSON.parse(persisted)).not.toHaveProperty("portableRestorePath");

    mockRuntimeState.stagedRestoreStatus = "none";
    loadMockState();
    await expect(mockInvoke("get_staged_restore_status")).resolves.toBe("ready");

    window.localStorage.setItem(
      MOCK_STATE_KEY,
      JSON.stringify({ stagedRestoreStatus: "promoted" }),
    );
    mockRuntimeState.stagedRestoreStatus = "none";
    loadMockState();
    await expect(mockInvoke("get_staged_restore_status")).resolves.toBe("none");
  });

  it("clears transient Outside AI approval and staged recovery state on reset", async () => {
    const prepared = await mockInvoke<{ approvalId: string }>(
      "prepare_external_ai_request",
      { request: outsideAiRequest() },
    );
    await mockInvoke("stage_portable_restore", {
      passphrase: "sixteen-letters!",
    });

    resetMockData();

    await expect(mockInvoke("get_staged_restore_status")).resolves.toBe("none");
    expect(
      JSON.parse(window.localStorage.getItem(MOCK_STATE_KEY) ?? "{}"),
    ).toMatchObject({ stagedRestoreStatus: "none" });
    await expect(
      mockInvoke("cancel_external_ai_request", { approvalId: prepared.approvalId }),
    ).rejects.toThrow("could not be verified");
  });

  it("routes representative runtime commands through feature owners", async () => {
    const benchmark = await mockInvoke<SalaryBenchmark>("get_salary_benchmark", {
      jobTitle: "Training Coordinator",
      location: "Chicago, IL",
      seniority: "mid",
    });
    expect(benchmark).toMatchObject({
      job_title: "Training Coordinator",
      location: "Chicago, IL",
      seniority_level: "Mid",
      p25_salary: expect.any(Number),
      median_salary: expect.any(Number),
      p75_salary: expect.any(Number),
      max_salary: expect.any(Number),
      sample_size: expect.any(Number),
    });

    const script = await mockInvoke<string>("generate_negotiation_script", {
      scenario: "initial_offer",
      params: {
        company: "CareBridge Health",
        current_offer: "60000",
        job_title: "Training Coordinator",
        location: "Chicago, IL",
        target_min: "68000",
        target_max: "74000",
        years_experience: "5",
      },
    });
    expect(script).toContain("Training Coordinator");
    expect(script).toContain("$68,000");
    expect(script).toContain("$74,000");

    const ats = await mockInvoke<AtsDetectionResponse>("detect_ats_platform", {
      url: "https://boards.greenhouse.io/example/jobs/123",
    });
    expect(ats).toMatchObject({
      platform: "greenhouse",
      commonFields: expect.arrayContaining(["firstName", "lastName", "email"]),
      automationNotes: expect.any(String),
    });

    const fillResult = await mockInvoke<FillResultWithAttempt>("fill_application_form", {
      jobUrl: "https://boards.greenhouse.io/example/jobs/123",
      jobHash: "job-hash-1",
    });
    expect(fillResult).toMatchObject({
      filledFields: expect.arrayContaining(["firstName", "lastName", "email"]),
      unfilledFields: expect.any(Array),
      captchaDetected: false,
      readyForReview: true,
      errorMessage: null,
      attemptId: expect.any(Number),
      durationMs: expect.any(Number),
      atsPlatform: "greenhouse",
    });

    await expect(mockInvoke<boolean>("is_browser_running")).resolves.toBe(true);
    await expect(mockInvoke<void>("mark_attempt_submitted", {
      attemptId: fillResult.attemptId,
    })).resolves.toBeUndefined();
    await expect(mockInvoke<void>("close_automation_browser")).resolves.toBeUndefined();
    await expect(mockInvoke<boolean>("is_browser_running")).resolves.toBe(false);

    const suggestions = await mockInvoke<AnswerSuggestion[]>("get_suggested_answers", {
      question: "Are you authorized to work in the United States?",
      limit: 3,
    });
    expect(suggestions[0]).toMatchObject({
      answer: "Yes",
      source: { type: "manual", answerId: 1 },
      timesUsed: expect.any(Number),
      lastUsedDaysAgo: expect.any(Number),
    });

    await expect(mockInvoke<void>("complete_setup", { config: {} })).resolves.toBeUndefined();
    await expect(mockInvoke<void>("mark_job_as_real", { jobId: 1 })).resolves.toBeUndefined();
    await expect(mockInvoke<void>("mark_job_as_ghost", { jobId: 1 })).resolves.toBeUndefined();
  });
});
