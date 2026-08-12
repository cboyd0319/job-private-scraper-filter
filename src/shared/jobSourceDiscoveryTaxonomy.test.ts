import { describe, expect, it } from "vitest";
import { CAREER_PROFILES } from "./careerProfileTaxonomy";
import {
  JOB_SOURCE_DISCOVERY_TAXONOMY,
  authenticatedJobSourceDiscoveryEntries,
  publicNativeJobSourceDiscoveryEntries,
  publicUnauthenticatedJobSourceDiscoveryEntries,
  publicUserAgreementJobSourceDiscoveryEntries,
  restrictedInteractiveJobSourceDiscoveryEntries,
  restrictedJobSourceDiscoveryEntries,
  sourceDiscoveryEntriesForCareerProfile,
  technicalAccessForJobSource,
} from "./jobSourceDiscoveryTaxonomy";
import { RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES } from "./restrictedSourceTaxonomy";

describe("jobSourceDiscoveryTaxonomy", () => {
  it("keeps source IDs unique", () => {
    const ids = JOB_SOURCE_DISCOVERY_TAXONOMY.map((entry) => entry.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("only references known career profile IDs", () => {
    const careerProfileIds = new Set(
      CAREER_PROFILES.map((profile) => profile.id),
    );

    for (const source of JOB_SOURCE_DISCOVERY_TAXONOMY) {
      if (source.careerProfileIds === "all") {
        continue;
      }

      for (const profileId of source.careerProfileIds) {
        expect(
          careerProfileIds.has(profileId),
          `${source.id} references unknown career profile ${profileId}`,
        ).toBe(true);
      }
    }
  });

  it("covers every career profile with multiple discovery paths", () => {
    for (const profile of CAREER_PROFILES) {
      const sources = sourceDiscoveryEntriesForCareerProfile(profile.id);

      expect(sources.length, `${profile.id} source count`).toBeGreaterThan(5);
      expect(
        sources.some((source) => source.category === "employer-careers"),
        `${profile.id} employer-careers coverage`,
      ).toBe(true);
      expect(
        sources.some((source) => source.category === "regional-local"),
        `${profile.id} regional-local coverage`,
      ).toBe(true);
    }
  });

  it("tracks public native and official source candidates separately from restricted boards", () => {
    const publicNativeIds = publicNativeJobSourceDiscoveryEntries().map(
      (entry) => entry.id,
    );
    const restrictedIds = restrictedJobSourceDiscoveryEntries().map(
      (entry) => entry.id,
    );

    expect(publicNativeIds).toEqual(
      expect.arrayContaining([
        "greenhouse",
        "lever",
        "ashby",
        "smartrecruiters",
        "recruitee",
        "personio",
        "jobicy",
        "usajobs",
      ]),
    );
    expect(restrictedIds).toEqual(
      expect.arrayContaining([
        "linkedin",
        "indeed",
        "builtin",
        "cv-library",
        "totaljobs",
        "stepstone",
        "otta",
      ]),
    );
    expect(publicNativeIds).not.toContain("linkedin");
  });

  it("covers roadmap ATS families with reviewed definitions or adapters", () => {
    const sourceIds = JOB_SOURCE_DISCOVERY_TAXONOMY.map((entry) => entry.id);

    expect(sourceIds).toEqual(
      expect.arrayContaining([
        "greenhouse",
        "lever",
        "ashby",
        "smartrecruiters",
        "workday",
        "workable",
        "recruitee",
        "phenom",
        "radancy-talentbrew",
        "breezy",
        "jazzhr",
        "bullhorn",
      ]),
    );

    for (const sourceId of [
      "greenhouse",
      "lever",
      "ashby",
      "smartrecruiters",
      "workday",
      "workable",
      "recruitee",
      "phenom",
      "radancy-talentbrew",
      "breezy",
      "jazzhr",
      "bullhorn",
    ]) {
      const source = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
        (entry) => entry.id === sourceId,
      );

      expect(source?.hostPatterns.length, `${sourceId} host patterns`).toBeGreaterThan(0);
      expect(source?.implementationPath, `${sourceId} implementation path`).toMatch(
        /adapter|detect|endpoint|api|import/i,
      );
      expect(source?.notes, `${sourceId} notes`).not.toHaveLength(0);
    }
  });

  it("covers API-catalog long-tail ATS families without claiming native support", () => {
    const longTailSourceIds = [
      "clearcompany",
      "dayforce",
      "avature",
      "jobdiva",
      "ceipal",
      "crelate",
      "trackerrms",
      "vincere",
      "applicantpro",
      "applicantstack",
      "homerun",
      "manatal",
      "recruit-crm",
      "loxo",
      "hibob",
      "factorial",
      "join",
      "polymer",
      "recooty",
    ];

    for (const sourceId of longTailSourceIds) {
      const source = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
        (entry) => entry.id === sourceId,
      );

      expect(source, `${sourceId} entry`).toBeDefined();
      expect(source).toMatchObject({
        category: "ats-platform",
        accessModel: "employer-career-system",
        status: "research",
      });
      expect(source?.hostPatterns.length, `${sourceId} hosts`).toBeGreaterThan(0);
      expect(source?.implementationPath, `${sourceId} implementation`).toMatch(
        /detect|review/i,
      );
      expect(source?.notes, `${sourceId} notes`).toMatch(/source-specific/i);
    }
  });

  it("records enterprise ATS validation status and exclusions from local API research", () => {
    const sourceById = (id: string) =>
      JOB_SOURCE_DISCOVERY_TAXONOMY.find((entry) => entry.id === id);

    expect(sourceById("workday")?.notes).toMatch(
      /19 public employer tenants with 57 canonical sample rows/i,
    );
    expect(sourceById("phenom")?.notes).toMatch(
      /13 public employer pages with 39 canonical sample rows/i,
    );
    expect(sourceById("radancy-talentbrew")?.notes).toMatch(
      /Sysco public HTML fallback lane with 45 canonical sample rows/i,
    );
    expect(sourceById("oracle-recruiting")).toMatchObject({
      technicalAccess: "public-unauthenticated",
    });
    expect(sourceById("oracle-recruiting")?.notes).toMatch(/JPMorgan Chase remains excluded/i);
    expect(sourceById("taleo")).toMatchObject({
      technicalAccess: "public-unauthenticated",
    });
    expect(sourceById("taleo")?.notes).toMatch(/UnitedHealth Group remains excluded/i);
    expect(sourceById("smartrecruiters")?.notes).toMatch(
      /policy-blocked by robots/i,
    );
    expect(sourceById("eightfold")?.notes).toMatch(
      /Microsoft has a frontend JSON endpoint candidate/i,
    );
    expect(sourceById("icims")?.notes).toMatch(
      /State Farm and Liberty Mutual/i,
    );
    expect(sourceById("sap-successfactors")?.notes).toMatch(
      /American Airlines/i,
    );
  });

  it("covers employer career-page discovery examples outside broad boards", () => {
    const employerCareers = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "employer-careers-pages",
    );

    expect(employerCareers).toMatchObject({
      category: "employer-careers",
      accessModel: "employer-career-system",
      careerProfileIds: "all",
    });
    expect(employerCareers?.hostPatterns).toEqual(
      expect.arrayContaining(["careers.*", "*/careers", "*/jobs"]),
    );
    expect(employerCareers?.examples).toEqual(
      expect.arrayContaining([
        "Fivetran Careers",
        "SpaceX Careers",
        "Google Careers",
        "Yahoo Careers",
        "IBM Careers",
        "Microsoft Careers",
      ]),
    );
  });

  it("covers requested country and regional discovery markets", () => {
    const regionEntries = (region: string) =>
      JOB_SOURCE_DISCOVERY_TAXONOMY.filter((entry) =>
        entry.regions.includes(region),
      );

    expect(regionEntries("US").map((entry) => entry.id)).toEqual(
      expect.arrayContaining(["builtin", "usajobs", "state-workforce-boards"]),
    );
    expect(regionEntries("UK").map((entry) => entry.id)).toEqual(
      expect.arrayContaining(["cv-library", "totaljobs", "gov-uk-find-a-job"]),
    );
    expect(regionEntries("India").map((entry) => entry.id)).toEqual(
      expect.arrayContaining(["naukri", "foundit", "shine", "timesjobs"]),
    );
    expect(regionEntries("Bangladesh").map((entry) => entry.id)).toEqual(
      expect.arrayContaining(["bdjobs"]),
    );
    expect(regionEntries("Canada").map((entry) => entry.id)).toEqual(
      expect.arrayContaining(["canada-job-bank"]),
    );
  });

  it("records source-mined candidates without claiming native support", () => {
    const sourceById = (id: string) =>
      JOB_SOURCE_DISCOVERY_TAXONOMY.find((entry) => entry.id === id);

    expect(sourceById("naukri")).toMatchObject({
      accessModel: "restricted-user-gated",
      status: "candidate",
      requiresUserAgreement: true,
    });
    expect(sourceById("bayt")).toMatchObject({
      accessModel: "restricted-user-gated",
      status: "candidate",
      requiresUserAgreement: true,
    });
    expect(sourceById("bdjobs")).toMatchObject({
      accessModel: "restricted-user-gated",
      status: "candidate",
      requiresUserAgreement: true,
    });
    expect(sourceById("google-jobs-search")).toMatchObject({
      accessModel: "review-required",
      status: "candidate",
    });
  });

  it("keeps Greenhouse current and legacy host coverage explicit", () => {
    const greenhouse = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "greenhouse",
    );

    expect(greenhouse?.hostPatterns).toEqual(
      expect.arrayContaining([
        "boards-api.greenhouse.io",
        "job-boards.greenhouse.io",
        "boards.greenhouse.io",
      ]),
    );
  });

  it("treats Built In and LinkedIn as restricted user-gated sources", () => {
    const builtin = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "builtin",
    );
    const linkedin = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "linkedin",
    );
    const linkedinJobsTracker = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "linkedin-jobs-tracker",
    );

    expect(builtin).toMatchObject({
      accessModel: "restricted-user-gated",
      status: "manual-only",
      requiresUserAgreement: true,
    });
    expect("technicalAccess" in builtin!).toBe(false);
    expect(technicalAccessForJobSource(builtin!)).toBe("public-unauthenticated");
    expect(builtin?.hostPatterns).toEqual(
      expect.arrayContaining(["builtin.com", "builtincolorado.com"]),
    );
    expect(builtin?.locationSearchPatterns).toEqual(
      expect.arrayContaining([
        "/jobs",
        "/jobs?state=<state>&country=<country>&allLocations=true",
        "/jobs?city=<city>&state=<state>&country=<country>&allLocations=true",
      ]),
    );
    expect(linkedin).toMatchObject({
      accessModel: "restricted-user-gated",
      technicalAccess: "authenticated-user-session",
      requiresUserAgreement: true,
    });
    expect(linkedin?.searchParameterPatterns).toEqual(
      expect.arrayContaining([
        "keywords=<role-or-skill>",
        "geoId=<linkedin-location-id>",
        "f_TPR=<posted-within-seconds>",
        "f_AL=true",
        "referralSearchId=<session-search-id>",
      ]),
    );
    expect(linkedin?.navigationSurfacePatterns).toEqual(
      expect.arrayContaining([
        "preferences",
        "job tracker",
        "my career insights",
      ]),
    );
    expect(linkedinJobsTracker).toMatchObject({
      accessModel: "restricted-user-gated",
      technicalAccess: "authenticated-user-session",
      requiresUserAgreement: true,
    });
    expect(linkedinJobsTracker?.searchParameterPatterns).toEqual(
      expect.arrayContaining(["stage=applied", "stage=saved"]),
    );
  });

  it("keeps retired scheduled boards manual-only after policy review", () => {
    for (const id of ["builtin", "dice", "glassdoor", "simplyhired"]) {
      const source = JOB_SOURCE_DISCOVERY_TAXONOMY.find((entry) => entry.id === id);

      expect(source).toMatchObject({
        status: "manual-only",
        requiresUserAgreement: true,
      });
      expect(source?.implementationPath).toContain(
        "Scheduled and pasted-URL fetches are policy-disabled.",
      );
    }
  });

  it("separates public consent-gated sources from authenticated user sessions", () => {
    const publicUnauthenticatedIds =
      publicUnauthenticatedJobSourceDiscoveryEntries().map((entry) => entry.id);
    const publicAgreementIds =
      publicUserAgreementJobSourceDiscoveryEntries().map((entry) => entry.id);
    const authenticatedIds = authenticatedJobSourceDiscoveryEntries().map(
      (entry) => entry.id,
    );

    expect(publicUnauthenticatedIds).toEqual(
      expect.arrayContaining(["greenhouse", "indeed", "builtin", "dice"]),
    );
    expect(publicAgreementIds).toEqual(
      expect.arrayContaining([
        "indeed",
        "builtin",
        "dice",
        "glassdoor",
      ]),
    );
    expect(authenticatedIds).toEqual(
      expect.arrayContaining([
        "linkedin",
        "linkedin-jobs-tracker",
        "flexjobs",
        "upwork",
        "freelancer",
        "toptal",
      ]),
    );

    expect(publicAgreementIds).not.toContain("linkedin");
    expect(publicAgreementIds).not.toContain("upwork");
    expect(authenticatedIds).not.toContain("indeed");
    expect(authenticatedIds).not.toContain("builtin");
  });

  it("keeps reviewed public APIs and public feeds low-friction", () => {
    const lowFrictionPublicIds = publicNativeJobSourceDiscoveryEntries().map(
      (entry) => entry.id,
    );

    expect(lowFrictionPublicIds).toEqual(
      expect.arrayContaining([
        "greenhouse",
        "lever",
        "remoteok",
        "remote-first-jobs",
        "we-work-remotely",
        "hacker-news-who-is-hiring",
      ]),
    );

    for (const source of publicNativeJobSourceDiscoveryEntries()) {
      expect(source.requiresUserAgreement).not.toBe(true);
      expect(source.restrictedInteractiveSessionPolicy).toBeUndefined();
      expect(technicalAccessForJobSource(source)).not.toBe(
        "authenticated-user-session",
      );
    }
  });

  it("keeps YC user-opened without an automation agreement path", () => {
    const yc = JOB_SOURCE_DISCOVERY_TAXONOMY.find(
      (entry) => entry.id === "yc-work-at-a-startup",
    );

    expect(yc).toMatchObject({
      accessModel: "review-required",
      status: "manual-only",
      implementationPath: "Open the YC search in the user's browser.",
    });
    expect(yc?.requiresUserAgreement).not.toBe(true);
  });

  it("caps restricted authenticated interactive sessions and forbids auth storage", () => {
    const interactiveSources = restrictedInteractiveJobSourceDiscoveryEntries();

    expect(interactiveSources.map((entry) => entry.id).sort()).toEqual([
      "linkedin",
      "linkedin-jobs-tracker",
    ]);

    for (const source of interactiveSources) {
      expect(technicalAccessForJobSource(source)).toBe(
        "authenticated-user-session",
      );
      expect(source.restrictedInteractiveSessionPolicy).toMatchObject({
        requiresUserInitiatedAction: true,
        requiresFreshLogin: true,
        preLoginWarningRequired: true,
        storesAuthTokens: false,
        storesSessionCookies: false,
        storesBrowserStorage: false,
        storesAuthorizationHeaders: false,
        backgroundAutomationAllowed: false,
        offlineUseAllowed: false,
        privacyReminderMinutes: RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES,
        hardSessionExpiryRequired: false,
      });
    }
  });

  it("does not time-gate unauthenticated restricted public sources", () => {
    for (const source of publicUserAgreementJobSourceDiscoveryEntries()) {
      expect(technicalAccessForJobSource(source)).toBe("public-unauthenticated");
      expect(source.restrictedInteractiveSessionPolicy).toBeUndefined();
    }
  });
});
