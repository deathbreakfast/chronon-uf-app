import { test, expect, seedAuth, waitForHydrated, createJobViaForm, uniqueJobName } from "./fixtures";

test.describe("pw-chronon-dashboard", () => {
  test("pw-chronon-dashboard-happy-kpis", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-dashboard")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-dashboard")).toContainText(
      "Scheduled jobs on this site: what needs you, and what runs next.",
    );
    for (const id of [
      "chronon-stat-failing-jobs",
      "chronon-stat-runs-today",
      "chronon-stat-worked-24h",
      "chronon-stat-running-now",
      "chronon-stat-paused",
    ]) {
      await expect(page.getByTestId(id)).toBeVisible({ timeout: 60_000 });
    }
    await expect(page.getByTestId("chronon-stat-worked-24h")).toContainText(/\d+\.\d%|—/);
    await expect(page.locator("#chronon-dashboard-recent-runs").getByText(seeded.fixtures.job_name).first()).toBeVisible({
      timeout: 60_000,
    });
    // The chart moved to the Runs page.
    await expect(page.locator("#chronon-run-trend-card")).toHaveCount(0);
  });

  test("pw-chronon-dashboard-happy-retries-count-once", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    // The seeded purge failed on its first try and three retries: one failing job.
    await expect(page.getByTestId("chronon-stat-failing-jobs")).toContainText(/[1-9]/, {
      timeout: 60_000,
    });
    const purgeRow = page
      .getByTestId("chronon-recent-run-row")
      .filter({ hasText: seeded.fixtures.purge_job_name })
      .first();
    await expect(purgeRow).toContainText("4 of 4 attempts", { timeout: 60_000 });
    await expect(purgeRow).toContainText("Failed");
    await expect(purgeRow).toContainText("Schedule");
  });

  test("pw-chronon-dashboard-happy-attention-failed-run", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const failed = page
      .getByTestId("chronon-attention-item")
      .filter({ hasText: `${seeded.fixtures.purge_job_name} failed` });
    await expect(failed).toHaveCount(1, { timeout: 60_000 });
    await expect(failed).toHaveAttribute("data-kind", "failed");
    await expect(failed).toContainText("All 3 retries failed the same way");
    await expect(failed).toContainText("unless someone runs it");
    await failed.getByTestId("chronon-attention-see-run").getByRole("link").click();
    await expect(page).toHaveURL(
      new RegExp(`/chronon/runs/${encodeURIComponent(seeded.fixtures.purge_run_id)}`),
      { timeout: 60_000 },
    );
  });

  test("pw-chronon-dashboard-happy-attention-paused-job", async ({ page }) => {
    await seedAuth(page, "admin");
    const name = uniqueJobName("e2e-attn-paused");
    await createJobViaForm(page, name);
    await page.getByTestId("pause-resume-button").getByRole("button").click();
    await page.getByTestId("pause-dialog-confirm").getByRole("button").click();
    await expect(page.getByTestId("paused-banner")).toBeVisible({ timeout: 60_000 });

    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const paused = page.getByTestId("chronon-attention-item").filter({ hasText: `${name} is paused` });
    await expect(paused).toHaveCount(1, { timeout: 60_000 });
    await expect(paused).toHaveAttribute("data-kind", "paused");
    await expect(paused).toContainText("paused it today");
    await expect(paused.getByTestId("chronon-attention-see-run")).toHaveCount(0);
    await paused.getByTestId("chronon-attention-open-job").getByRole("link").click();
    await expect(page).toHaveURL(/\/chronon\/jobs\//, { timeout: 60_000 });
    await expect(page.getByTestId("paused-banner")).toBeVisible({ timeout: 60_000 });
  });

  test("pw-chronon-dashboard-happy-coming-up-in-words", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const rows = page.getByTestId("chronon-coming-up-row");
    await expect(rows.first()).toBeVisible({ timeout: 60_000 });
    expect(await rows.count()).toBeLessThanOrEqual(5);
    await expect(rows.first()).toContainText(/(Today|Tomorrow|\w{3}, \w{3} \d+), \d{2}:\d{2}/);
    await expect(rows.first()).toContainText(/in \d+ (min|h|days)/);
    await expect(rows.first()).toContainText(/Every|Once on/);
    await page.getByTestId("chronon-coming-up-all-jobs").getByRole("link").click();
    await expect(page).toHaveURL(/\/chronon\/jobs$/, { timeout: 60_000 });
  });

  test("pw-chronon-dashboard-sad-no-outage-bar-when-healthy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-dashboard")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-stat-paused")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("chronon-dashboard-outage")).toHaveCount(0);
    await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0);
  });
});
