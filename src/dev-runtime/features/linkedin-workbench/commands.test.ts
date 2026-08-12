import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke, resetMockData } from "../../mocks/handlers";

type LinkedInWorkbenchEventResult = {
  jobId: number;
  jobHash: string;
  applicationId: number | null;
  status: string;
  needsDetails: boolean;
  savedAsBookmark: boolean;
  hidden: boolean;
};

describe("LinkedIn workbench mock runtime commands", () => {
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

  it("requires a backend-style review before recording", async () => {
    await expect(
      mockInvoke("record_linkedin_workbench_event", {
        input: { eventType: "saved" },
      }),
    ).rejects.toThrow("Review LinkedIn Workbench");
    expect(
      await mockInvoke<string>("get_linkedin_workbench_review_status"),
    ).toBe("review_required");
    expect(await mockInvoke<boolean>("review_linkedin_workbench")).toBe(true);
    expect(
      await mockInvoke<string>("get_linkedin_workbench_review_status"),
    ).toBe("reviewed");
    expect(await mockInvoke<boolean>("revoke_linkedin_workbench_review")).toBe(
      true,
    );
    expect(
      await mockInvoke<string>("get_linkedin_workbench_review_status"),
    ).toBe("review_required");
  });

  it("records an applied action without retaining sensitive URL details", async () => {
    await mockInvoke("review_linkedin_workbench");
    const result = await mockInvoke<LinkedInWorkbenchEventResult>(
      "record_linkedin_workbench_event",
      {
        input: {
          eventType: "applied",
          title: "Principal Security Engineer",
          company: "Example Co",
          url: "https://www.linkedin.com/jobs/view/123?token=secret",
          notes:
            "User clicked Log applied from https://www.linkedin.com/jobs/view/123?token=secret li_at=raw-cookie.",
        },
      },
    );

    expect(result).toMatchObject({
      status: "applied",
      needsDetails: false,
      savedAsBookmark: true,
      hidden: false,
      applicationId: expect.any(Number),
    });
    expect(result.jobHash).not.toContain("token=secret");

    const linkedInJobs = await mockInvoke<Array<{
      title: string;
      company: string;
      url: string;
      bookmarked: boolean;
      notes: string | null;
    }>>("get_jobs", { source: "linkedin" });
    expect(linkedInJobs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          title: "Principal Security Engineer",
          company: "Example Co",
          url: "https://www.linkedin.com/jobs/view/123",
          bookmarked: true,
          notes:
            "User clicked Log applied from https://www.linkedin.com/jobs/view/123 li_at=[REDACTED]",
        }),
      ]),
    );
  });

  it("records expanded ledger actions", async () => {
    await mockInvoke("review_linkedin_workbench");
    for (const [eventType, expectedStatus] of [
      ["interview", "interview"],
      ["follow_up", "follow_up"],
      ["reminder", "reminder"],
      ["rejected", "rejected"],
    ] as const) {
      const result = await mockInvoke<LinkedInWorkbenchEventResult>(
        "record_linkedin_workbench_event",
        {
          input: {
            eventType,
            title: "Content Strategist",
            company: "Example Co",
            url: "https://www.linkedin.com/jobs/view/456",
          },
        },
      );
      expect(result).toMatchObject({
        status: expectedStatus,
        needsDetails: false,
        applicationId: expect.any(Number),
      });
    }
  });
});
