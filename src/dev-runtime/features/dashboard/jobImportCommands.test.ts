import { beforeEach, describe, expect, it } from "vitest";
import { mockJobs } from "../../mocks/data";
import { mockInvoke, resetMockData } from "../../mocks/handlers";
import {
  handleMockJobImportCommand,
  type MockJobImportPreview,
  type MockJobImportResult,
} from "./jobImportCommands";

describe("Dashboard job-import mock commands", () => {
  beforeEach(() => {
    resetMockData();
  });

  it("previews and imports jobs with minimized backend payloads", async () => {
    const url =
      "https://alice:secret@jobs.example.com/careers/care-coordinator?jobId=123&utm_source=newsletter&token=raw-secret#private";
    const canonicalUrl =
      "https://jobs.example.com/careers/care-coordinator?jobId=123";

    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_job_import",
      {
        url,
      },
    );

    expect(preview).toMatchObject({
      title: "Care Coordinator",
      import_id: expect.any(String),
      company: "jobs.example.com",
      url: canonicalUrl,
      location: "Remote",
      description_preview: expect.stringContaining("Care Coordinator"),
      salary: "$55k-$72k",
      employment_types: ["FULL_TIME"],
      remote: true,
      missing_fields: [],
      already_exists: false,
    });
    expect(preview.date_posted).toEqual(expect.any(String));

    const imported = await mockInvoke<MockJobImportResult>(
      "confirm_job_import",
      {
        importId: preview.import_id,
      },
    );
    expect(imported).toEqual({ jobId: expect.any(Number) });

    const duplicatePreview = await mockInvoke<MockJobImportPreview>(
      "preview_job_import",
      { url },
    );
    expect(duplicatePreview.already_exists).toBe(true);
    await expect(
      mockInvoke<MockJobImportResult>("confirm_job_import", {
        importId: preview.import_id,
      }),
    ).rejects.toThrow("This job preview expired");
  });

  it("binds pending imports to their confirmation operation without consuming cross-confirmed drafts", async () => {
    const jobPreview = await mockInvoke<MockJobImportPreview>(
      "preview_job_import",
      { url: "https://jobs.example.com/careers/care-coordinator" },
    );
    await expect(
      mockInvoke("confirm_smart_paste", { importId: jobPreview.import_id }),
    ).rejects.toThrow("This Smart Paste draft expired");
    await expect(
      mockInvoke("confirm_job_import", { importId: jobPreview.import_id }),
    ).resolves.toEqual({ jobId: expect.any(Number) });

    const smartPreview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Title: Operations Coordinator",
          "Company: Example Services",
          "Job link: https://jobs.example.com/careers/operations-coordinator",
        ].join("\n"),
      },
    );
    await expect(
      mockInvoke("confirm_job_import", { importId: smartPreview.import_id }),
    ).rejects.toThrow("This job preview expired");
    await expect(
      mockInvoke("confirm_smart_paste", { importId: smartPreview.import_id }),
    ).resolves.toEqual({ jobId: expect.any(Number) });
  });

  it("stages and saves a Smart Paste draft after review", async () => {
    const text = [
      "Office Manager",
      "Example Services",
      "https://jobs.example.com/careers/office-manager?utm_source=newsletter",
      "Coordinate office operations.",
    ].join("\n");

    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      { text },
    );

    expect(preview).toMatchObject({
      import_id: expect.any(String),
      title: "Office Manager",
      company: "Example Services",
      url: "https://jobs.example.com/careers/office-manager",
      location: null,
      description_preview: "Coordinate office operations.",
      salary: null,
      date_posted: null,
      employment_types: [],
      remote: false,
      missing_fields: [],
      already_exists: false,
    });

    const saved = await mockInvoke<MockJobImportResult>("confirm_smart_paste", {
      importId: preview.import_id,
    });
    expect(saved).toEqual({ jobId: expect.any(Number) });
  });

  it("saves the full reviewed Smart Paste description with omitted native fields", async () => {
    const description = "Full review detail. ".repeat(40);
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Title: Operations Coordinator",
          "Company: Example Services",
          "Job link: https://jobs.example.com/careers/operations-coordinator",
          `Description: ${description}`,
        ].join("\n"),
      },
    );

    expect(preview.description_preview).toHaveLength(503);
    await mockInvoke<MockJobImportResult>("confirm_smart_paste", {
      importId: preview.import_id,
    });

    const [saved] = await mockInvoke<
      Array<{
        description: string | null;
        location: string | null;
        salary_min: number | null;
        salary_max: number | null;
        score: number | null;
      }>
    >("get_recent_jobs", { limit: 1 });
    expect(saved).toMatchObject({
      description: description.trim(),
      location: null,
      salary_min: null,
      salary_max: null,
      score: null,
    });
  });

  it("keeps incomplete Smart Paste text a reviewable unsaved draft", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      { text: "Office Manager\nExample Services" },
    );

    expect(preview.import_id).toBeNull();
    expect(preview.missing_fields).toEqual(["url"]);

    const reviewed = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: "Office Manager\nExample Services",
        title: "Office Manager",
        company: "Example Services",
        jobUrl: "https://jobs.example.com/careers/office-manager",
        location: "Denver, CO",
      },
    );
    expect(reviewed).toMatchObject({
      import_id: expect.any(String),
      location: "Denver, CO",
      missing_fields: [],
    });

    await expect(
      mockInvoke("confirm_smart_paste", { importId: "mock-missing" }),
    ).rejects.toThrow("This Smart Paste draft expired");
  });

  it("parses labeled Smart Paste fields with the production grammar", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Job title: Operations Coordinator",
          "Company: Example Services",
          "Location: Denver, CO",
          "Description: Coordinate office operations.",
          "Job link: https://jobs.example.com/careers/operations-coordinator?utm_source=newsletter",
        ].join("\n"),
      },
    );

    expect(preview).toMatchObject({
      title: "Operations Coordinator",
      company: "Example Services",
      location: "Denver, CO",
      description_preview: "Coordinate office operations.",
      url: "https://jobs.example.com/careers/operations-coordinator",
    });
  });

  it("rejects credential-like Smart Paste source text and review edits before staging", async () => {
    const text = [
      "Title: Operations Coordinator",
      "Company: Example Services",
      "Job link: https://jobs.example.com/careers/operations-coordinator",
    ].join("\n");

    for (const credentialMaterial of [
      "Cookie: li_at=AQED-opaque-value",
      '{"access_token":"oauth-secret-value"}',
      "Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==",
    ]) {
      await expect(
        mockInvoke("preview_smart_paste", {
          text: `${text}\n${credentialMaterial}`,
        }),
      ).rejects.toThrow(
        "Pasted job details contain session or credential material",
      );
    }

    for (const edits of [
      { title: "Authorization: Bearer eyJhbGciOi.fake.payload" },
      { company: "Cookie: sessionid=secret-session-value-1234567890" },
      {
        jobUrl:
          "https://jobs.example.com/careers/operations-coordinator?access_token=oauth-secret-value",
      },
      { location: "password=hunter2" },
    ]) {
      await expect(
        mockInvoke("preview_smart_paste", { text, ...edits }),
      ).rejects.toThrow(
        "Pasted job details contain session or credential material",
      );
    }
  });

  it("rejects quoted Basic and Negotiate authorization credentials", async () => {
    const text = [
      "Title: Operations Coordinator",
      "Company: Example Services",
      "Job link: https://jobs.example.com/careers/operations-coordinator",
    ].join("\n");

    for (const authorization of [
      'Authorization: Basic "QWxhZGRpbjpvcGVuIHNlc2FtZQ=="',
      'Authorization: Negotiate "QWxhZGRpbjpvcGVuIHNlc2FtZQ=="',
      'Authorization: "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="',
      "Authorization: 'Negotiate QWxhZGRpbjpvcGVuIHNlc2FtZQ=='",
    ]) {
      await expect(
        mockInvoke("preview_smart_paste", { text: `${text}\n${authorization}` }),
      ).rejects.toThrow(
        "Pasted job details contain session or credential material",
      );
    }
  });

  it("rejects cookie-object credentials before creating a Smart Paste preview", async () => {
    await expect(
      mockInvoke("preview_smart_paste", {
        text: [
          "Title: Operations Coordinator",
          "Company: Example Services",
          "Job link: https://jobs.example.com/careers/operations-coordinator",
          '{"name":"li_at","value":"AQED-opaque-value"}',
        ].join("\n"),
      }),
    ).rejects.toThrow(
      "Pasted job details contain session or credential material",
    );
  });

  it("rejects unsafe bare Smart Paste URLs before considering later candidates", async () => {
    await expect(
      mockInvoke("preview_smart_paste", {
        text: [
          "Office Manager",
          "Example Services",
          "https://localhost:3000/careers/office-manager",
          "https://jobs.example.com/careers/office-manager",
        ].join("\n"),
      }),
    ).rejects.toThrow("Paste the full job link from your browser address bar.");
  });

  it("recognizes Smart Paste URLs wrapped in brackets", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Office Manager",
          "Example Services",
          "[https://jobs.example.com/careers/office-manager]",
        ].join("\n"),
      },
    );

    expect(preview.url).toBe(
      "https://jobs.example.com/careers/office-manager",
    );
  });

  it("matches native job-link query stripping and public host validation", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Office Manager",
          "Example Services",
          "https://jobs.example.com/careers/office-manager?jobId=123&sid=private-id",
        ].join("\n"),
      },
    );
    expect(preview.url).toBe(
      "https://jobs.example.com/careers/office-manager?jobId=123",
    );

    for (const url of [
      "https://jobs.internal/careers/office-manager",
      "https://[fe80::1]/careers/office-manager",
      "https://[::ffff:7f00:1]/careers/office-manager",
      "https://[::ffff:c0a8:1]/careers/office-manager",
    ]) {
      await expect(
        mockInvoke("preview_smart_paste", {
          text: `Office Manager\nExample Services\n${url}`,
        }),
      ).rejects.toThrow("Paste the full job link from your browser address bar.");
    }
  });

  it("rejects embedded non-public IPv4 windows while allowing public controls", async () => {
    for (const url of [
      "https://127.0.0.1.nip.io/careers/office-manager",
      "https://jobs.10.0.0.1.example.com/careers/office-manager",
    ]) {
      await expect(
        mockInvoke("preview_smart_paste", {
          text: `Office Manager\nExample Services\n${url}`,
        }),
      ).rejects.toThrow("Paste the full job link from your browser address bar.");
    }

    await expect(
      mockInvoke<MockJobImportPreview>("preview_smart_paste", {
        text: "Office Manager\nExample Services\nhttps://8.8.8.8.nip.io/careers/office-manager",
      }),
    ).resolves.toMatchObject({
      url: "https://8.8.8.8.nip.io/careers/office-manager",
    });
  });

  it("removes all LinkedIn query parameters after Smart Paste privacy stripping", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Office Manager",
          "Example Services",
          "https://www.linkedin.com/jobs/view/123456?trk=public_jobs&currentJobId=123456",
        ].join("\n"),
      },
    );

    expect(preview.url).toBe("https://www.linkedin.com/jobs/view/123456");
  });

  it("matches Smart Paste field byte limits at the boundary", async () => {
    const text = [
      "Title: Operations Coordinator",
      "Company: Example Services",
      "Location: Denver, CO",
      "Job link: https://jobs.example.com/careers/operations-coordinator",
    ].join("\n");

    for (const { field, limit, errorField } of [
      { field: "title", limit: 500, errorField: "job title" },
      { field: "company", limit: 200, errorField: "company name" },
      { field: "location", limit: 200, errorField: "location" },
    ] as const) {
      const withinLimit = "é".repeat(limit / 2);
      await expect(
        mockInvoke("preview_smart_paste", { text, [field]: withinLimit }),
      ).resolves.toMatchObject({ [field]: withinLimit });

      await expect(
        mockInvoke("preview_smart_paste", {
          text,
          [field]: `${withinLimit}é`,
        }),
      ).rejects.toThrow(`Pasted ${errorField} exceeds the local draft limit`);
    }

    await expect(
      mockInvoke("preview_smart_paste", {
        text,
        title: ` ${"é".repeat(250)}`,
      }),
    ).rejects.toThrow("Pasted job title exceeds the local draft limit");
  });

  it("keeps unlabeled Smart Paste notes as description when later labels fill required fields", async () => {
    const preview = await mockInvoke<MockJobImportPreview>(
      "preview_smart_paste",
      {
        text: [
          "Hiring team note one.",
          "Hiring team note two.",
          "Title: Operations Coordinator",
          "Company: Example Services",
          "Job link: https://jobs.example.com/careers/operations-coordinator",
        ].join("\n"),
      },
    );

    expect(preview).toMatchObject({
      title: "Operations Coordinator",
      company: "Example Services",
      description_preview: "Hiring team note one.\nHiring team note two.",
    });
  });

  it("rejects unsafe links and removes sensitive URL details", async () => {
    await expect(
      mockInvoke<MockJobImportPreview>("preview_job_import", {
        url: "http://jobs.example.com/careers/care-coordinator",
      }),
    ).rejects.toThrow(
      "Paste an https job posting link from your browser address bar.",
    );
    await expect(
      mockInvoke<MockJobImportPreview>("preview_job_import", {
        url: "http://localhost:3000/jobs/care-coordinator",
      }),
    ).rejects.toThrow("Paste the full job link from your browser address bar.");

    const redirectPreview = await mockInvoke<MockJobImportPreview>(
      "preview_job_import",
      {
        url: "https://jobs.example.com/careers/case-manager?jobId=456&redirect=https%3A%2F%2Fprivate.example%2Fcallback%3Ftoken%3Draw-secret&source=mail",
      },
    );
    expect(redirectPreview.url).toBe(
      "https://jobs.example.com/careers/case-manager?jobId=456",
    );
    expect(redirectPreview.url).not.toContain("raw-secret");
  });

  it("rejects commands owned by another feature", () => {
    const state = {
      jobs: mockJobs.map((job) => ({ ...job })),
      pendingUrlImports: [],
    };

    expect(
      handleMockJobImportCommand("generate_deep_links", undefined, state),
    ).toMatchObject({ handled: false, shouldSave: false, state });
  });
});
