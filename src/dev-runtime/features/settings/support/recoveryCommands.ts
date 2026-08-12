import {
  getArg,
  getStringArg,
} from "../../../mocks/handlers/commandHelpers";
import type { MockRecoveryState } from "../../../mocks/runtimeState";

export interface MockRecoveryCommandResult {
  handled: boolean;
  shouldSave: boolean;
  value: unknown;
}

type MockStorageArea = "application_data" | "configuration" | "cache";
type MockPrivacyState =
  | "looks_good"
  | "needs_attention"
  | "paused_for_safety"
  | "optional_improvement";

interface MockLocalStorageRecoveryReport {
  state: "ready" | "restore_from_backup_required" | "unavailable";
  reclaimable_bytes: number;
  wal_bytes: number | null;
  incremental_vacuum_supported: boolean;
  cleanup_available: boolean;
  connectivity_required: false;
}

interface MockPrivacyDoctorCheck {
  id: string;
  state: MockPrivacyState;
  message: string;
  action: string | null;
  connectivity_required: false;
}

interface MockPlatformPermissionCheck {
  area: MockStorageArea;
  state: "private" | "missing" | "needs_repair" | "manual_review" | "unchecked";
  action: null | "repair_locally" | "follow_manual_guidance";
  connectivity_required: false;
}

interface MockLocalRecoveryReport {
  schema_version: number;
  connectivity_required: false;
  queued_local_work: {
    pending_url_imports: number;
    capacity: number;
    available_offline: true;
    connectivity_required: false;
  };
  storage: MockLocalStorageRecoveryReport;
  privacy_doctor: {
    schema_version: number;
    overall: MockPrivacyState;
    checks: MockPrivacyDoctorCheck[];
    connectivity_required: false;
  };
  platform_health: {
    schema_version: number;
    permissions: MockPlatformPermissionCheck[];
    package_repair: {
      mode: "guidance_only";
      actions: {
        action:
          | "use_downloaded_verified_installer"
          | "obtain_verified_installer";
        connectivity_required: boolean;
      }[];
    };
  };
}

const MOCK_STORAGE_AREAS: readonly MockStorageArea[] = [
  "application_data",
  "configuration",
  "cache",
];
const PENDING_URL_IMPORT_CAPACITY = 20;
const MIN_PORTABLE_PASSPHRASE_CHARS = 16;

export function handleMockRecoveryCommand(
  command: string,
  args: Record<string, unknown> | undefined,
  pendingUrlImportCount: number,
  state: MockRecoveryState,
): MockRecoveryCommandResult {
  switch (command) {
    case "get_local_recovery_report":
      return handled(getMockLocalRecoveryReport(pendingUrlImportCount));

    case "run_local_storage_cleanup":
      return handled(getMockStorageReport(0));

    case "repair_local_permissions":
      return handled(repairMockLocalPermissions(args));

    case "repair_invalid_startup_config":
      return handled({
        outcome: "preserved_and_reset",
        connectivity_required: false,
      });

    case "create_portable_backup":
      requireMockPortablePassphrase(args);
      return handled({ outcome: "succeeded", connectivity_required: false });

    case "create_reviewed_export":
      return handled({ outcome: "succeeded", connectivity_required: false });

    case "stage_portable_restore":
      requireMockPortablePassphrase(args);
      state.stagedRestoreStatus = "ready";
      return handled(
        {
          outcome: "staged",
          connectivity_required: false,
          restart_required: true,
        },
        true,
      );

    case "get_staged_restore_status":
      return handled(state.stagedRestoreStatus);

    case "cancel_staged_restore":
      if (state.stagedRestoreStatus === "none") {
        return handled({
          outcome: "not_found",
          connectivity_required: false,
          restart_required: false,
        });
      }
      state.stagedRestoreStatus = "none";
      return handled(
        {
          outcome: "cancelled",
          connectivity_required: false,
          restart_required: false,
        },
        true,
      );

    default:
      return { handled: false, shouldSave: false, value: undefined };
  }
}

function handled(
  value: unknown,
  shouldSave = false,
): MockRecoveryCommandResult {
  return { handled: true, shouldSave, value };
}

function requireMockPortablePassphrase(
  args: Record<string, unknown> | undefined,
): void {
  const passphrase = getStringArg(args, "passphrase") ?? "";
  if ([...passphrase].length < MIN_PORTABLE_PASSPHRASE_CHARS) {
    throw new Error("Use a backup passphrase with at least 16 characters.");
  }
}

function isMockStorageArea(value: unknown): value is MockStorageArea {
  return MOCK_STORAGE_AREAS.includes(value as MockStorageArea);
}

function repairMockLocalPermissions(
  args: Record<string, unknown> | undefined,
): {
  schema_version: number;
  area: MockStorageArea;
  outcome: "repaired";
  connectivity_required: false;
} {
  const area = getArg(args, "area");
  if (!isMockStorageArea(area)) {
    throw new Error("Choose a known local storage area to repair.");
  }
  return {
    schema_version: 1,
    area,
    outcome: "repaired",
    connectivity_required: false,
  };
}

function getMockStorageReport(
  reclaimableBytes: number,
): MockLocalStorageRecoveryReport {
  return {
    state: "ready",
    reclaimable_bytes: reclaimableBytes,
    wal_bytes: 65536,
    incremental_vacuum_supported: true,
    cleanup_available: true,
    connectivity_required: false,
  };
}

function getMockLocalRecoveryReport(
  pendingUrlImportCount: number,
): MockLocalRecoveryReport {
  return {
    schema_version: 2,
    connectivity_required: false,
    queued_local_work: {
      pending_url_imports: pendingUrlImportCount,
      capacity: PENDING_URL_IMPORT_CAPACITY,
      available_offline: true,
      connectivity_required: false,
    },
    storage: getMockStorageReport(262144),
    privacy_doctor: {
      schema_version: 1,
      overall: "optional_improvement",
      checks: [
        {
          id: "telemetry",
          state: "looks_good",
          message: "JobSentinel does not send telemetry.",
          action: null,
          connectivity_required: false,
        },
        {
          id: "storage",
          state: "looks_good",
          message: "Local storage passed its integrity checks.",
          action: null,
          connectivity_required: false,
        },
        {
          id: "backup_history",
          state: "optional_improvement",
          message: "No portable backup creation is recorded.",
          action: "create_portable_backup",
          connectivity_required: false,
        },
        {
          id: "credential_vault",
          state: "optional_improvement",
          message: "System credential storage is checked only when you use it.",
          action: null,
          connectivity_required: false,
        },
        {
          id: "external_ai",
          state: "looks_good",
          message: "External AI is disabled.",
          action: null,
          connectivity_required: false,
        },
        {
          id: "browser_import",
          state: "looks_good",
          message: "Browser Import is stopped or has a current one-use pairing.",
          action: null,
          connectivity_required: false,
        },
        {
          id: "sources",
          state: "looks_good",
          message: "Enabled restricted sources have local acknowledgements.",
          action: null,
          connectivity_required: false,
        },
      ],
      connectivity_required: false,
    },
    platform_health: {
      schema_version: 1,
      permissions: [
        {
          area: "application_data",
          state: "private",
          action: null,
          connectivity_required: false,
        },
        {
          area: "configuration",
          state: "private",
          action: null,
          connectivity_required: false,
        },
        {
          area: "cache",
          state: "needs_repair",
          action: "repair_locally",
          connectivity_required: false,
        },
      ],
      package_repair: {
        mode: "guidance_only",
        actions: [
          {
            action: "use_downloaded_verified_installer",
            connectivity_required: false,
          },
          {
            action: "obtain_verified_installer",
            connectivity_required: true,
          },
        ],
      },
    },
  };
}
