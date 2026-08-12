import { renderHook } from "@testing-library/react";
import type { Dispatch, SetStateAction } from "react";
import { describe, expect, it, vi } from "vitest";
import type { Config } from "../config/SettingsConfig";
import { makeConfig as makeBaseConfig } from "../SettingsPage.testFixtures";
import { useJobBoardRecommendations } from "./SettingsJobBoardRecommendations";

function makeConfig(overrides: Partial<Config> = {}): Config {
  return {
    ...makeBaseConfig(),
    keywords_boost: [],
    salary_floor_usd: 0,
    ...overrides,
  };
}

function renderRecommendations(config: Config) {
  const setConfig = vi.fn() as unknown as Dispatch<
    SetStateAction<Config | null>
  >;
  return renderHook(() => useJobBoardRecommendations(config, setConfig)).result
    .current;
}

describe("useJobBoardRecommendations", () => {
  it("does not recommend retired YC automation for startup intent", () => {
    const recommendations = renderRecommendations(
      makeConfig({ keywords_boost: ["seed stage operations"] }),
    );

    expect(recommendations.map((item) => item.board)).not.toContain(
      "YC Startups",
    );
  });

  it("recommends USAJobs from shared government intent terms", () => {
    const recommendations = renderRecommendations(
      makeConfig({ title_allowlist: ["Public Sector Program Analyst"] }),
    );

    expect(recommendations.map((item) => item.board)).toContain("USAJobs");
  });

  it("does not recommend retired restricted scheduled sources", () => {
    const recommendations = renderRecommendations(
      makeConfig({
        title_allowlist: ["Software Engineer"],
        location_preferences: {
          allow_remote: false,
          allow_hybrid: true,
          allow_onsite: true,
          cities: ["Austin"],
        },
      }),
    );

    const boards = recommendations.map((item) => item.board);
    expect(boards).not.toContain("BuiltIn");
    expect(boards).not.toContain("Dice");
  });
});
