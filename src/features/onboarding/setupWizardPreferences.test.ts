import { describe, expect, it } from "vitest";
import {
  applyReviewVolumePreference,
  buildSetupSearchSummary,
  createDefaultSetupConfig,
  formatSetupPayFloorSummary,
  getSuggestedJobSourceOptions,
  ghostConfigForFreshnessPreference,
  normalizeSetupPayFloorUsd,
  toResumeSkillSuggestions,
} from "./setupWizardPreferences";

describe("Setup Wizard preference helpers", () => {
  it("creates default first-run config without enabling external sources", () => {
    const config = createDefaultSetupConfig();

    expect(config.location_preferences).toMatchObject({
      allow_remote: true,
      allow_hybrid: true,
      allow_onsite: true,
      cities: [],
    });
    expect(config.salary_floor_usd).toBe(0);
    expect(config.alerts.desktop.enabled).toBe(false);
    expect(config.alerts.desktop.play_sound).toBe(false);
    expect(config.alerts.desktop.show_when_focused).toBe(false);
    expect(config.alerts.slack.webhook_url).toBe("");
    expect(config.remoteok.enabled).toBe(false);
    expect(config.hn_hiring.enabled).toBe(false);
    expect(config.weworkremotely.enabled).toBe(false);
    expect(config.simplyhired.enabled).toBe(false);
    expect(config.ghost_config).toEqual(
      ghostConfigForFreshnessPreference("fresh_verified_first")
    );
  });

  it("applies review volume without changing enabled source choices", () => {
    const config = {
      ...createDefaultSetupConfig(),
      remoteok: { enabled: true, tags: ["healthcare"], limit: 50 },
      hn_hiring: { enabled: false, remote_only: true, limit: 100 },
      weworkremotely: { enabled: true, limit: 50 },
      simplyhired: { enabled: true, query: "Office Manager", limit: 50 },
    };

    expect(applyReviewVolumePreference(config, "focused")).toMatchObject({
      immediate_alert_threshold: 0.92,
      remoteok: { enabled: true, tags: ["healthcare"], limit: 25 },
      hn_hiring: { enabled: false, remote_only: true, limit: 50 },
      weworkremotely: { enabled: true, limit: 25 },
      simplyhired: { enabled: true, query: "Office Manager", limit: 25 },
    });
  });

  it("keeps resume skill suggestions local, deduped, and review-sized", () => {
    expect(
      toResumeSkillSuggestions([
        { skill_name: "Scheduling", source: "resume" },
        { skill_name: " scheduling ", source: "resume" },
        { skill_name: "Case management", source: "manual" },
        { skill_name: "", source: "resume" },
        { skill_name: "Client service", source: "resume" },
        { skill_name: "Calendar management", source: "resume" },
        { skill_name: "Inventory", source: "resume" },
        { skill_name: "Data entry", source: "resume" },
        { skill_name: "Front desk", source: "resume" },
        { skill_name: "Extra", source: "resume" },
      ])
    ).toEqual([
      "Scheduling",
      "Client service",
      "Calendar management",
      "Inventory",
      "Data entry",
      "Front desk",
    ]);
  });

  it("builds plain-language search summaries from config", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Office Coordinator"],
      keywords_boost: ["Scheduling"],
      keywords_exclude: ["night shift"],
      location_preferences: {
        allow_remote: true,
        allow_hybrid: true,
        allow_onsite: false,
        cities: ["Denver"],
      },
      salary_floor_usd: 60000,
      alerts: {
        ...createDefaultSetupConfig().alerts,
        desktop: {
          enabled: true,
          play_sound: false,
          show_when_focused: false,
        },
      },
    };

    expect(buildSetupSearchSummary(config, "balanced", "broad")).toMatchObject({
      titles: "Office Coordinator",
      wantedWork: "Scheduling",
      avoidedWork: "night shift",
      location: "remote, hybrid near Denver",
      freshness: "Balanced",
      reviewVolume: "Broad discovery",
      jobSources: "No outside job sources selected; add reviewed sources in Settings.",
      alerts: "Quiet desktop alerts; no sound",
      pay: "At least $60,000/year",
    });
  });

  it("converts hourly setup pay to yearly stored pay", () => {
    expect(normalizeSetupPayFloorUsd("20", "hourly")).toBe(41600);
    expect(normalizeSetupPayFloorUsd("60000", "yearly")).toBe(60000);
    expect(normalizeSetupPayFloorUsd("", "hourly")).toBe(0);
  });

  it("summarizes hourly setup pay without hiding yearly storage meaning", () => {
    expect(formatSetupPayFloorSummary(41600, "hourly")).toBe(
      "At least $20/hour, about $41,600/year",
    );
  });

  it("summarizes desktop alerts as off until the user turns them on", () => {
    const config = createDefaultSetupConfig();

    expect(buildSetupSearchSummary(config, "balanced", "balanced")).toMatchObject({
      alerts: "Desktop alerts off; add alerts later in Settings",
    });
  });

  it("suggests technical sources without selecting them", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Software Engineer"],
      keywords_boost: ["React"],
    };

    expect(getSuggestedJobSourceOptions(config).map((source) => source.key)).toEqual([
      "remoteok",
      "weworkremotely",
      "hn_hiring",
    ]);
    expect(buildSetupSearchSummary(config, "balanced", "balanced")).toMatchObject({
      jobSources: "No outside job sources selected; add reviewed sources in Settings.",
    });
  });

  it("suggests technical sources for reviewed data analyst searches without selecting them", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Data Analyst"],
      keywords_boost: ["Tableau"],
    };

    expect(getSuggestedJobSourceOptions(config).map((source) => source.key)).toEqual([
      "remoteok",
      "weworkremotely",
      "hn_hiring",
    ]);
    expect(buildSetupSearchSummary(config, "balanced", "balanced")).toMatchObject({
      jobSources: "No outside job sources selected; add reviewed sources in Settings.",
    });
  });

  it("does not suggest a retired scheduled source for non-technical searches", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Office Manager"],
      keywords_boost: ["Scheduling"],
    };

    expect(getSuggestedJobSourceOptions(config)).toEqual([]);
    expect(buildSetupSearchSummary(config, "balanced", "balanced")).toMatchObject({
      jobSources: "No outside job sources selected; add reviewed sources in Settings.",
    });
  });

  it("does not summarize a retired source from legacy config", () => {
    const config = {
      ...createDefaultSetupConfig(),
      title_allowlist: ["Software Engineer"],
      keywords_boost: ["React"],
      remoteok: {
        ...createDefaultSetupConfig().remoteok,
        enabled: true,
      },
      simplyhired: {
        ...createDefaultSetupConfig().simplyhired,
        enabled: true,
      },
    };

    expect(buildSetupSearchSummary(config, "balanced", "balanced")).toMatchObject({
      jobSources: "Remote OK selected.",
    });
  });
});
