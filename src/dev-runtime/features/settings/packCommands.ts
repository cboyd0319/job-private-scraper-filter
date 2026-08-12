/** Provides deterministic, stateful pack management behavior in browser development. */

import type {
  PackManagementReview,
  PackReleaseReview,
} from "../../../features/settings/packs/packManagementProjection";

const activeRelease: PackReleaseReview = {
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
};

const removedRelease: PackReleaseReview = {
  ...activeRelease,
  releaseSequence: 1,
  packVersion: "3.0.1",
  state: "removed",
  artifactCleanupPending: true,
  isActive: false,
  isHighWater: false,
};

const defaultPack: PackManagementReview = {
  publisherKeyId: "jobsentinel-release-v1",
  publisherPublicKeySha256:
    "a3b1c9d7e5f40123456789abcdef0123456789abcdef0123456789abcdef0123",
  packId: "jobsentinel.sources.us",
  state: "ready",
  updateAvailable: false,
  cleanupPending: true,
  generation: 4,
  currentRelease: activeRelease,
  releases: [activeRelease, removedRelease],
};

let packs: PackManagementReview[] = [];

function cloneRelease(release: PackReleaseReview): PackReleaseReview {
  return {
    ...release,
    privacyLabels: [...release.privacyLabels],
    allowedDataCategories: [...release.allowedDataCategories],
    allowedTaskKinds: [...release.allowedTaskKinds],
    allowedActions: [...release.allowedActions],
    approvalGates: [...release.approvalGates],
    externalDestinations: [...release.externalDestinations],
  };
}

function clonePack(pack: PackManagementReview): PackManagementReview {
  return {
    ...pack,
    currentRelease: cloneRelease(pack.currentRelease),
    releases: pack.releases.map(cloneRelease),
  };
}

export function resetMockPackManagement(): void {
  packs = [clonePack(defaultPack)];
}

function selectedPack(args: Record<string, unknown> | undefined) {
  const publisherKeyId = args?.publisherKeyId;
  const packId = args?.packId;
  const expectedGeneration = args?.expectedGeneration;
  if (
    typeof publisherKeyId !== "string" ||
    typeof packId !== "string" ||
    typeof expectedGeneration !== "number"
  ) {
    throw new Error("invalid pack identity");
  }
  const pack = packs.find(
    (candidate) =>
      candidate.publisherKeyId === publisherKeyId && candidate.packId === packId,
  );
  if (!pack || pack.generation !== expectedGeneration) {
    throw new Error("pack changed; refresh before trying again");
  }
  return pack;
}

export function handleMockPackCommand(
  command: string,
  args: Record<string, unknown> | undefined,
): unknown {
  if (command === "list_pack_management") return packs.map(clonePack);
  if (
    command !== "disable_pack" &&
    command !== "uninstall_pack" &&
    command !== "retry_pack_cleanup"
  ) return undefined;

  const pack = selectedPack(args);
  if (command === "disable_pack") {
    pack.state = "disabled";
    pack.generation += 1;
    return { state: pack.state, generation: pack.generation };
  }
  if (command === "uninstall_pack") {
    pack.state = "removed";
    pack.generation += 1;
    pack.cleanupPending = false;
    pack.currentRelease = {
      ...pack.currentRelease,
      state: "removed",
      isActive: false,
      artifactCleanupPending: false,
    };
    pack.releases = pack.releases.map((release) => ({
      ...release,
      state: "removed",
      isActive: false,
      artifactCleanupPending: false,
    }));
    return { generation: pack.generation, cleanupPending: false };
  }

  pack.cleanupPending = false;
  pack.releases = pack.releases.map((release) => ({
    ...release,
    artifactCleanupPending: false,
  }));
  return { generation: pack.generation, cleanupPending: false };
}

resetMockPackManagement();
