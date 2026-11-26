import { test, expect, seedAuth, waitForHydrated, expectMutationDenied } from "./fixtures";

test.describe("pw-chronon-run-now", () => {
  test("pw-chronon-run-now-happy-admin", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("run-now-button")).toBeVisible({ timeout: 60_000 });
    // A script with no parameters still gets the dialog, so the person sees what will run.
    await page.getByTestId("run-now-button").getByRole("button").click();
    const dialog = page.getByTestId("run-now-dialog");
    await expect(dialog).toBeVisible({ timeout: 60_000 });
    await expect(page.getByRole("dialog")).toContainText(`Run ${seeded.fixtures.job_name} now?`);
    await expect(dialog).toContainText("This script takes no parameters.", { timeout: 60_000 });
    await expect(page.getByTestId("run-now-facts")).toContainText("Worker pool");
    await expect(page.getByTestId("run-now-schedule-line")).toContainText("The schedule doesn't change.");
    await page.getByTestId("run-now-submit").getByRole("button").click();
    await expect(page.getByTestId("chronon-run-started-notice")).toContainText("Run started", {
      timeout: 60_000,
    });
    await expect(page.locator(".orbital-message-bar--error")).toHaveCount(0);
  });

  test("pw-chronon-run-now-happy-typed-field", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.params_job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByTestId("run-now-button").getByRole("button").click();
    const field = page.getByTestId("run-now-param-older_than_days");
    const input = field.locator("input");
    await expect(input).toHaveValue("30", { timeout: 60_000 });
    const submit = page.getByTestId("run-now-submit").getByRole("button");

    await input.fill("fourteen");
    await expect(field).toContainText("Enter a whole number");
    await expect(submit).toBeDisabled();

    await input.fill("14");
    await expect(field).toContainText("Stored value: 30. This run uses 14; the stored value stays 30.");
    await expect(submit).toBeEnabled();
    await submit.click();

    const notice = page.getByTestId("chronon-run-started-notice");
    await expect(notice).toContainText("The schedule is unchanged.", { timeout: 60_000 });
    await notice.getByRole("link", { name: "Open the run" }).click();
    await expect(page).toHaveURL(/\/chronon\/runs\/[^/]+$/, { timeout: 60_000 });
    await waitForHydrated(page);
    await expect(page.getByTestId("run-started-by")).toContainText("Ada Admin (Run now)", {
      timeout: 60_000,
    });
    await expect(page.getByTestId("run-parameters")).toContainText("older_than_days: 14 (stored: 30)");
  });

  test("pw-chronon-run-now-sad-non-admin", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    await page.goto(`/chronon/jobs/${encodeURIComponent(seeded.fixtures.job_id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("chronon-job-detail")).toBeVisible({ timeout: 60_000 });
    // Outsiders may still open detail (session reads); run-now requires ChrononAdmin.
    const runNow = page.getByTestId("run-now-button");
    if ((await runNow.count()) > 0) {
      await runNow.getByRole("button").click();
      await expectMutationDenied(page);
    } else {
      // Permission-gated chrome hidden is also a valid deny containment.
      await expect(runNow).toHaveCount(0);
    }
  });
});
