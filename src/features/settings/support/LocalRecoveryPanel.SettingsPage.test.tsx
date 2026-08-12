import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import {
  mockInvoke,
  resetSettingsLoadTest,
  setupHappyPath,
} from "../SettingsPage.testSupport";
import Settings from "../SettingsPage";

describe("Settings local recovery isolation", () => {
  beforeEach(resetSettingsLoadTest);

  it("keeps the settings form usable when the recovery report fails", async () => {
    setupHappyPath();
    const happyPath = mockInvoke.getMockImplementation();
    mockInvoke.mockImplementation((command, args) => {
      if (command === "get_local_recovery_report") {
        return Promise.reject(new Error("private raw recovery detail"));
      }
      return happyPath?.(command, args) ?? Promise.resolve(null);
    });

    render(<Settings onClose={vi.fn()} />);

    expect(await screen.findByText("Settings")).toBeInTheDocument();
    expect(
      await screen.findByText(
        "Local recovery status is unavailable. Other settings are still available.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Save Changes" }),
    ).toBeEnabled();
    expect(document.body).not.toHaveTextContent("private raw recovery detail");
  });
});
