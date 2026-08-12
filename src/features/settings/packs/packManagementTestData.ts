/** Provides signed-pack management response fixtures for renderer tests. */

export function release(overrides: Record<string, unknown> = {}) {
  return {
    releaseSequence: 2,
    packVersion: "3.0.2",
    packType: "source",
    executionClass: "static_content",
    state: "ready",
    quarantineReason: null,
    artifactCleanupPending: false,
    isActive: true,
    isRollback: false,
    isHighWater: true,
    publisherName: "JobSentinel",
    license: "MIT",
    minimumAppVersion: "3.0.0",
    maximumAppVersion: "3.2.0",
    payloadBytes: 2048,
    fixtureSummary: "Three reviewed source fixtures",
    purpose: "source_support",
    modelDownloadRequired: false,
    lastSelfTestedAt: "2026-08-11T18:30:00Z",
    privacyLabels: ["local_only"],
    allowedDataCategories: [],
    allowedTaskKinds: [],
    allowedActions: [],
    approvalGates: [],
    gatewayPolicyId: null,
    externalDestinations: [],
    usesExternalAi: false,
    ...overrides,
  };
}

export function pack(overrides: Record<string, unknown> = {}) {
  const currentRelease = overrides.currentRelease ?? release();
  return {
    publisherKeyId: "jobsentinel-release-v1",
    publisherPublicKeySha256:
      "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef0123",
    packId: "jobsentinel.sources.us",
    state: "ready",
    updateAvailable: false,
    cleanupPending: false,
    generation: 4,
    currentRelease,
    releases: overrides.releases ?? [currentRelease],
    ...overrides,
  };
}
