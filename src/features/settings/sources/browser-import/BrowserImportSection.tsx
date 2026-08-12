import { useState, useEffect, useCallback } from "react";
import { invoke } from "../../../../platform/tauri";
import { Button } from "../../../../ui/Button";
import { Card } from "../../../../ui/Card";
import { Badge } from "../../../../ui/Badge";
import { HelpIcon } from "../../../../ui/HelpIcon";
import { recordBrowserAssistLearningSignalIfEnabled } from "../../../../shared/browserAssistLearning";
import { logError } from "../../../../shared/errorReporting/logger";
import {
  BrowserImportControls,
  MAX_BROWSER_IMPORT_PORT,
  MIN_BROWSER_IMPORT_PORT,
} from "./BrowserImportControls";
import {
  type BookmarkletConfig,
  type BookmarkletImportConfirmResult,
  DEFAULT_BOOKMARKLET_CONFIG,
  type DiscardBookmarkletImportsResponse,
  isBookmarkletConfig,
  isPendingBookmarkletImportList,
  type PendingBookmarkletImport,
  pendingActionKey,
} from "./browserImportModel";
import { BrowserImportReviewList } from "./BrowserImportReviewList";

export function BrowserImportSection() {
  const [config, setConfig] = useState<BookmarkletConfig>(
    DEFAULT_BOOKMARKLET_CONFIG,
  );
  const [portInput, setPortInput] = useState(
    String(DEFAULT_BOOKMARKLET_CONFIG.port),
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connectionNotice, setConnectionNotice] = useState<string | null>(null);
  const [copiedAction, setCopiedAction] = useState<"import" | "applied" | null>(
    null,
  );
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [targetUrl, setTargetUrl] = useState("");
  const [pendingImports, setPendingImports] = useState<
    PendingBookmarkletImport[]
  >([]);
  const [pendingLoading, setPendingLoading] = useState(false);
  const [pendingMessage, setPendingMessage] = useState<string | null>(null);
  const [pendingError, setPendingError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<string | null>(null);

  const parsedPort = Number(portInput);
  const portChanged = portInput !== String(config.port);
  const portInputError =
    portChanged &&
    (!Number.isInteger(parsedPort) ||
      parsedPort < MIN_BROWSER_IMPORT_PORT ||
      parsedPort > MAX_BROWSER_IMPORT_PORT)
      ? `Use a number from ${MIN_BROWSER_IMPORT_PORT} to ${MAX_BROWSER_IMPORT_PORT}.`
      : null;
  let targetUrlError: string | null = null;
  if (targetUrl.trim()) {
    try {
      if (new URL(targetUrl).protocol !== "https:") {
        targetUrlError = "Enter a public https job page address.";
      }
    } catch {
      targetUrlError = "Enter a public https job page address.";
    }
  }

  const loadPendingImports = useCallback(async () => {
    try {
      setPendingLoading(true);
      const result = await invoke<unknown>("get_pending_bookmarklet_imports");
      setPendingImports(isPendingBookmarkletImportList(result) ? result : []);
      setPendingError(null);
    } catch (err) {
      logError("Failed to load pending browser imports:", err);
      setPendingError(
        "Could not load jobs waiting for review. Click Refresh Review List or reopen Settings.",
      );
    } finally {
      setPendingLoading(false);
    }
  }, []);

  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const cfg = await invoke<BookmarkletConfig>("get_bookmarklet_config");
      const nextConfig = isBookmarkletConfig(cfg)
        ? cfg
        : DEFAULT_BOOKMARKLET_CONFIG;
      setConfig(nextConfig);
      setPortInput(String(nextConfig.port));
      setError(null);
    } catch (err) {
      logError("Failed to load bookmarklet config:", err);
      setError(
        "Browser Import could not load. Close and reopen Settings. If this keeps happening, copy or save a safe support report from Settings.",
      );
    } finally {
      setHasLoaded(true);
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (!hasLoaded) {
      return;
    }

    void loadPendingImports();

    if (!config.enabled) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void loadPendingImports();
    }, 5000);

    return () => window.clearInterval(intervalId);
  }, [config.enabled, hasLoaded, loadPendingImports]);

  const toggleServer = async () => {
    try {
      setLoading(true);
      if (config.enabled) {
        await invoke("stop_bookmarklet_server");
        setConfig({ ...config, enabled: false });
      } else {
        const startedConfig = await invoke<BookmarkletConfig>(
          "start_bookmarklet_server",
          {
            port: config.port,
          },
        );
        setConfig(startedConfig);
        setPortInput(String(startedConfig.port));
        setConnectionNotice(
          startedConfig.port === config.port
            ? null
            : "Browser Import found an available setup number because the saved one was in use. Copy a fresh browser button before importing.",
        );
      }
      setError(null);
    } catch (err) {
      logError("Failed to toggle bookmarklet server:", err);
      setError(
        "Browser Import could not be changed. Try again. If this keeps happening, copy or save a safe support report from Settings.",
      );
    } finally {
      setLoading(false);
    }
  };

  const updatePort = async () => {
    if (portInputError || !portChanged) {
      return;
    }

    try {
      setLoading(true);
      await invoke("set_bookmarklet_port", { port: parsedPort });
      setConfig({ ...config, port: parsedPort });
      setPortInput(String(parsedPort));
      setError(null);
    } catch (err) {
      logError("Failed to update bookmarklet port:", err);
      setError(
        "That browser button number could not be saved. Leave it unchanged unless JobSentinel help instructions give you a different number.",
      );
    } finally {
      setLoading(false);
    }
  };

  const copyBookmarklet = async (applied = false) => {
    if (!targetUrl.trim() || targetUrlError) {
      setError("Enter a public https job page address before copying.");
      return;
    }

    try {
      const approved = await invoke<boolean>(
        applied ? "copy_applied_bookmarklet_code" : "copy_bookmarklet_code",
        { targetUrl: targetUrl.trim() },
      );
      if (!approved) {
        setCopiedAction(null);
        setError(null);
        return;
      }
      setCopiedAction(applied ? "applied" : "import");
      setError(null);
      setConnectionNotice(null);
      setTimeout(() => setCopiedAction(null), 2000);
    } catch (err) {
      logError("Could not copy browser import button:", err);
      setCopiedAction(null);
      setError(
        "Could not copy Browser Button. Allow clipboard access, then click Copy Browser Button again. If this keeps happening, copy or save a safe support report from Settings.",
      );
    }
  };

  const savePendingImports = async (ids: string[]) => {
    if (ids.length === 0) {
      return;
    }

    const actionKey = pendingActionKey("save", ids);
    try {
      setPendingAction(actionKey);
      setPendingError(null);
      const selectedImports = pendingImports.filter((item) =>
        ids.includes(item.id),
      );
      const result = await invoke<BookmarkletImportConfirmResult>(
        "confirm_pending_bookmarklet_imports",
        { ids },
      );
      if (result.imported + result.skipped > 0) {
        for (const item of selectedImports) {
          recordBrowserAssistLearningSignalIfEnabled({
            source: "browser-import",
            action:
              item.operation === "applied_logging" ? "applied" : "import_saved",
            title: item.title,
            company: item.company,
            recordedAt: new Date().toISOString(),
          });
        }
      }
      setPendingImports((current) =>
        current.filter((item) => !ids.includes(item.id)),
      );
      const savedLabel =
        selectedImports.length === 1 &&
        selectedImports[0]?.operation === "applied_logging"
          ? "applied draft"
          : result.imported === 1
            ? "browser import"
            : "browser imports";
      const skippedCopy =
        result.skipped > 0 ? ` ${result.skipped} already existed.` : "";
      setPendingMessage(
        `Saved ${result.imported} ${savedLabel}.${skippedCopy}`,
      );
    } catch (err) {
      logError("Could not save pending browser imports:", err);
      setPendingError(
        "Could not save this browser import. Try again, or copy a safe support report from Settings.",
      );
    } finally {
      setPendingAction(null);
    }
  };

  const skipPendingImports = async (ids: string[]) => {
    if (ids.length === 0) {
      return;
    }

    const actionKey = pendingActionKey("skip", ids);
    try {
      setPendingAction(actionKey);
      setPendingError(null);
      const result = await invoke<DiscardBookmarkletImportsResponse>(
        "discard_pending_bookmarklet_imports",
        { ids },
      );
      setPendingImports((current) =>
        current.filter((item) => !ids.includes(item.id)),
      );
      const skippedLabel =
        result.discarded === 1 ? "browser import" : "browser imports";
      setPendingMessage(`Skipped ${result.discarded} ${skippedLabel}.`);
    } catch (err) {
      logError("Could not skip pending browser imports:", err);
      setPendingError(
        "Could not skip this browser import. Try again, or copy a safe support report from Settings.",
      );
    } finally {
      setPendingAction(null);
    }
  };

  if (loading && !hasLoaded) {
    return (
      <Card className="p-6" aria-busy="true">
        <div className="text-center text-gray-400">
          Loading Browser Import...
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-lg font-semibold text-white mb-2">
            Install Browser Button
          </h3>
          <p className="text-sm text-gray-400">
            Review jobs found from pages you choose, then save them locally
          </p>
        </div>
        <HelpIcon
          text="The browser import button finds job details on the page you are viewing and sends them to a local review list. You choose what to save. All processing stays on your computer."
          position="left"
        />
      </div>

      {error && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-4">
          <p className="text-sm text-red-400">{error}</p>
        </div>
      )}

      {connectionNotice && (
        <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4">
          <p role="status" className="text-sm text-blue-200">
            {connectionNotice}
          </p>
        </div>
      )}

      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="flex-1">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Browser Import
            </label>
            <div className="flex items-center gap-3">
              <Badge variant={config.enabled ? "success" : "surface"}>
                {config.enabled ? "On" : "Off"}
              </Badge>
              <Button
                onClick={toggleServer}
                disabled={loading}
                variant={config.enabled ? "danger" : "primary"}
                size="sm"
              >
                {config.enabled ? "Turn Off" : "Turn On"}
              </Button>
            </div>
          </div>
        </div>

        <BrowserImportReviewList
          action={pendingAction}
          error={pendingError}
          imports={pendingImports}
          loading={pendingLoading}
          message={pendingMessage}
          onRefresh={loadPendingImports}
          onSave={savePendingImports}
          onSkip={skipPendingImports}
        />

        <BrowserImportControls
          copiedAction={copiedAction}
          enabled={config.enabled}
          loading={loading}
          onCopy={() => void copyBookmarklet()}
          onCopyApplied={() => void copyBookmarklet(true)}
          onPortInputChange={setPortInput}
          onSavePort={updatePort}
          onTargetUrlChange={setTargetUrl}
          onToggleAdvanced={() => setShowAdvanced((current) => !current)}
          portChanged={portChanged}
          portInput={portInput}
          portInputError={portInputError}
          showAdvanced={showAdvanced}
          targetUrl={targetUrl}
          targetUrlError={targetUrlError}
        />
      </div>
    </Card>
  );
}
