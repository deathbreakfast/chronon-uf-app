import { test, expect, seedAuth, waitForHydrated, createJobViaForm, uniqueJobName } from "./fixtures";

test.describe("pw-chronon-pause", () => {
  test("pw-chronon-pause-happy-pause-resume-survive-reload", async ({ page }) => {
    await seedAuth(page, "admin");
    await createJobViaForm(page, uniqueJobName("e2e-pause"));

    const toggle = page.getByTestId("pause-resume-button").getByRole("button");
    await expect(toggle).toHaveText(/Pause/, { timeout: 60_000 });
    await toggle.click();
    await expect(page.getByTestId("pause-dialog")).toContainText("No runs until someone resumes it.", {
      timeout: 30_000,
    });
    await page.getByTestId("pause-dialog-confirm").getByRole("button").click();

    const banner = page.getByTestId("paused-banner");
    await expect(banner).toBeVisible({ timeout: 60_000 });
    await expect(banner).toContainText("Paused by Ada Admin", { timeout: 60_000 });

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("paused-banner")).toContainText("Paused by", { timeout: 60_000 });
    await expect(page.getByTestId("pause-resume-button").getByRole("button")).toHaveText(/Resume/);

    await page.getByTestId("paused-banner-resume").getByRole("button").click();
    await expect(page.getByTestId("resume-run-once")).toBeVisible({ timeout: 30_000 });
    await page.getByTestId("pause-dialog-confirm").getByRole("button").click();
    await expect(page.getByTestId("paused-banner")).toHaveCount(0, { timeout: 60_000 });

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("pause-resume-button").getByRole("button")).toHaveText(/Pause/, {
      timeout: 60_000,
    });
    await expect(page.getByTestId("paused-banner")).toHaveCount(0);
  });
});
