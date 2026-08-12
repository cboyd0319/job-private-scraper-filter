import { describe, expect, it } from "vitest";
import { createDefaultSetupConfig } from "./setupWizardPreferences";
import {
  getSetupWizardSourceReviewOptions,
  toggleSetupJobSource,
} from "./setupWizardSourceReviewState";

describe("setup wizard source review state", () => {
  it("does not offer a retired broad source", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Office Manager"],
      keywords_boost: ["Scheduling"],
    };

    expect(getSetupWizardSourceReviewOptions(config)).toEqual([]);
  });

  it("toggles an active suggested source", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Software Engineer"],
    };

    const toggled = toggleSetupJobSource(config, "remoteok", true);

    expect(toggled.remoteok.enabled).toBe(true);
    expect(toggled.simplyhired.enabled).toBe(false);
  });
});
