import { expect, test, type Page } from "@playwright/test";

const MOCK_INVOKE_CONTROLS_KEY = "jobsentinel.mockInvokeControls.v1";
const MOCK_STATE_KEY = "jobsentinel.mockState.v1";

const VIEWPORTS = [
  { name: "desktop", width: 1440, height: 1000 },
  { name: "mobile", width: 390, height: 844 },
] as const;

type VerificationLog = {
  consoleErrors: string[];
  pageErrors: string[];
};

function watchErrors(page: Page): VerificationLog {
  const log: VerificationLog = { consoleErrors: [], pageErrors: [] };

  page.on("console", (message) => {
    if (message.type() === "error") {
      log.consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    log.pageErrors.push(error.message);
  });

  return log;
}

async function forceFirstRunSetup(page: Page): Promise<void> {
  await page.addInitScript(
    ({ controlsKey, stateKey }) => {
      window.localStorage.removeItem(stateKey);
      window.localStorage.setItem(
        controlsKey,
        JSON.stringify({
          delayMs: 0,
          responses: {
            is_first_run: true,
            get_active_resume: {
              id: 1,
              name: "Local Resume",
            },
            get_user_skills: [
              { skill_name: "Calendar management", source: "resume" },
              { skill_name: "Customer scheduling", source: "resume" },
              { skill_name: "Spreadsheet reporting", source: "resume" },
            ],
          },
        }),
      );
    },
    {
      controlsKey: MOCK_INVOKE_CONTROLS_KEY,
      stateKey: MOCK_STATE_KEY,
    },
  );
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const metrics = await page.evaluate(() => ({
    bodyWidth: document.body.scrollWidth,
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));

  expect(Math.max(metrics.documentWidth, metrics.bodyWidth)).toBeLessThanOrEqual(
    metrics.viewportWidth,
  );
}

async function expectCleanErrors(log: VerificationLog): Promise<void> {
  expect(log.pageErrors).toEqual([]);
  expect(log.consoleErrors).toEqual([]);
}

async function setLocationPreference(
  page: Page,
  name: RegExp,
  checked: boolean,
): Promise<void> {
  const option = page.getByRole("checkbox", { name });
  await expect(option).toBeVisible();

  if ((await option.getAttribute("aria-checked")) !== String(checked)) {
    await option.click();
  }
}

test.describe("Setup Wizard first-run flow", () => {
  for (const viewport of VIEWPORTS) {
    test(`skips setup without saving or starting source work at ${viewport.name} viewport`, async ({ page }) => {
      await page.setViewportSize(viewport);
      await forceFirstRunSetup(page);
      const log = watchErrors(page);

      await page.goto("/", { waitUntil: "domcontentloaded" });

      await expect(page.getByText("Skipping lasts only for this session and saves no search. Setup returns next time. You can still review or import local jobs.")).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await page.getByRole("button", { name: "Skip for now" }).click();
      await expect(page.getByRole("heading", { name: "JobSentinel" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Import Job", exact: true })).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await expectCleanErrors(log);
    });
  }

  for (const viewport of VIEWPORTS) {
    test(`completes reviewed setup actions at ${viewport.name} viewport`, async ({ page }) => {
      await page.setViewportSize(viewport);
      await forceFirstRunSetup(page);
      const log = watchErrors(page);

      await page.goto("/", { waitUntil: "domcontentloaded" });

      await expect(page.getByRole("heading", { name: "Work You Want" })).toBeVisible();
      await expectNoHorizontalOverflow(page);

      await page.getByRole("button", { name: "Build My Search" }).click();
      await expect(page.getByRole("heading", { name: "Job Basics" })).toBeVisible();
      await expect(
        page.getByText("Add at least one job title", { exact: true }),
      ).toBeVisible();
      await expectNoHorizontalOverflow(page);

      await page.getByRole("button", { name: "Check saved resume skills" }).click();
      await expect(page.getByText("Local Resume")).toBeVisible();
      await page.getByRole("button", { name: "Add all visible" }).click();
      await expect(
        page.getByText("Calendar management", { exact: true }).first(),
      ).toBeVisible();

      await page.getByRole("button", { name: "Add Office Assistant job title" }).click();
      await page.getByPlaceholder("Add a skill...").fill("Calendar coordination");
      await page.getByPlaceholder("Add a skill...").press("Enter");
      await page
        .getByPlaceholder("e.g., night shift, heavy travel")
        .fill("weekend work");
      await page
        .getByPlaceholder("e.g., night shift, heavy travel")
        .press("Enter");
      await page.getByRole("radio", { name: /hourly/i }).check();
      await page.getByLabel("Minimum pay").fill("28");
      await expect(page.getByText("Office Assistant")).toBeVisible();
      await expect(
        page.getByText("weekend work", { exact: true }).first(),
      ).toBeVisible();
      await expectNoHorizontalOverflow(page);

      await page.getByRole("button", { name: /^Continue$/ }).click();
      await expect(page.getByRole("heading", { name: "Location" })).toBeVisible();
      await setLocationPreference(page, /Remote:/, false);
      await setLocationPreference(page, /Hybrid:/, false);
      await setLocationPreference(page, /On-site:/, false);
      await expect(page.getByRole("button", { name: /^Continue$/ })).toBeDisabled();
      await page.getByRole("button", { name: "Not sure about location yet" }).click();
      await page.getByRole("button", { name: "Detect location" }).click();
      await page.getByRole("button", { name: "Use This" }).click();
      await expect(
        page.getByText("Denver, CO", { exact: true }).first(),
      ).toBeVisible();
      await page.getByPlaceholder("e.g., Chicago, Austin").fill("Austin, TX");
      await page.getByPlaceholder("e.g., Chicago, Austin").press("Enter");
      await expect(
        page.getByText("Austin, TX", { exact: true }).first(),
      ).toBeVisible();
      await expectNoHorizontalOverflow(page);

      await page.getByRole("button", { name: /^Continue$/ }).click();
      await expect(page.getByRole("heading", { name: "Notifications" })).toBeVisible();
      await page.getByRole("radio", { name: /Widest search/ }).check();
      await page.getByRole("radio", { name: /Broad discovery/ }).check();
      await page.getByRole("checkbox", { name: /Desktop alerts/ }).check();
      await expect(page.getByRole("checkbox", { name: /Quiet job-search mode/ })).toBeEnabled();
      await page.getByRole("checkbox", { name: /Quiet job-search mode/ }).check();

      await expectNoHorizontalOverflow(page);

      await page.getByRole("button", { name: "Start Finding Jobs" }).click();
      await expect(page.getByRole("heading", { name: "JobSentinel" })).toBeVisible();
      await expect(page.getByRole("navigation", { name: "Main navigation" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Import Job", exact: true })).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await expectCleanErrors(log);
    });
  }
});
