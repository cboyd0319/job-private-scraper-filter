import { describe, expect, it } from "vitest";
import type { MockRecoveryState } from "../../../mocks/runtimeState";
import { handleMockRecoveryCommand } from "./recoveryCommands";

describe("Recovery mock commands", () => {
  const newState = (): MockRecoveryState => ({ stagedRestoreStatus: "none" });

  it("reports offline-ready local recovery with schema version 2", () => {
    const state = newState();
    const result = handleMockRecoveryCommand(
      "get_local_recovery_report",
      undefined,
      3,
      state,
    );

    expect(result.handled).toBe(true);
    expect(result.value).toMatchObject({
      schema_version: 2,
      connectivity_required: false,
      queued_local_work: {
        pending_url_imports: 3,
        capacity: 20,
        available_offline: true,
      },
      storage: { state: "ready", cleanup_available: true },
      privacy_doctor: { schema_version: 1, overall: "optional_improvement" },
      platform_health: {
        schema_version: 1,
        package_repair: { mode: "guidance_only" },
      },
    });
  });

  it("reclaims only already-free storage during cleanup", () => {
    const state = newState();
    expect(
      handleMockRecoveryCommand(
        "run_local_storage_cleanup",
        undefined,
        0,
        state,
      ).value,
    ).toMatchObject({
      state: "ready",
      reclaimable_bytes: 0,
      cleanup_available: true,
      connectivity_required: false,
    });
  });

  it("requires a 16-character passphrase for portable backup and restore", () => {
    const state = newState();
    expect(() =>
      handleMockRecoveryCommand(
        "create_portable_backup",
        { passphrase: "too-short" },
        0,
        state,
      ),
    ).toThrow("at least 16 characters");

    expect(
      handleMockRecoveryCommand(
        "create_portable_backup",
        { passphrase: "sixteen-letters!" },
        0,
        state,
      ).value,
    ).toEqual({ outcome: "succeeded", connectivity_required: false });

    expect(
      handleMockRecoveryCommand(
        "stage_portable_restore",
        { passphrase: "sixteen-letters!" },
        0,
        state,
      ).value,
    ).toEqual({
      outcome: "staged",
      connectivity_required: false,
      restart_required: true,
    });
  });

  it("keeps staged restore state through cancellation", () => {
    const state = newState();
    expect(
      handleMockRecoveryCommand(
        "stage_portable_restore",
        { passphrase: "sixteen-letters!" },
        0,
        state,
      ).value,
    ).toMatchObject({ outcome: "staged" });

    expect(
      handleMockRecoveryCommand(
        "get_staged_restore_status",
        undefined,
        0,
        state,
      ).value,
    ).toBe("ready");

    expect(
      handleMockRecoveryCommand(
        "cancel_staged_restore",
        undefined,
        0,
        state,
      ).value,
    ).toEqual({
      outcome: "cancelled",
      connectivity_required: false,
      restart_required: false,
    });

    expect(
      handleMockRecoveryCommand(
        "get_staged_restore_status",
        undefined,
        0,
        state,
      ).value,
    ).toBe("none");
    expect(
      handleMockRecoveryCommand(
        "cancel_staged_restore",
        undefined,
        0,
        state,
      ).value,
    ).toMatchObject({ outcome: "not_found" });
  });

  it("repairs only known storage areas", () => {
    const state = newState();
    expect(
      handleMockRecoveryCommand(
        "repair_local_permissions",
        { area: "cache" },
        0,
        state,
      ).value,
    ).toMatchObject({ area: "cache", outcome: "repaired", schema_version: 1 });

    expect(() =>
      handleMockRecoveryCommand(
        "repair_local_permissions",
        { area: "elsewhere" },
        0,
        state,
      ),
    ).toThrow("known local storage area");
  });

  it("rejects commands owned by another feature", () => {
    expect(
      handleMockRecoveryCommand("get_config", undefined, 0, newState()),
    ).toMatchObject({ handled: false });
  });
});
