import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  copySanitizedDebugReport,
  saveSanitizedDebugReport,
} from "./supportReport";
import {
  generateMockFeedbackReport,
  sanitizeMockFeedbackText,
} from "./mocks/supportReports";

const mockInvoke = vi.mocked(invoke);

describe("safe support reports", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it("copies a backend-sanitized report without private error details", async () => {
    mockInvoke
      .mockResolvedValueOnce("base report from backend")
      .mockResolvedValueOnce("final sanitized report");

    const result = await copySanitizedDebugReport([
      {
        id: "err-1",
        timestamp: "2026-05-28T10:15:00.000Z",
        message: "Failed at resume=private-file/secret.txt with token=abc123",
        stack: "Error: Failed at private path",
        type: "api",
        url: "http://localhost/?token=abc123",
        userAgent: "test-agent",
        context: {
          company: "Acme Health",
          file: "Alice Resume.pdf",
          url: "https://example.com/private-job",
        },
      },
    ]);

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "generate_feedback_report", {
      category: "bug",
      description: "User generated a safe support report from JobSentinel.",
      includeDebugInfo: true,
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "sanitize_feedback_text", {
      content: expect.stringContaining("RECENT APP PROBLEMS"),
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "sanitize_feedback_text", {
      content: expect.stringContaining("Review before sharing."),
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "sanitize_feedback_text", {
      content: expect.not.stringContaining("Support-only details:"),
    });
    const sanitizerInput = mockInvoke.mock.calls[1]?.[1];
    expect(sanitizerInput).toEqual({
      content: expect.stringContaining("Problem type: api"),
    });
    expect(sanitizerInput).toEqual({
      content: expect.not.stringContaining("Message:"),
    });
    expect(sanitizerInput).toEqual({
      content: expect.not.stringContaining("Extra safe details:"),
    });
    for (const privateDetail of [
      "private-file",
      "Acme Health",
      "Alice Resume.pdf",
      '"url"',
    ]) {
      expect(sanitizerInput).toEqual({
        content: expect.not.stringContaining(privateDetail),
      });
    }
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      "final sanitized report",
    );
    expect(result).toEqual({
      content: "final sanitized report",
      copied: true,
      errorCount: 1,
    });
  });

  it("saves only the backend-sanitized report", async () => {
    mockInvoke
      .mockResolvedValueOnce("base report from backend")
      .mockResolvedValueOnce("final sanitized report")
      .mockResolvedValueOnce("jobsentinel-support-report.txt")
      .mockResolvedValueOnce({
        fileName: "jobsentinel-support-report.txt",
        revealToken: "feedback-token",
      });

    const result = await saveSanitizedDebugReport([]);

    expect(mockInvoke).toHaveBeenNthCalledWith(2, "sanitize_feedback_text", {
      content: expect.stringContaining("No recent app problems."),
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(4, "save_feedback_file", {
      content: "final sanitized report",
      suggestedFilename: "jobsentinel-support-report.txt",
    });
    expect(result).toEqual({
      fileName: "jobsentinel-support-report.txt",
      revealToken: "feedback-token",
    });
  });

  it("keeps the dev report schema while removing uncommon local paths", () => {
    const report = generateMockFeedbackReport(
      {
        category: "bug",
        description:
          "UNC `\\\\server\\Veteran Files\\Alice Resume.pdf`\nMount `/mnt/private/Acme Notes.txt`\nNBSP\u00a0\\\\server\\share\\NBSP Resume.pdf\nComma, /srv/private/Comma Notes.txt",
        includeDebugInfo: true,
      },
      {
        keywords_boost: [],
        location_preferences: { cities: [] },
        salary_floor_usd: 0,
        alerts: {},
      },
      false,
    );
    const delivered = sanitizeMockFeedbackText({ content: report });

    expect(delivered).toContain("schema_version: 1.1");
    expect(delivered).toContain("privacy_doctor_present: false");
    expect(delivered).not.toContain("Alice Resume.pdf");
    expect(delivered).not.toContain("Acme Notes.txt");
    expect(delivered).not.toContain("NBSP Resume.pdf");
    expect(delivered).not.toContain("Comma Notes.txt");
    expect(delivered.match(/\[LOCAL_PATH\]/g)).toHaveLength(4);
  });
});
