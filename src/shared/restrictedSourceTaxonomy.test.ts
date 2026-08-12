import { describe, expect, it } from "vitest";
import { JOB_SOURCE_DISCOVERY_TAXONOMY } from "./jobSourceDiscoveryTaxonomy";
import {
  DEFAULT_RESTRICTED_SOURCE_ACKNOWLEDGEMENTS,
  RESTRICTED_AUTHENTICATED_SOURCE_WARNING,
  RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES,
  RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS,
  RESTRICTED_JOB_SOURCE_DOMAINS,
  isRestrictedJobSourceHost,
  isRestrictedJobSourceUrl,
  isPolicyBlockedAutomationUrl,
  normalizeRestrictedSourceAcknowledgements,
} from "./restrictedSourceTaxonomy";

function isConcreteHostPattern(value: string): boolean {
  return /^[a-z0-9.-]+\.[a-z]{2,}$/i.test(value);
}

function hasRestrictedSourceHost(hostname: string): boolean {
  return RESTRICTED_JOB_SOURCE_DOMAINS.some(
    (domain) => hostname === domain || hostname.endsWith(`.${domain}`),
  );
}

describe("restrictedSourceTaxonomy", () => {
  it("requires every restricted source domain to document a specific reason", () => {
    const domains = new Set<string>();
    const weakReasonPattern =
      /\b(todo|tbd|placeholder|same as above|because restricted|needs warning)\b/i;

    for (const record of RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS) {
      expect(record.domain).toBe(record.domain.trim().toLowerCase());
      expect(record.domain).toMatch(/^[a-z0-9.-]+\.[a-z]{2,}$/);
      expect(domains.has(record.domain)).toBe(false);
      domains.add(record.domain);

      expect(record.sourceRefs.length).toBeGreaterThan(0);
      expect(record.category.length).toBeGreaterThan(0);
      expect(record.reason.trim().length).toBeGreaterThanOrEqual(120);
      expect(record.reason).not.toMatch(weakReasonPattern);
    }

    expect(RESTRICTED_JOB_SOURCE_DOMAINS).toEqual(
      RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS.map((record) => record.domain),
    );
  });

  it("matches restricted source hostnames and subdomains", () => {
    expect(isRestrictedJobSourceHost("linkedin.com")).toBe(true);
    expect(isRestrictedJobSourceHost("www.glassdoor.com")).toBe(true);
    expect(isRestrictedJobSourceHost("builtin.com")).toBe(true);
    expect(isRestrictedJobSourceHost("www.builtincolorado.com")).toBe(true);
    expect(isRestrictedJobSourceHost("jobs.naukri.com")).toBe(true);
    expect(isRestrictedJobSourceHost("boards.greenhouse.io")).toBe(false);
    expect(isRestrictedJobSourceHost("jobs.lever.co")).toBe(false);
  });

  it("matches restricted source URLs without throwing on invalid input", () => {
    expect(isRestrictedJobSourceUrl("https://www.indeed.com/jobs?q=manager")).toBe(true);
    expect(isRestrictedJobSourceUrl("https://builtin.com/jobs")).toBe(true);
    expect(isRestrictedJobSourceUrl("https://remoteok.com/remote-jobs")).toBe(false);
    expect(isRestrictedJobSourceUrl("not a url")).toBe(false);
  });

  it("marks retired scheduled-source domains as policy-blocked automation", () => {
    const blocked = RESTRICTED_JOB_SOURCE_DOMAIN_RECORDS.filter(
      (record) => record.category === "policy-blocked-automation",
    ).map((record) => record.domain);

    expect(blocked).toEqual([
      "builtin.com",
      "builtincolorado.com",
      "dice.com",
      "glassdoor.com",
      "simplyhired.com",
    ]);
  });

  it("matches policy-blocked automation URLs without widening host boundaries", () => {
    expect(isPolicyBlockedAutomationUrl("https://jobs.dice.com/role/1")).toBe(true);
    expect(isPolicyBlockedAutomationUrl("https://dice.com.example/role/1")).toBe(false);
    expect(isPolicyBlockedAutomationUrl("not a url")).toBe(false);
  });

  it("normalizes legacy source acknowledgement defaults", () => {
    expect(normalizeRestrictedSourceAcknowledgements({ dice: true })).toEqual({
      ...DEFAULT_RESTRICTED_SOURCE_ACKNOWLEDGEMENTS,
      dice: true,
    });
  });

  it("centralizes restricted authenticated source session rules", () => {
    expect(RESTRICTED_INTERACTIVE_SESSION_REMINDER_MINUTES).toBe(60);
    expect(RESTRICTED_AUTHENTICATED_SOURCE_WARNING).toContain(
      "when you ask",
    );
    expect(RESTRICTED_AUTHENTICATED_SOURCE_WARNING).toContain(
      "does not save your sign-in",
    );
    expect(RESTRICTED_AUTHENTICATED_SOURCE_WARNING).toContain(
      "Use JobSentinel's local buttons and notes",
    );
  });

  it("warns for every concrete restricted discovery source host", () => {
    const missingHosts = JOB_SOURCE_DISCOVERY_TAXONOMY.flatMap((source) => {
      if (
        source.accessModel !== "restricted-user-gated" &&
        source.requiresUserAgreement !== true
      ) {
        return [];
      }

      return source.hostPatterns
        .filter(isConcreteHostPattern)
        .filter((host) => !hasRestrictedSourceHost(host))
        .map((host) => `${source.id}:${host}`);
    });

    expect(missingHosts).toEqual([]);
  });
});
