import { test, expect, seedAuth, waitForHydrated, createJobViaForm, uniqueJobName } from "./fixtures";

test.describe("pw-chronon-history", () => {
  test("pw-chronon-history-happy-saver-summary-and-diff", async ({ page }) => {
    await seedAuth(page, "admin");
    await createJobViaForm(page, uniqueJobName("e2e-history"));

    // Revision 2: move the job to another worker pool.
    await page.getByTestId("edit-job-button").locator("button").click();
    const select = page.getByTestId("job-edit-pool").locator("select");
    await expect(select).toBeVisible({ timeout: 60_000 });
    await select.selectOption("e2e-pool-a");
    await page.getByTestId("save-job-button").locator("button").click();
    await expect(page.getByTestId("job-detail-pool")).toHaveText("e2e-pool-a", { timeout: 60_000 });

    await page.getByTestId("job-tabs").getByRole("tab", { name: "History" }).click();
    await expect(page).toHaveURL(/tab=history/, { timeout: 30_000 });
    const history = page.getByTestId("job-tab-history");
    const rows = history.getByTestId("job-history-row");
    await expect(rows).toHaveCount(2, { timeout: 60_000 });

    const current = rows.filter({ hasText: "Revision 2" });
    await expect(current).toContainText("Moved to the e2e-pool-a worker pool");
    await expect(current).toContainText("Ada Admin");
    await expect(current.getByTestId("job-history-current")).toHaveText("Current");
    const first = rows.filter({ hasText: "Revision 1" });
    await expect(first).toContainText("Created the job");
    await expect(first.getByTestId("job-history-current")).toHaveCount(0);
    await expect(history).toContainText("Every save makes a revision. Older revisions can't be edited.");

    const diff = history.getByTestId("job-history-diff-card");
    await expect(history.getByTestId("job-history-diff-title")).toHaveText(
      "Revision 2 compared with revision 1",
    );
    await expect(diff).toContainText("worker pool: general");
    await expect(diff).toContainText("worker pool: e2e-pool-a");
    await expect(diff).toContainText("schedule: Every day at 03:00 UTC (0 3 * * *)");
    await expect(diff).toContainText("script: e2e_echo");
  });

  test("pw-chronon-history-happy-select-revision-without-reload", async ({ page }) => {
    await seedAuth(page, "admin");
    await createJobViaForm(page, uniqueJobName("e2e-history-pick"));
    await page.getByTestId("edit-job-button").locator("button").click();
    await page.getByTestId("job-edit-pool").locator("select").selectOption("e2e-pool-a");
    await page.getByTestId("save-job-button").locator("button").click();
    await expect(page.getByTestId("job-detail-pool")).toHaveText("e2e-pool-a", { timeout: 60_000 });

    await page.getByTestId("job-tabs").getByRole("tab", { name: "History" }).click();
    const history = page.getByTestId("job-tab-history");
    const rows = history.getByTestId("job-history-row");
    await expect(rows).toHaveCount(2, { timeout: 60_000 });

    // Mark the page; a reload or remount would drop the marker.
    await page.evaluate(() => {
      (window as unknown as { __historyMarker: string }).__historyMarker = "kept";
      (document.querySelector('[data-testid="job-history-card"]') as HTMLElement).dataset.e2eMarker = "kept";
    });

    await rows.filter({ hasText: "Revision 1" }).click();
    await expect(history.getByTestId("job-history-diff-title")).toHaveText(
      "Revision 1, as the job was created",
      { timeout: 30_000 },
    );
    await expect(rows.filter({ hasText: "Revision 1" })).toHaveAttribute("data-selected", "true");
    await expect(history.getByTestId("job-history-diff-card")).toContainText("worker pool: general");

    await rows.filter({ hasText: "Revision 2" }).click();
    await expect(history.getByTestId("job-history-diff-title")).toHaveText(
      "Revision 2 compared with revision 1",
    );
    expect(await page.evaluate(() => (window as unknown as { __historyMarker?: string }).__historyMarker)).toBe(
      "kept",
    );
    await expect(history.getByTestId("job-history-card")).toHaveAttribute("data-e2e-marker", "kept");
  });
});
