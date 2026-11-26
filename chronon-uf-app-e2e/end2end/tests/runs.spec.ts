import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-chronon-runs", () => {
  test("pw-chronon-runs-happy-list-detail", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/runs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-runs-page")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-runs-data-table")).toBeVisible({ timeout: 60_000 });
    await page.goto(`/chronon/runs/${encodeURIComponent(seeded.fixtures.run_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-run-detail")).toBeVisible({ timeout: 60_000 });
  });

  test("pw-chronon-runs-happy-retries-fold-into-one-row", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/runs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("chronon-runs-data-table");
    const purgeRows = table.getByRole("row").filter({ hasText: seeded.fixtures.purge_job_name });
    // The failed run and its three retries are one Failed row with 4 of 4 attempts.
    const failedRow = purgeRows.filter({ hasText: "4 of 4" });
    await expect(failedRow).toHaveCount(1, { timeout: 60_000 });
    await expect(failedRow).toContainText("Failed");
    await expect(failedRow).toContainText("Schedule");
    await expect(page.getByTestId("chronon-runs-caption")).toContainText(
      `${seeded.fixtures.purge_job_name} run was retried 3 times`,
      { timeout: 60_000 },
    );
  });

  test("pw-chronon-runs-happy-started-by-person", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/runs?time=any", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("chronon-runs-data-table");
    // The nightly fixture run was started by hand by admin, shown by display name.
    await expect(
      table.getByRole("row").filter({ hasText: seeded.fixtures.job_name }).filter({ hasText: "Ada Admin (Run now)" }).first(),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("pw-chronon-runs-happy-filter-without-reload", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/runs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("chronon-runs-data-table");
    await expect(table.getByText(seeded.fixtures.purge_job_name).first()).toBeVisible({ timeout: 60_000 });
    // Mark the page; a reload or remount would drop the marker.
    await page.evaluate(() => {
      (document.getElementById("chronon-runs-page") as HTMLElement).dataset.e2eMarker = "kept";
    });
    await page.getByTestId("chronon-runs-filter-result").locator("select").selectOption("worked");
    await expect(page).toHaveURL(/result=worked/, { timeout: 30_000 });
    await expect(table.getByRole("row").filter({ hasText: "4 of 4" })).toHaveCount(0, { timeout: 60_000 });
    await page.getByTestId("chronon-runs-filter-result").locator("select").selectOption("failed");
    await expect(table.getByRole("row").filter({ hasText: "4 of 4" })).toHaveCount(1, { timeout: 60_000 });
    await expect(page.locator("#chronon-runs-page")).toHaveAttribute("data-e2e-marker", "kept");
  });

  test("pw-chronon-runs-happy-job-card", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.purge_job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const card = page.getByTestId("chronon-recent-runs-card");
    await expect(card).toBeVisible({ timeout: 60_000 });
    await expect(card.getByTestId("chronon-job-week-day")).toHaveCount(7, { timeout: 60_000 });
    await expect(card.locator('[data-testid="chronon-job-week-day"][data-result="failed"]')).toHaveCount(1);
    await expect(card.getByTestId("chronon-job-runs-summary")).toContainText(
      "after 4 attempts. Before that it worked 5 days in a row.",
      { timeout: 60_000 },
    );
    await expect(card.getByRole("row").filter({ hasText: "4 of 4" })).toHaveCount(1, { timeout: 60_000 });
    await expect(page.getByTestId("chronon-job-failing-banner")).toContainText("run failed", {
      timeout: 60_000,
    });
  });
});
