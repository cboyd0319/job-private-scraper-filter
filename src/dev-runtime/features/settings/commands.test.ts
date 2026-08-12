import { describe, expect, it } from "vitest";
import { mockConfig } from "../../mocks/data";
import { getDefaultGhostConfig } from "../../mocks/handlers/commandHelpers";
import { handleMockSettingsCommand } from "./commands";

function createState() {
  return {
    config: { ...mockConfig },
    credentials: {},
    credentialUnlock: { mode: "system" as const, configured: false, unlocked: true },
    ghostConfig: getDefaultGhostConfig(),
    bookmarkletConfig: { port: 4321, enabled: false },
    pendingBookmarkletImports: [],
  };
}

describe("Settings mock commands", () => {
  it("does not count a legacy JobsWithGPT approval as an enabled source", () => {
    const state = createState();
    const endpoint = "https://api.jobswithgpt.example/mcp";
    state.config = {
      ...state.config,
      title_allowlist: ["Case Manager"],
      jobswithgpt_endpoint: endpoint,
      jobswithgpt_approval: {
        enabled: true,
        approved_at: "2026-07-19T00:00:00Z",
        payload: {
          endpoint,
          titles: ["Case Manager"],
          location: null,
          remote_only: false,
          limit: 100,
        },
      },
    };

    expect(
      handleMockSettingsCommand("get_dashboard_preferences", undefined, state).value,
    ).toMatchObject({ anyJobSourceEnabled: false });
  });

  it("starts Browser Import with the requested available port", () => {
    const result = handleMockSettingsCommand(
      "start_bookmarklet_server",
      { port: 4321 },
      createState(),
    );

    expect(result).toMatchObject({
      handled: true,
      shouldSave: true,
      value: { port: 4321, enabled: true },
      state: { bookmarkletConfig: { port: 4321, enabled: true } },
    });
  });

  it("rejects commands owned by another feature", () => {
    const state = createState();
    expect(handleMockSettingsCommand("complete_setup", undefined, state)).toMatchObject({
      handled: false,
      shouldSave: false,
      state,
    });
  });
});
