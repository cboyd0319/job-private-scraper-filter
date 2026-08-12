/** Owns generation-bound pack disable, removal, and cleanup interactions. */

import { useState } from "react";
import { invoke } from "../../../platform/tauri";
import { Button } from "../../../ui/Button";
import type { PackManagementReview } from "./packManagementProjection";

type PackAction = "disable" | "remove" | "cleanup";

const ACTION_COMMANDS: Record<PackAction, string> = {
  disable: "disable_pack",
  remove: "uninstall_pack",
  cleanup: "retry_pack_cleanup",
};

const ACTION_FAILURES: Record<PackAction, string> = {
  disable: "Pack could not be disabled. Refresh and try again.",
  remove: "Pack files could not be removed. Refresh and try again.",
  cleanup: "Pack cleanup could not be retried. Refresh and try again.",
};

export function PackLifecycleControls({
  pack,
  onChanged,
}: {
  pack: PackManagementReview;
  onChanged: () => Promise<void>;
}) {
  const [pendingAction, setPendingAction] = useState<PackAction | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const runAction = async (action: PackAction) => {
    setPendingAction(action);
    setActionError(null);
    try {
      await invoke(ACTION_COMMANDS[action], {
        publisherKeyId: pack.publisherKeyId,
        packId: pack.packId,
        expectedGeneration: pack.generation,
      });
      setConfirmRemove(false);
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
        {pack.state !== "removed" ? (
          <Button
            size="sm"
            variant="danger"
            disabled={pendingAction !== null}
            onClick={() => setConfirmRemove(true)}
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

      {confirmRemove ? (
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
              onClick={() => setConfirmRemove(false)}
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
