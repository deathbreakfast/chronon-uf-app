import { test, expect, seedAuth, waitForHydrated, createJobViaForm, uniqueJobName } from "./fixtures";

test.describe("pw-chronon-schedule", () => {
  test("pw-chronon-schedule-happy-edit-preview-save", async ({ page }) => {
    await seedAuth(page, "admin");
    await createJobViaForm(page, uniqueJobName("e2e-purge"));

    await expect(page.getByTestId("job-flash")).toContainText("first runs", { timeout: 60_000 });
    await expect(page.getByTestId("schedule-card-sentence")).toHaveText("Every day at 03:00 UTC", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("schedule-card-cron")).toContainText("0 3 * * *");
    await expect(page.getByTestId("schedule-card-next-runs").locator("tbody tr")).toHaveCount(5);

    await page.getByTestId("edit-schedule-button").getByRole("button").click();
    const dialog = page.getByTestId("schedule-dialog");
    await expect(dialog).toBeVisible({ timeout: 60_000 });
    await dialog.getByTestId("schedule-at").locator("input").fill("04:00");
    await expect(dialog.getByTestId("schedule-sentence")).toHaveText("Every day at 04:00 UTC", {
      timeout: 30_000,
    });
    const preview = dialog.getByTestId("schedule-dialog-preview");
    await expect(preview.locator("thead")).toContainText("Now");
    await expect(preview.locator("thead")).toContainText("After you save");
    await expect(preview.locator("strong[data-changed='true']").first()).toContainText("04:00");
    await expect(page.getByTestId("schedule-dialog-summary")).toContainText(
      "Saving makes revision 2.",
    );

    await page.getByTestId("schedule-dialog-save").getByRole("button").click();
    await expect(page.getByTestId("job-flash")).toContainText("Schedule saved as revision 2.", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("schedule-card-sentence")).toHaveText("Every day at 04:00 UTC");
    await expect(page.getByTestId("schedule-card-next-runs")).toContainText("04:00");

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("schedule-card-sentence")).toHaveText("Every day at 04:00 UTC", {
      timeout: 60_000,
    });
  });

  test("pw-chronon-schedule-sad-hour-out-of-range", async ({ page }) => {
    await seedAuth(page, "admin");
    await createJobViaForm(page, uniqueJobName("e2e-badhour"));

    await page.getByTestId("edit-schedule-button").getByRole("button").click();
    const dialog = page.getByTestId("schedule-dialog");
    await expect(dialog).toBeVisible({ timeout: 60_000 });
    await dialog.getByTestId("cron-expr").locator("input").fill("0 24 * * *");
    await expect(dialog.getByTestId("cron-expr")).toContainText(
      "Hour must be 0 to 23. You entered 24.",
      { timeout: 30_000 },
    );
    await expect(page.getByTestId("schedule-dialog-save").getByRole("button")).toBeDisabled();
  });
});
