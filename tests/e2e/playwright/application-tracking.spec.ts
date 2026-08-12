// Verifies application-tracking board, review, status, reminder, and toolbar journeys in supported browsers.

import { test, expect } from "@playwright/test";
import { ApplicationsPage } from "./page-objects/ApplicationsPage";

const STATUS_COLUMNS = [
  ["to_apply", "To Apply"],
  ["applied", "Applied"],
  ["screening_call", "Screening Call"],
  ["phone_interview", "Phone Interview"],
  ["technical_interview", "Skills Interview"],
  ["onsite_interview", "Onsite Interview"],
  ["offer_received", "Offer Received"],
  ["offer_accepted", "Offer Accepted"],
  ["offer_rejected", "Offer Declined"],
  ["rejected", "Not Selected"],
  ["withdrawn", "Withdrawn"],
  ["ghosted", "No Response"],
] as const;

test.describe("Application Tracking", () => {
  let applicationsPage: ApplicationsPage;

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.removeItem("jobsentinel.mockState.v1");
      sessionStorage.clear();
    });
    applicationsPage = new ApplicationsPage(page);
    await applicationsPage.navigateTo();
  });

  test.describe("Kanban Board", () => {
    test("displays the kanban board @smoke", async () => {
      await expect(applicationsPage.kanbanBoard).toBeVisible();
      await expect(pageTitle(applicationsPage)).toBeVisible();
    });

    test("displays every backend status column", async () => {
      await expect(applicationsPage.columns).toHaveCount(STATUS_COLUMNS.length);

      for (const [status, label] of STATUS_COLUMNS) {
        await expect(applicationsPage.getColumn(status)).toBeVisible();
        await expect(applicationsPage.getColumn(status)).toContainText(label);
      }
    });

    test("loads mock applications into canonical columns", async () => {
      await expect(applicationsPage.getColumn("to_apply")).toContainText("SEO Manager");
      await expect(applicationsPage.getColumn("applied")).toContainText("E-Commerce Manager");
      await expect(applicationsPage.getColumn("screening_call")).toContainText("Content Marketing Manager");
    });

    test("shows empty canonical columns as drop targets", async () => {
      await expect(applicationsPage.getColumn("phone_interview")).toContainText("Drop here");
      await expect(applicationsPage.getColumn("offer_received")).toContainText("Drop here");
      await expect(applicationsPage.getColumn("ghosted")).toContainText("Drop here");
    });

    test("shows quick stats from application statuses", async ({ page }) => {
      await expect(page.getByText("Applied:", { exact: true })).toBeVisible();
      await expect(page.getByText("Interviews:", { exact: true })).toBeVisible();
      await expect(page.getByText("Offers:", { exact: true })).toBeVisible();
      await expect(page.getByText("In Progress:", { exact: true })).toBeVisible();
    });

    test("shows next actions in the Daily Mission panel", async ({ page }) => {
      await expect(applicationsPage.reviewPanel).toBeVisible();
      await expect(applicationsPage.reviewPanel).toContainText("Daily mission");
      await expect(applicationsPage.reviewPanel).toContainText("Review follow up");
      await expect(applicationsPage.reviewPanel).toContainText("Prepare or follow up");
      await expect(applicationsPage.reviewPanel).toContainText("Review this tracked role");

      await applicationsPage.reviewPanel
        .getByRole("button", { name: "Review this tracked role" })
        .click();

      await expect(page.getByTestId("application-detail-dialog")).toBeVisible();
    });

    test("keeps the page inside a narrow viewport", async ({ page }) => {
      await page.setViewportSize({ width: 390, height: 844 });
      await expect(applicationsPage.kanbanBoard).toBeVisible();

      const metrics = await page.evaluate(() => {
        return {
          viewportWidth: window.innerWidth,
          documentWidth: document.documentElement.scrollWidth,
          bodyWidth: document.body.scrollWidth,
        };
      });

      expect(Math.max(metrics.documentWidth, metrics.bodyWidth)).toBeLessThanOrEqual(metrics.viewportWidth);
    });
  });

  test.describe("Application Cards", () => {
    test("displays application cards with required fields", async () => {
      await expect(applicationsPage.applicationCards).toHaveCount(3);

      const firstCard = await applicationsPage.getApplicationCard(0);
      await expect(firstCard.position).toContainText("SEO Manager");
      await expect(firstCard.company).toContainText("Shopify");
      await expect(firstCard.status).toContainText("To Apply");
    });

    test("shows not-applied and applied dates correctly", async () => {
      const toApplyCard = applicationsPage.getApplicationCardByText("SEO Manager");
      const appliedCard = applicationsPage.getApplicationCardByText("E-Commerce Manager");

      await expect(toApplyCard.locator("[data-testid='application-date']")).toContainText("Not applied yet");
      await expect(appliedCard.locator("[data-testid='application-date']")).toContainText("Applied:");
    });

    test("opens application detail dialog from a card", async () => {
      await applicationsPage.openApplicationDetails("SEO Manager");

      await expect(applicationsPage.detailDialog).toBeVisible();
      await expect(applicationsPage.detailDialog).toContainText("SEO Manager");
      await expect(applicationsPage.detailDialog).toContainText("Shopify");
      await expect(applicationsPage.statusSelect).toHaveValue("to_apply");
      await expect(applicationsPage.notesTextarea).toBeVisible();
    });

    test("loads existing notes in detail dialog", async () => {
      await applicationsPage.openApplicationDetails("E-Commerce Manager");

      await expect(applicationsPage.detailDialog).toContainText("Previous notes: Applied via website");
      await expect(applicationsPage.notesTextarea).toHaveValue("Applied via website");
    });

    test("does not carry unsaved notes between application detail dialogs", async () => {
      await applicationsPage.openApplicationDetails("SEO Manager");
      await applicationsPage.notesTextarea.fill("QA stale note leak");
      await applicationsPage.closeDialogButton.click();
      await expect(applicationsPage.detailDialog).toBeHidden();

      await applicationsPage.openApplicationDetails("Content Marketing Manager");

      await expect(applicationsPage.notesTextarea).toHaveValue("Recruiter reached out");
    });
  });

  test.describe("Status and Notes", () => {
    test("updates status through the detail modal", async () => {
      await applicationsPage.openApplicationDetails("SEO Manager");
      await applicationsPage.statusSelect.selectOption("applied");

      await expect(applicationsPage.statusSelect).toHaveValue("applied");
      await applicationsPage.closeDialogButton.click();

      await expect.poll(() => applicationsPage.getCardsInColumn("applied")).toBe(2);
      await expect(applicationsPage.getColumn("applied")).toContainText("SEO Manager");
    });

    test("moves a card by drag and drop", async () => {
      const card = applicationsPage.getApplicationCardByText("SEO Manager");
      await card.focus();
      await card.press("Space");
      await expect(card).toHaveAttribute("aria-pressed", "true");
      await card.press("ArrowRight");
      await expect(card.locator("[data-testid='application-status']")).toHaveText("Applied");
      await card.press("Space");
      await expect(card).toHaveAttribute("aria-pressed", "false");
    });

    test("saves notes and shows them on the card", async () => {
      const note = "Follow up after portfolio review";

      await applicationsPage.openApplicationDetails("SEO Manager");
      await applicationsPage.notesTextarea.fill(note);
      await applicationsPage.saveNotesButton.click();

      await expect(applicationsPage.detailDialog).toBeHidden();
      await expect(applicationsPage.getApplicationCardByText("SEO Manager")).toContainText(note);
    });
  });

  test.describe("Reminders and Toolbar", () => {
    test("shows pending reminders with valid dates", async () => {
      await expect(applicationsPage.pendingReminders).toBeVisible();
      await expect(applicationsPage.pendingReminderRows).toHaveCount(1);
      await expect(applicationsPage.pendingReminderRows.first()).toContainText("E-Commerce Manager at Wayfair");
      await expect(applicationsPage.pendingReminderRows.first()).not.toContainText("Invalid");
    });

    test("completes a pending reminder", async () => {
      await applicationsPage.pendingReminderRows.first().getByRole("button", { name: "Done" }).click();

      await expect(applicationsPage.pendingReminderRows).toHaveCount(0);
      await expect(applicationsPage.pendingReminders).toBeHidden();
    });

    test("opens application summary panel", async ({ page }) => {
      await applicationsPage.summaryButton.click();

      await expect(page.getByRole("dialog", { name: "Application Summary" })).toBeVisible();
    });

    test("opens interview scheduler", async ({ page }) => {
      await applicationsPage.interviewsButton.click();

      await expect(page.getByRole("dialog", { name: "Interview Schedule" })).toBeVisible();
    });

    test("opens cover letter templates", async ({ page }) => {
      await applicationsPage.templatesButton.click();

      await expect(page.getByRole("dialog", { name: "Cover Letter Templates" })).toBeVisible();
    });
  });
});

function pageTitle(applicationsPage: ApplicationsPage) {
  return applicationsPage.page.getByRole("heading", { name: "Application Tracker" });
}
