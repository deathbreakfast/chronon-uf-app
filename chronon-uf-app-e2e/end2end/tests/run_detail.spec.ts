import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-chronon-run-detail", () => {
  test("pw-chronon-run-detail-happy-attempts", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto(`/chronon/runs/${encodeURIComponent(seeded.fixtures.purge_run_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const detail = page.getByTestId("chronon-run-detail");
    await expect(detail).toBeVisible({ timeout: 60_000 });

    const crumb = page.getByTestId("chronon-run-breadcrumb");
    await expect(crumb.getByRole("link", { name: "Jobs" })).toBeVisible({ timeout: 60_000 });
    await expect(crumb.getByRole("link", { name: seeded.fixtures.purge_job_name })).toBeVisible();

    // A title in words, never the raw run id.
    const title = page.getByTestId("chronon-run-title");
    await expect(title).toContainText(/^Run at \d\d:\d\d (today|yesterday)$/);
    await expect(title).not.toContainText(seeded.fixtures.purge_run_id);

    const facts = page.getByTestId("chronon-run-facts");
    await expect(facts).toContainText("Started by");
    await expect(page.getByTestId("run-started-by")).toContainText("Schedule (every day at");
    await expect(page.getByTestId("run-parameters")).toContainText("older_than_days: 30");

    const attempts = page.getByTestId("chronon-run-attempt");
    await expect(attempts).toHaveCount(4, { timeout: 60_000 });
    await expect(attempts.nth(0)).toContainText("First try");
    await expect(attempts.nth(0)).toContainText("(this one)");
    await expect(attempts.nth(3)).toContainText("Retry 3");
    await expect(page.getByTestId("chronon-run-attempts")).toContainText(
      "Retries run 30 s apart. The job allows 3.",
    );

    await expect(page.getByText("The script stopped with an error")).toBeVisible();
    await expect(page.getByText("Script output can hold personal data.")).toBeVisible();
    await expect(page.getByTestId("chronon-run-stderr")).toContainText("connection refused");
  });

  test("pw-chronon-run-detail-happy-run-again-prefilled", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto(`/chronon/runs/${encodeURIComponent(seeded.fixtures.purge_run_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByTestId("run-again-button").getByRole("button").click();
    const dialog = page.getByTestId("run-now-dialog");
    await expect(dialog).toBeVisible({ timeout: 60_000 });
    await expect(
      page.getByTestId("run-now-param-older_than_days").locator("input"),
    ).toHaveValue("30", { timeout: 60_000 });
    await expect(page.getByRole("dialog")).toContainText(`Run ${seeded.fixtures.purge_job_name} now?`);
  });

  test("pw-chronon-run-detail-sad-not-found", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/chronon/runs/__chronon_e2e_no_such_run__", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const empty = page.getByTestId("chronon-run-not-found");
    await expect(empty).toContainText("Run not found", { timeout: 60_000 });
    await empty.getByRole("link", { name: "Back to Runs" }).click();
    await expect(page).toHaveURL(/\/chronon\/runs$/, { timeout: 60_000 });
  });
});
