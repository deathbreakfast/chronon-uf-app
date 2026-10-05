import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-chronon-jobs", () => {
  test("pw-chronon-jobs-happy-list-detail", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/jobs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-jobs-page")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-jobs-data-table")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(seeded.fixtures.job_name).first()).toBeVisible({
      timeout: 60_000,
    });
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(seeded.fixtures.job_name).first()).toBeVisible();
  });

  test("pw-chronon-jobs-sad-unknown-job", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/jobs/__chronon_e2e_no_such_job__", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText("Job not found.")).toBeVisible({ timeout: 60_000 });
  });

  test("pw-chronon-jobs-sad-unverified-create", async ({ page }) => {
    await seedAuth(page, "unverified");
    await page.goto("/chronon/jobs/new", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-create-page")).toHaveCount(0);
    await expect(
      page.getByTestId("email-verification-required-empty-state"),
    ).toBeAttached({ timeout: 60_000 });
  });
});

test.describe("pw-chronon-jobs-pool", () => {
  test("pw-chronon-jobs-happy-edit-pool", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    const detailPath = `/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`;
    const setPool = async (pool: string) => {
      await page.goto(detailPath, { waitUntil: "domcontentloaded" });
      await waitForHydrated(page);
      await page.getByTestId("edit-job-button").locator("button").click();
      const select = page.getByTestId("job-edit-pool").locator("select");
      await expect(select).toBeVisible({ timeout: 60_000 });
      await select.selectOption(pool);
      await page.getByTestId("save-job-button").locator("button").click();
      await expect(page.getByTestId("job-detail-pool")).toHaveText(pool, { timeout: 60_000 });
    };

    await page.goto(detailPath, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("job-detail-pool")).toHaveText("general", { timeout: 60_000 });
    await page.getByTestId("edit-job-button").locator("button").click();
    await expect(page.getByTestId("job-edit-pool").locator("option")).toHaveText([
      "general (default)",
      "e2e-pool-a",
    ]);

    await setPool("e2e-pool-a");
    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("job-detail-pool")).toHaveText("e2e-pool-a", {
      timeout: 60_000,
    });

    // Put the seeded job back so later specs' runs land in the drained pool.
    await setPool("general");
  });

  test("pw-chronon-jobs-happy-create-pool-picker", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/jobs/new", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-create-page")).toBeVisible({ timeout: 60_000 });
    await page.getByText("Advanced Options").click();
    const select = page.getByTestId("job-pool").locator("select");
    await expect(select).toBeVisible({ timeout: 60_000 });
    await expect(select).toHaveValue("general");
    await select.selectOption("e2e-pool-a");
    await page.getByText("Advanced Options").click();
    await expect(page.getByText(/Pool: e2e-pool-a/)).toBeVisible({ timeout: 30_000 });
  });
});
