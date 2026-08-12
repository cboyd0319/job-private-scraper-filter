import { describe, expect, it } from "vitest";
import {
  formatJobSourceLabel,
  getJobSourceGuidance,
} from "./jobSourceGuidance";

describe("sourceLabels", () => {
  it("normalizes common source keys before lookup", () => {
    expect(formatJobSourceLabel("Manual Import")).toBe("Saved by you");
    expect(formatJobSourceLabel("manual_import")).toBe("Saved by you");
    expect(formatJobSourceLabel("user-source-actions")).toBe("Saved by you");
  });

  it("labels employer-side hiring sources in plain language", () => {
    const guidance = getJobSourceGuidance("greenhouse");

    expect(guidance.label).toBe("Greenhouse hiring page");
    expect(guidance.description).toContain("verify before tailoring");
    expect(guidance.review).toBeUndefined();
  });

  it("labels community hiring posts without acronym-first wording", () => {
    expect(formatJobSourceLabel("hn_hiring")).toBe("Startup and tech job posts");
  });

  it("cleans unknown source IDs before rendering", () => {
    const guidance = getJobSourceGuidance("city_careers");

    expect(formatJobSourceLabel("city_careers")).toBe("City Careers");
    expect(guidance.review?.title).toBe("Check source before tailoring");
    expect(guidance.review?.description).toContain("only has the source label");
  });

  it("adds visible review guidance for job boards and saved links", () => {
    expect(getJobSourceGuidance("LinkedIn").review?.title).toBe(
      "Verify employer page",
    );
    expect(getJobSourceGuidance("manual_import").review?.title).toBe(
      "Check saved link",
    );
  });

  it("labels missing source data without implying it came from the posting", () => {
    const guidance = getJobSourceGuidance("   ");

    expect(guidance.label).toBe("Source not shown");
    expect(guidance.description).toBe(
      "No source was recorded for this posting. Open the original job page before tailoring.",
    );
    expect(guidance.review?.ariaLabel).toBe(
      "Source not shown, open original job page before tailoring",
    );
    expect(guidance.review?.tone).toBe("warning");
  });
});
