import { useMemo, type Dispatch, type SetStateAction } from "react";
import {
  GOVERNMENT_SOURCE_TERMS,
  REMOTE_INTENT_TERMS,
} from "../../../shared/jobSourceRecommendationTaxonomy";
import { searchLooksTechFocused } from "../../../shared/jobSourceRecommendations";
import type { Config } from "../config/SettingsConfig";

export interface JobBoardRecommendation {
  board: string;
  reason: string;
  enable: () => void;
}

function includesAnyTerm(values: string[], terms: readonly string[]): boolean {
  return values.some((value) =>
    terms.some((term) => value.toLowerCase().includes(term)),
  );
}

export function useJobBoardRecommendations(
  config: Config | null,
  setConfig: Dispatch<SetStateAction<Config | null>>,
): JobBoardRecommendation[] {
  return useMemo(() => {
    if (!config) {
      return [];
    }

    const recommendations: JobBoardRecommendation[] = [];
    const keywords = [
      ...(config.keywords_boost ?? []),
      ...(config.title_allowlist ?? []),
    ].map((k) => k.toLowerCase());
    const allowRemote = config.location_preferences?.allow_remote ?? false;
    const isTechFocused = searchLooksTechFocused(keywords);
    const hasRemoteIntent =
      allowRemote || includesAnyTerm(keywords, REMOTE_INTENT_TERMS);

    if (isTechFocused && hasRemoteIntent) {
      if (!config.remoteok?.enabled) {
        recommendations.push({
          board: "RemoteOK",
          reason: "Useful for remote tech roles",
          enable: () =>
            setConfig({
              ...config,
              remoteok: {
                ...config.remoteok,
                enabled: true,
                tags: config.remoteok?.tags ?? [],
                limit: 50,
              },
            }),
        });
      }
      if (!config.weworkremotely?.enabled) {
        recommendations.push({
          board: "WeWorkRemotely",
          reason: "Useful for remote tech and product roles",
          enable: () =>
            setConfig({
              ...config,
              weworkremotely: {
                ...config.weworkremotely,
                enabled: true,
                limit: 50,
              },
            }),
        });
      }
    }

    if (isTechFocused) {
      if (!config.hn_hiring?.enabled) {
        recommendations.push({
          board: "Startup and tech job posts",
          reason: "Active monthly startup and tech hiring posts",
          enable: () =>
            setConfig({
              ...config,
              hn_hiring: {
                ...config.hn_hiring,
                enabled: true,
                remote_only: false,
                limit: 50,
              },
            }),
        });
      }
    }

    if (includesAnyTerm(keywords, GOVERNMENT_SOURCE_TERMS)) {
      if (!config.usajobs?.enabled) {
        recommendations.push({
          board: "USAJobs",
          reason: "You're interested in government roles",
          enable: () =>
            setConfig({
              ...config,
              usajobs: {
                ...config.usajobs,
                enabled: true,
                email: config.usajobs?.email ?? "",
                remote_only: false,
                date_posted_days: 30,
                limit: 100,
              },
            }),
        });
      }
    }

    return recommendations.slice(0, 3);
  }, [config, setConfig]);
}
