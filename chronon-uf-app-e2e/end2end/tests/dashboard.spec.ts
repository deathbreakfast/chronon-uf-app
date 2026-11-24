import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-chronon-dashboard", () => {
  test("pw-chronon-dashboard-happy-kpis", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-dashboard")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-stat-total-jobs")).toBeVisible();
    await expect(page.locator("#chronon-dashboard-recent-runs").getByText(seeded.fixtures.job_name)).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-chronon-dashboard-happy-retries-count-once", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    // The seeded purge failed on its first try and three retries: one failed run, not four.
    await expect(page.getByTestId("chronon-stat-failed")).toContainText(/(^|\D)1(\D|$)/, {
      timeout: 60_000,
    });
  });

  test("pw-chronon-dashboard-sad-empty-trend-not-crash", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-dashboard")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-stat-active")).toBeVisible();
    await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0);
  });
});
