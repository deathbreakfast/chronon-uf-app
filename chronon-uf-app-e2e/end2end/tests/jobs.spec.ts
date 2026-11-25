import { test, expect, seedAuth, waitForHydrated, uniqueJobName } from "./fixtures";

test.describe("pw-chronon-jobs", () => {
  test("pw-chronon-jobs-happy-list-detail", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/jobs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-jobs-page")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-jobs-data-table")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-jobs-data-table").getByText(seeded.fixtures.job_name)).toBeVisible({
      timeout: 60_000,
    });
    const row = page.getByTestId("chronon-jobs-data-table").getByRole("row", {
      name: new RegExp(seeded.fixtures.job_name),
    });
    await expect(row.getByTestId("jobs-schedule-cell")).toContainText("Every hour");
    await expect(row.getByTestId("jobs-schedule-cell")).toContainText("0 * * * *");
    await expect(row.getByTestId("jobs-next-run-cell")).toContainText(/UTC|Paused/);
    await expect(page.getByTestId("jobs-create-button")).toBeVisible();
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("job-header").getByText(seeded.fixtures.job_name)).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-chronon-jobs-happy-last-run-failed", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon/jobs", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    // The purge fixture's newest run failed on every attempt a few minutes ago.
    const row = page.getByTestId("chronon-jobs-data-table").getByRole("row", {
      name: new RegExp(seeded.fixtures.purge_job_name),
    });
    const cell = row.getByTestId("jobs-last-run-cell");
    await expect(cell).toContainText("Failed", { timeout: 60_000 });
    await expect(cell).not.toContainText("\u2014");
  });

  test("pw-chronon-jobs-sad-outsider-no-owner-or-schedule", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const caption = page.getByTestId("job-header-caption");
    await expect(caption).toContainText("Script", { timeout: 60_000 });
    await expect(caption).not.toContainText("Owner:");
    await expect(page.getByTestId("schedule-card-sentence")).toHaveText("Hidden");
  });

  test("pw-chronon-jobs-sad-unknown-job", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/jobs/__chronon_e2e_no_such_job__", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText("Job not found.")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("job-not-found").getByRole("button", { name: "Back to Jobs" })).toBeVisible();
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

test.describe("pw-chronon-jobs-create", () => {
  test("pw-chronon-jobs-sad-create-marks-every-field", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/jobs/new", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = uniqueJobName("e2e-thumbs");
    await page.getByTestId("job-name").locator("input").fill(name);
    const select = page.getByTestId("job-script").locator("select");
    await expect(select.locator('option[value="e2e_thumbnails"]')).toBeAttached({ timeout: 60_000 });
    await select.selectOption("e2e_thumbnails");
    await expect(page.getByTestId("param-max_width")).toContainText("Whole number", { timeout: 30_000 });
    await page.getByTestId("param-max_width").locator("input").fill("wide");
    await page.getByTestId("create-job").getByRole("button").click();

    await expect(page.getByTestId("create-errors-summary")).toContainText("2 fields need a look", {
      timeout: 30_000,
    });
    await expect(page.getByTestId("param-bucket")).toContainText("Enter a value.");
    await expect(page.getByTestId("param-max_width")).toContainText("Enter a whole number, such as 30.");
    await expect(page).toHaveURL(/\/chronon\/jobs\/new/);

    await page.getByTestId("param-bucket").locator("input").fill("thumbnails");
    await page.getByTestId("param-max_width").locator("input").fill("800");
    await page.getByTestId("create-job").getByRole("button").click();
    await expect(page).toHaveURL(/\/chronon\/jobs\/[^/?]+\?created=1/, { timeout: 60_000 });
    await expect(page.getByTestId("job-flash")).toContainText(`${name} first runs`, { timeout: 60_000 });
    await expect(page.getByTestId("job-params-table")).toContainText("thumbnails");
    await expect(page.getByTestId("job-params-table")).toContainText("Whole number");
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
