// Verifies settings navigation, persistence, reviewed source controls, and recovery journeys in supported browsers.

import { test, expect } from "@playwright/test";
import { SettingsPage } from "./page-objects/SettingsPage";

test.describe("Settings Save and Load", () => {
  let settingsPage: SettingsPage;

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      if (!sessionStorage.getItem("settings-e2e-reset")) {
        localStorage.removeItem("jobsentinel.mockState.v1");
        sessionStorage.setItem("settings-e2e-reset", "true");
      }
    });
    settingsPage = new SettingsPage(page);
    await settingsPage.navigateTo();
  });

  test.describe("Navigation", () => {
    test("opens settings modal on the dashboard @smoke", async () => {
      await expect(settingsPage.dialog).toBeVisible();
      await expect(settingsPage.basicTab).toHaveAttribute("aria-selected", "true");
      await expect(settingsPage.saveButton).toBeVisible();
    });

    test("switches between basic and advanced tabs", async () => {
      await settingsPage.switchTab("advanced");

      await expect(settingsPage.advancedTab).toHaveAttribute("aria-selected", "true");
      await expect(settingsPage.dialog).toContainText("Get Notified");

      await settingsPage.switchTab("basic");

      await expect(settingsPage.basicTab).toHaveAttribute("aria-selected", "true");
      await expect(settingsPage.dialog).toContainText("Job Titles You Want");
    });

    test("closes settings from the header close control", async () => {
      await settingsPage.closeButton.click();
      await expect(settingsPage.dialog).toBeHidden();
    });
  });

  test.describe("Search Preferences", () => {
    test("loads core job-search defaults", async () => {
      await expect(settingsPage.dialog).toContainText("SEO Manager");
      await expect(settingsPage.dialog).toContainText("Google Analytics");
      await expect(settingsPage.minimumSalaryInput).toHaveValue("80000");
      await expect(settingsPage.remoteCheckbox).toBeChecked();
      await expect(settingsPage.hybridCheckbox).toBeChecked();
      await expect(settingsPage.onsiteCheckbox).toBeChecked();
    });

    test("adds and removes a desired job title", async () => {
      await settingsPage.addListValue("Job Titles You Want", "Principal Product Manager");

      await expect(settingsPage.dialog).toContainText("Principal Product Manager");

      await settingsPage.removeBadge("Principal Product Manager");

      await expect(settingsPage.dialog).not.toContainText("Principal Product Manager");
    });

    test("updates work-style preferences", async () => {
      await settingsPage.setCheckbox(settingsPage.remoteCheckbox, false);
      await settingsPage.setCheckbox(settingsPage.hybridCheckbox, true);
      await settingsPage.setCheckbox(settingsPage.onsiteCheckbox, false);

      await expect(settingsPage.remoteCheckbox).not.toBeChecked();
      await expect(settingsPage.hybridCheckbox).toBeChecked();
      await expect(settingsPage.onsiteCheckbox).not.toBeChecked();
    });

    test("updates salary preferences", async () => {
      await settingsPage.minimumSalaryInput.fill("95000");
      await settingsPage.targetSalaryInput.fill("145000");

      await expect(settingsPage.minimumSalaryInput).toHaveValue("95000");
      await expect(settingsPage.targetSalaryInput).toHaveValue("145000");
    });

    test("persists saved basic settings after reopening", async () => {
      await settingsPage.addListValue("Job Titles You Want", "Staff Designer");
      await settingsPage.minimumSalaryInput.fill("99000");
      await settingsPage.setCheckbox(settingsPage.remoteCheckbox, false);

      await settingsPage.saveSettings();
      await settingsPage.navigateTo();

      await expect(settingsPage.dialog).toContainText("Staff Designer");
      await expect(settingsPage.minimumSalaryInput).toHaveValue("99000");
      await expect(settingsPage.remoteCheckbox).not.toBeChecked();
    });
  });

  test.describe("Sources & Alerts", () => {
    test.beforeEach(async () => {
      await settingsPage.switchTab("advanced");
    });

    test("loads notification, source, and backup controls", async () => {
      await expect(settingsPage.dialog).toContainText("Slack Notifications");
      await expect(settingsPage.dialog).toContainText("Email Alerts");
      await expect(settingsPage.dialog).toContainText("Desktop Notifications");
      await expect(settingsPage.dialog).toContainText("Posting Risk and Freshness");
      await expect(settingsPage.backupButton).toBeVisible();
      await expect(settingsPage.restoreButton).toBeVisible();
      await expect(settingsPage.feedbackButton).toBeVisible();
    });

    test("records LinkedIn Workbench actions from user-reviewed details", async ({ page }) => {
      const logAppliedButton = settingsPage.dialog.getByRole("button", {
        name: "Log applied",
      });

      await expect(settingsPage.dialog).toContainText("LinkedIn Workbench");
      await expect(logAppliedButton).toBeDisabled();

      const reviewCheckbox = settingsPage.dialog.getByLabel(
        /I understand\. Remember this on this computer/i,
      );
      await expect(reviewCheckbox).toBeEnabled();
      await reviewCheckbox.click();

      await expect(logAppliedButton).toBeEnabled();

      await settingsPage.dialog
        .getByRole("button", { name: "Open LinkedIn Jobs" })
        .click();
      await expect(
        settingsPage.dialog.getByText(/JobSentinel will not force this session closed/i),
      ).toBeVisible();

      await settingsPage.dialog
        .getByRole("button", { name: "Open applied jobs" })
        .click();

      await settingsPage.dialog.getByLabel(/Paste selected job text/i).fill(
        [
          "Principal Security Engineer at Example Co",
          "https://www.linkedin.com/jobs/view/123?token=secret",
        ].join("\n"),
      );
      await settingsPage.dialog
        .getByRole("button", { name: "Use pasted details" })
        .click();

      await expect(
        settingsPage.dialog.getByRole("textbox", {
          name: "Job title",
          exact: true,
        }),
      ).toHaveValue("Principal Security Engineer");
      await expect(
        settingsPage.dialog.getByRole("textbox", { name: "Company", exact: true }),
      ).toHaveValue("Example Co");
      await expect(
        settingsPage.dialog.getByRole("textbox", { name: "Job link", exact: true }),
      ).toHaveValue("https://www.linkedin.com/jobs/view/123");

      await logAppliedButton.click();
      await expect(page.getByText("Application logged")).toBeVisible();

      await settingsPage.dialog.getByRole("button", { name: "Save job" }).click();
      await expect(page.getByText("Job saved")).toBeVisible();

      await settingsPage.dialog.getByRole("button", { name: "Track job" }).click();
      await expect(page.getByText("Job tracked")).toBeVisible();

      await settingsPage.dialog.getByRole("button", { name: "Add note" }).click();
      await expect(page.getByText("Note saved")).toBeVisible();

      await settingsPage.dialog.getByRole("button", { name: "Not interested" }).click();
      await expect(page.getByText("Feedback saved")).toBeVisible();
    });

    test("saves a safe support report from settings @smoke", async ({ page }) => {
      await settingsPage.saveSafeSupportReportButton.click();

      await expect(page.getByText("Support report saved for review")).toBeVisible();
      await expect(
        page.getByText(
          /Review jobsentinel-feedback-\d{4}-\d{2}-\d{2}-\d{4}\.txt before sharing it/,
        ),
      ).toBeVisible();
    });

    test("toggles email alerts and validates email fields", async () => {
      await settingsPage.toggleEmailAlerts();

      await expect(
        settingsPage.dialog.getByText("Only if your email service gave you these details"),
      ).toBeVisible();

      await settingsPage.fromEmailInput.fill("invalid-email");

      await expect(settingsPage.dialog).toContainText("Use an email address like user@example.com.");

      await settingsPage.fromEmailInput.fill("sender@example.com");
      await settingsPage.toEmailInput.fill("alerts@example.com");

      await expect(settingsPage.dialog).not.toContainText("Please enter a valid email address");
    });

    test("stores Slack connection link status after saving", async () => {
      await settingsPage.slackWebhookInput.fill("https://hooks.slack.com/services/T000/B000/secret");

      await settingsPage.saveSettings();
      await settingsPage.navigateTo();
      await settingsPage.switchTab("advanced");

      await expect(settingsPage.section("Slack Notifications")).toContainText(
        /Saved securely|Enter new Slack connection link/i,
      );
    });

    test("updates ghost detection settings", async () => {
      const ghostSection = settingsPage.section("Posting Risk and Freshness");
      await ghostSection.getByRole("button", { name: /Custom/ }).click();
      const staleThresholdInput = ghostSection.locator("input[type='number']").first();

      await staleThresholdInput.fill("45");
      await ghostSection.getByRole("button", { name: "Save Settings" }).click();

      await expect(settingsPage.page.getByText("Posting risk settings saved")).toBeVisible();
      await expect(staleThresholdInput).toHaveValue("45");
    });

    test("resets ghost detection settings to defaults", async () => {
      const ghostSection = settingsPage.section("Posting Risk and Freshness");
      await ghostSection.getByRole("button", { name: /Custom/ }).click();
      const staleThresholdInput = ghostSection.locator("input[type='number']").first();

      await staleThresholdInput.fill("45");
      await ghostSection.getByRole("button", { name: "Reset to Defaults" }).click();

      await expect(staleThresholdInput).toHaveValue("60");
    });
  });

  test.describe("Recovery", () => {
    test("reloads defaults after mock storage is cleared", async ({ page }) => {
      await page.evaluate(() => {
        localStorage.removeItem("jobsentinel.mockState.v1");
        sessionStorage.clear();
      });

      await settingsPage.navigateTo();

      await expect(settingsPage.dialog).toBeVisible();
      await expect(settingsPage.minimumSalaryInput).toHaveValue("80000");
    });
  });
});
