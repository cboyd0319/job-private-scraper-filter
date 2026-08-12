import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  resetSettingsLoadTest,
  setupHappyPath,
} from "../SettingsPage.testSupport";
import Settings from "../SettingsPage";

describe("Settings portable recovery", () => {
  beforeEach(resetSettingsLoadTest);

  it("places encrypted backup and recovery in Settings", async () => {
    setupHappyPath();

    render(<Settings onClose={vi.fn()} />);

    expect(
      await screen.findByRole("heading", {
        name: "Encrypted Backup and Recovery",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Create encrypted backup" }),
    ).toBeDisabled();
  });
});
