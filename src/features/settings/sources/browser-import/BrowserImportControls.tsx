import { Button } from "../../../../ui/Button";

export const MIN_BROWSER_IMPORT_PORT = 1024;
export const MAX_BROWSER_IMPORT_PORT = 65535;

interface BrowserImportControlsProps {
  copiedAction: "import" | "applied" | null;
  enabled: boolean;
  loading: boolean;
  onCopy: () => void;
  onCopyApplied: () => void;
  onPortInputChange: (value: string) => void;
  onSavePort: () => void;
  onTargetUrlChange: (value: string) => void;
  onToggleAdvanced: () => void;
  portChanged: boolean;
  portInput: string;
  portInputError: string | null;
  showAdvanced: boolean;
  targetUrl: string;
  targetUrlError: string | null;
}

export function BrowserImportControls({
  copiedAction,
  enabled,
  loading,
  onCopy,
  onCopyApplied,
  onPortInputChange,
  onSavePort,
  onTargetUrlChange,
  onToggleAdvanced,
  portChanged,
  portInput,
  portInputError,
  showAdvanced,
  targetUrl,
  targetUrlError,
}: BrowserImportControlsProps) {
  return (
    <>
      <div>
        <button
          type="button"
          onClick={onToggleAdvanced}
          className="text-sm text-gray-400 hover:text-white underline"
          aria-expanded={showAdvanced}
        >
          Optional browser button setting
        </button>
        {showAdvanced && (
          <div className="mt-3 rounded-lg border border-gray-700 p-4">
            <label
              htmlFor="bookmarklet-port"
              className="block text-sm font-medium text-gray-300 mb-2"
            >
              Button setup number
            </label>
            <div className="flex flex-wrap items-start gap-3">
              <input
                id="bookmarklet-port"
                type="number"
                value={portInput}
                onChange={(event) => onPortInputChange(event.target.value)}
                disabled={enabled || loading}
                aria-invalid={Boolean(portInputError)}
                aria-describedby={
                  portInputError
                    ? "bookmarklet-port-error"
                    : "bookmarklet-port-help"
                }
                className="w-32 px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                min={MIN_BROWSER_IMPORT_PORT}
                max={MAX_BROWSER_IMPORT_PORT}
              />
              <Button
                onClick={onSavePort}
                size="sm"
                variant="secondary"
                disabled={
                  enabled || loading || !portChanged || Boolean(portInputError)
                }
              >
                Save Number
              </Button>
            </div>
            {portInputError && (
              <p
                id="bookmarklet-port-error"
                className="text-xs text-red-400 mt-1"
              >
                {portInputError}
              </p>
            )}
            <p
              id="bookmarklet-port-help"
              className="text-xs text-gray-500 mt-1"
            >
              Leave this number unchanged unless JobSentinel help instructions
              tell you otherwise.
            </p>
          </div>
        )}
      </div>

      <div className="border-t border-gray-700 pt-4">
        <h4 className="text-sm font-medium text-gray-300 mb-3">
          Set Up Browser Button
        </h4>
        <div className="space-y-3">
          <div className="bg-gray-800/50 rounded-lg p-4">
            <label
              htmlFor="browser-import-target"
              className="mb-2 block text-sm font-medium text-gray-300"
            >
              Job page address
            </label>
            <input
              id="browser-import-target"
              type="url"
              value={targetUrl}
              onChange={(event) => onTargetUrlChange(event.target.value)}
              disabled={!enabled || loading}
              placeholder="https://company.example/jobs/123"
              autoComplete="url"
              aria-invalid={Boolean(targetUrlError)}
              aria-describedby={
                targetUrlError
                  ? "browser-import-target-error"
                  : "browser-import-target-help"
              }
              className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
            />
            {targetUrlError ? (
              <p
                id="browser-import-target-error"
                className="mt-1 text-xs text-red-400"
              >
                {targetUrlError}
              </p>
            ) : (
              <p
                id="browser-import-target-help"
                className="mt-1 text-xs text-gray-500"
              >
                Enter the public https page you plan to import. JobSentinel
                confirms the site in a native dialog before copying.
              </p>
            )}
            <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
              <span className="text-xs text-gray-400">Browser action</span>
              <div className="flex flex-wrap gap-2">
                <Button
                  onClick={onCopyApplied}
                  size="sm"
                  variant="secondary"
                  disabled={
                    !enabled ||
                    loading ||
                    !targetUrl.trim() ||
                    Boolean(targetUrlError)
                  }
                >
                  {copiedAction === "applied"
                    ? "Copied!"
                    : "Copy I Just Applied Button"}
                </Button>
                <Button
                  onClick={onCopy}
                  size="sm"
                  variant="ghost"
                  disabled={
                    !enabled ||
                    loading ||
                    !targetUrl.trim() ||
                    Boolean(targetUrlError)
                  }
                >
                  {copiedAction === "import"
                    ? "Copied!"
                    : "Copy Browser Button"}
                </Button>
              </div>
            </div>
            <p className="text-xs text-gray-500">
              Each button works once for the confirmed site and expires after
              about ten minutes. The applied button creates a local review draft
              and marks details it could not read.
            </p>
          </div>

          <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4">
            <h5 className="text-sm font-medium text-blue-400 mb-2">
              Choose How to Save Jobs:
            </h5>
            <p className="text-sm text-gray-300 mb-3">
              Recommended: use the browser button on an individual job page. It
              adds the visible posting details you choose to a review list in
              JobSentinel. Paste a link only when the browser button cannot read
              the page.
            </p>
            <ol className="text-sm text-gray-300 space-y-2 list-decimal list-inside">
              <li>Turn on Browser Import above</li>
              <li>Enter the individual job page address above</li>
              <li>Copy the browser button and confirm the site</li>
              <li>Use your browser's Add Bookmark option</li>
              <li>Name it "Import to JobSentinel"</li>
              <li>Paste the copied text into the bookmark link field</li>
              <li>Save the bookmark to your bookmarks bar</li>
              <li>Copy a fresh browser button for every import</li>
            </ol>
          </div>

          <div className="bg-green-500/10 border border-green-500/20 rounded-lg p-4">
            <h5 className="text-sm font-medium text-green-400 mb-2">
              How to Use:
            </h5>
            <ol className="text-sm text-gray-300 space-y-2 list-decimal list-inside">
              <li>Open an individual job page.</li>
              <li>
                Use the saved "Import to JobSentinel" item in your bookmarks
                bar.
              </li>
              <li>
                JobSentinel adds the current posting details it can read to your
                review list
              </li>
              <li>Return here, check the details, then save the job.</li>
            </ol>
          </div>

          <div className="bg-yellow-500/10 border border-yellow-500/20 rounded-lg p-4">
            <h5 className="text-sm font-medium text-yellow-400 mb-2">
              Where It Works Best:
            </h5>
            <div className="grid grid-cols-2 gap-2 text-sm text-gray-300">
              <div>
                <p className="font-medium">Official sources:</p>
                <ul className="list-disc list-inside text-xs space-y-1 mt-1">
                  <li>Company career pages</li>
                  <li>Company application pages</li>
                  <li>Public pages with full job details</li>
                </ul>
              </div>
              <div>
                <p className="font-medium">Best page shape:</p>
                <ul className="list-disc list-inside text-xs space-y-1 mt-1">
                  <li>Career pages with job details on the page</li>
                  <li>Pages opened by you in your browser</li>
                </ul>
              </div>
            </div>
            <p className="text-xs text-gray-400 mt-2">
              Some large job boards do not let JobSentinel read saved pages.
              JobSentinel respects those controls. If a job appears with missing
              details, edit it after saving or use JobSentinel's search link for
              that site.
            </p>
          </div>
        </div>
      </div>
    </>
  );
}
