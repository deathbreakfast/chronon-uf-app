import { test, expect, seedAuth, waitForHydrated, type Page } from "./fixtures";

const jobPath = (id: string, tab?: string) =>
  `/chronon/jobs/${encodeURIComponent(id)}${tab ? `?tab=${tab}` : ""}`;

async function open(page: Page, path: string) {
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
}

async function openAccessTab(page: Page, jobId: string) {
  await open(page, jobPath(jobId));
  await page.getByRole("tab", { name: "Access" }).click();
  // Orbital's Card copies its attributes onto its own <style> tag too; match the card itself.
  await expect(page.locator('div[data-testid="access-card"]')).toBeVisible({ timeout: 60_000 });
}

async function runNow(page: Page) {
  await page.getByTestId("run-now-button").getByRole("button").click();
  await expect(page.getByTestId("run-now-dialog")).toBeVisible({ timeout: 60_000 });
  await page.getByTestId("run-now-submit").getByRole("button").click();
  await expect(page.getByTestId("chronon-run-started-notice")).toContainText("Run started", {
    timeout: 60_000,
  });
}

test.describe("e2e.access", () => {
  test("e2e.access.admins_own_every_job", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await open(page, "/chronon/jobs");
    const row = page.getByRole("row").filter({ hasText: seeded.fixtures.access_job_name });
    await expect(row.getByTestId("jobs-your-access")).toHaveText("Owner", { timeout: 60_000 });

    await openAccessTab(page, seeded.fixtures.access_job_id);
    await expect(
      page.getByTestId("access-row").filter({ hasText: seeded.fixtures.admins_group }),
    ).toBeVisible({ timeout: 60_000 });
    // Switching back keeps the overview mounted: no denied state, no reload.
    await page.getByRole("tab", { name: "Overview" }).click();
    await expect(page.getByTestId("job-overview-recent-runs")).toBeVisible();
    await expect(page.getByTestId("access-denied")).toHaveCount(0);
  });

  test("e2e.access.team_can_run", async ({ page }) => {
    let seeded = await seedAuth(page, "member");
    await open(page, jobPath(seeded.fixtures.access_job_id));
    // Before the grant the member sees the job, can't run it, and can ask.
    await expect(page.getByTestId("run-now-denied")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("denied-action-button").first()).toBeDisabled();
    await expect(page.getByTestId("access-denied").first()).toBeVisible();

    seeded = await seedAuth(page, "admin");
    await openAccessTab(page, seeded.fixtures.access_job_id);
    await page.getByTestId("access-card-add").click();
    const fields = page.getByTestId("add-access-fields");
    await expect(fields).toBeVisible({ timeout: 60_000 });
    await fields.getByRole("combobox").first().fill("Platform");
    await page
      .getByTestId("principal-picker-row")
      .filter({ hasText: seeded.fixtures.team_name })
      .click({ timeout: 60_000 });
    await fields.locator("select").selectOption("run");
    await page.getByTestId("add-access-save").click();
    const teamRow = page.getByTestId("access-row").filter({ hasText: seeded.fixtures.team_name });
    await expect(teamRow).toBeVisible({ timeout: 60_000 });

    seeded = await seedAuth(page, "member");
    await open(page, "/chronon/jobs");
    const listRow = page.getByRole("row").filter({ hasText: seeded.fixtures.access_job_name });
    await expect(listRow.getByTestId("jobs-your-access")).toHaveText("Can run", {
      timeout: 60_000,
    });
    await open(page, jobPath(seeded.fixtures.access_job_id));
    await expect(page.getByTestId("run-now-button")).toBeVisible({ timeout: 60_000 });
    await runNow(page);
    await open(page, jobPath(seeded.fixtures.access_job_id, "runs"));
    const runs = page.getByTestId("chronon-runs-data-table");
    await expect(runs).toContainText("member (Run now)", { timeout: 60_000 });
    await expect(runs).toContainText("Completed");
  });

  test("e2e.access.request_and_approve", async ({ page }) => {
    let seeded = await seedAuth(page, "outsider");
    await open(page, "/chronon/jobs");
    const listRow = page.getByRole("row").filter({ hasText: seeded.fixtures.request_job_name });
    await expect(listRow.getByTestId("jobs-your-access")).toHaveText("No access", {
      timeout: 60_000,
    });

    await open(page, jobPath(seeded.fixtures.request_job_id));
    const denied = page.getByTestId("access-denied").first();
    await expect(denied).toBeVisible({ timeout: 60_000 });
    await expect(denied).toHaveAttribute("data-action", "View");
    await denied.getByTestId("access-denied-request").click();
    const fields = page.getByTestId("request-access-fields");
    await expect(fields).toBeVisible();
    await fields.getByRole("textbox").last().fill("I'm on call for the rollup this week");
    await page.getByTestId("request-access-send").click();
    await expect(denied.getByTestId("access-denied-sent")).toBeVisible({ timeout: 60_000 });

    // The owner approves it through Gauge's request flow, as themselves.
    await seedAuth(page, "admin");
    const approved = await page.request.post("/api/test/gauge/approve-requests");
    expect(approved.ok()).toBeTruthy();
    const body = (await approved.json()) as { approved: string[] };
    expect(body.approved.length).toBeGreaterThan(0);

    seeded = await seedAuth(page, "outsider");
    await open(page, jobPath(seeded.fixtures.request_job_id));
    await expect(page.getByTestId("job-overview-recent-runs")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("access-denied")).toHaveCount(0);
    await open(page, "/chronon/jobs");
    await expect(
      page
        .getByRole("row")
        .filter({ hasText: seeded.fixtures.request_job_name })
        .getByTestId("jobs-your-access"),
    ).toHaveText("Can view", { timeout: 60_000 });
  });

  test("e2e.access.last_owner_refused", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    await openAccessTab(page, seeded.fixtures.access_job_id);
    const adminsRow = page
      .getByTestId("access-row")
      .filter({ hasText: seeded.fixtures.admins_group });
    await adminsRow.getByTestId("access-row-menu").click();
    await page.getByRole("menuitem", { name: "Remove" }).click();
    await page.getByTestId("remove-access-confirm").click();
    await expect(page.getByTestId("remove-access-error")).toContainText(
      "needs at least one owner",
      { timeout: 60_000 },
    );
    await page.keyboard.press("Escape");
    await expect(adminsRow).toBeVisible();
  });

  test("e2e.access.run_refused_after_run_removed", async ({ page }) => {
    let seeded = await seedAuth(page, "member");
    // Hold new runs in the queue so access can change before the run starts.
    const hold = await page.request.post("/api/test/runs/hold", { data: { hold: true } });
    expect(hold.ok()).toBeTruthy();
    try {
      await open(page, jobPath(seeded.fixtures.refused_job_id));
      await expect(page.getByTestId("run-now-button")).toBeVisible({ timeout: 60_000 });
      await runNow(page);

      // The owner takes Run away from the team, leaving Can view.
      seeded = await seedAuth(page, "admin");
      await openAccessTab(page, seeded.fixtures.refused_job_id);
      const teamRow = page
        .getByTestId("access-row")
        .filter({ hasText: seeded.fixtures.team_name });
      await teamRow.getByTestId("access-row-role-select").locator("select").selectOption("view");
      await expect(page.getByTestId("access-card-notice")).toBeVisible({ timeout: 60_000 });
    } finally {
      const released = await page.request.post("/api/test/runs/release");
      expect(released.ok()).toBeTruthy();
    }

    await open(page, jobPath(seeded.fixtures.refused_job_id, "runs"));
    const runs = page.getByTestId("chronon-runs-data-table");
    await expect(runs).toContainText("Refused", { timeout: 60_000 });
    await expect(runs).toContainText(
      `member (Run now) · no longer holds Run on ${seeded.fixtures.refused_job_name}`,
    );

    // The member can still see the job, and Run now is now the disabled, ask-for-it kind.
    seeded = await seedAuth(page, "member");
    await open(page, jobPath(seeded.fixtures.refused_job_id));
    await expect(page.getByTestId("run-now-denied")).toBeVisible({ timeout: 60_000 });
  });
});
