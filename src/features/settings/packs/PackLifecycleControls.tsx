/** Owns reviewed, generation-bound pack lifecycle interactions. */

import { type ReactNode, useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import type {
  PackManagementReview,
  PackReleaseReview,
} from "../../../shared/packManagementProjection";

type PackAction =
  "activate" | "enable" | "rollback" | "disable" | "remove" | "cleanup";
type PackConfirmation = "activate" | "rollback" | "remove";

const ACTION_COMMANDS: Record<PackAction, string> = {
  activate: "activate_pack",
  enable: "enable_pack",
  rollback: "rollback_pack",
  disable: "disable_pack",
  remove: "uninstall_pack",
  cleanup: "retry_pack_cleanup",
};

const ACTION_FAILURES: Record<PackAction, string> = {
  activate: "Pack could not be activated. Refresh and review it again.",
  enable: "Pack could not be enabled. Refresh and try again.",
  rollback: "Pack could not be rolled back. Refresh and review it again.",
  disable: "Pack could not be disabled. Refresh and try again.",
  remove: "Pack files could not be removed. Refresh and try again.",
  cleanup: "Pack cleanup could not be retried. Refresh and try again.",
};

export function PackLifecycleControls({
  pack,
  onChanged,
  renderReleaseReview,
}: {
  pack: PackManagementReview;
  onChanged: () => Promise<void>;
  renderReleaseReview: (release: PackReleaseReview) => ReactNode;
}) {
  const [pendingAction, setPendingAction] = useState<PackAction | null>(null);
  const [confirmation, setConfirmation] = useState<PackConfirmation | null>(
    null,
  );
  const [actionError, setActionError] = useState<string | null>(null);
  const activationRelease = pack.releases.find(
    (release) => release.isHighWater,
  );
  const rollbackRelease = pack.releases.find((release) => release.isRollback);

  const runAction = async (action: PackAction) => {
    setPendingAction(action);
    setActionError(null);
    try {
      const args: Record<string, string | number> = {
        publisherKeyId: pack.publisherKeyId,
        packId: pack.packId,
        expectedGeneration: pack.generation,
      };
      if (action === "activate" && activationRelease) {
        args.releaseSequence = activationRelease.releaseSequence;
      }
      await invoke(ACTION_COMMANDS[action], args);
      setConfirmation(null);
      await onChanged();
    } catch {
      setActionError(ACTION_FAILURES[action]);
    } finally {
      setPendingAction(null);
    }
  };

  return (
    <>
      {actionError ? (
        <p role="alert" className="mt-4 text-sm text-danger">
          {actionError}
        </p>
      ) : null}

      <div className="mt-4 flex flex-wrap gap-2">
        {pack.state === "needs_review" || pack.updateAvailable ? (
          <Button
            size="sm"
            disabled={pendingAction !== null || !activationRelease}
            onClick={() => setConfirmation("activate")}
          >
            {pack.updateAvailable ? "Activate update" : "Activate pack"}
          </Button>
        ) : null}
        {pack.state === "disabled" ? (
          <Button
            size="sm"
            variant="secondary"
            loading={pendingAction === "enable"}
            loadingText="Enabling..."
            disabled={pendingAction !== null}
            onClick={() => void runAction("enable")}
          >
            Enable pack
          </Button>
        ) : null}
        {pack.state === "ready" ? (
          <Button
            size="sm"
            variant="secondary"
            loading={pendingAction === "disable"}
            loadingText="Disabling..."
            disabled={pendingAction !== null}
            onClick={() => void runAction("disable")}
          >
            Disable pack
          </Button>
        ) : null}
        {pack.state === "ready" && rollbackRelease ? (
          <Button
            size="sm"
            variant="secondary"
            disabled={pendingAction !== null}
            onClick={() => setConfirmation("rollback")}
          >
            Roll back pack
          </Button>
        ) : null}
        {pack.state !== "removed" ? (
          <Button
            size="sm"
            variant="danger"
            disabled={pendingAction !== null}
            onClick={() => setConfirmation("remove")}
          >
            Remove pack
          </Button>
        ) : null}
        {pack.cleanupPending ? (
          <Button
            size="sm"
            variant="secondary"
            loading={pendingAction === "cleanup"}
            loadingText="Retrying..."
            disabled={pendingAction !== null}
            onClick={() => void runAction("cleanup")}
          >
            Retry cleanup
          </Button>
        ) : null}
      </div>

      {confirmation === "activate" && activationRelease ? (
        <div
          role="group"
          aria-label="Confirm pack activation"
          className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-3"
        >
          <p className="text-sm text-surface-700 dark:text-surface-200">
            Review the exact signed permissions for release{" "}
            {activationRelease.releaseSequence}
            before making it available to JobSentinel.
          </p>
          <section
            aria-label={`Activation release ${activationRelease.releaseSequence} review`}
          >
            {renderReleaseReview(activationRelease)}
          </section>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={pendingAction !== null}
              onClick={() => setConfirmation(null)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              loading={pendingAction === "activate"}
              loadingText="Activating..."
              disabled={pendingAction !== null}
              onClick={() => void runAction("activate")}
            >
              Confirm activation
            </Button>
          </div>
        </div>
      ) : null}

      {confirmation === "rollback" && rollbackRelease ? (
        <div
          role="group"
          aria-label="Confirm pack rollback"
          className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-3"
        >
          <p className="text-sm text-surface-700 dark:text-surface-200">
            This will replace the active release with retained release{" "}
            {rollbackRelease.releaseSequence}
            after verifying it again.
          </p>
          <section
            aria-label={`Rollback release ${rollbackRelease.releaseSequence} review`}
          >
            {renderReleaseReview(rollbackRelease)}
          </section>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={pendingAction !== null}
              onClick={() => setConfirmation(null)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              loading={pendingAction === "rollback"}
              loadingText="Rolling back..."
              disabled={pendingAction !== null}
              onClick={() => void runAction("rollback")}
            >
              Confirm rollback
            </Button>
          </div>
        </div>
      ) : null}

      {confirmation === "remove" ? (
        <div
          role="group"
          aria-label="Confirm pack removal"
          className="mt-3 rounded-md border border-danger/30 bg-danger/5 p-3"
        >
          <p className="text-sm text-surface-700 dark:text-surface-200">
            Removing this pack disables it and removes its local files. Its
            signed history stays visible to prevent an unsafe downgrade.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="secondary"
              disabled={pendingAction !== null}
              onClick={() => setConfirmation(null)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              variant="danger"
              loading={pendingAction === "remove"}
              loadingText="Removing..."
              disabled={pendingAction !== null}
              onClick={() => void runAction("remove")}
            >
              Remove pack files
            </Button>
          </div>
        </div>
      ) : null}
    </>
  );
}
