import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-chronon-scripts", () => {
  test("pw-chronon-scripts-happy-list", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/scripts", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-scripts-page")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-scripts-data-table")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("chronon-scripts-data-table").getByText(seeded.fixtures.script_name)).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-chronon-scripts-sad-empty-search-not-crash", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/scripts", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-scripts-page")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0);
  });
});
