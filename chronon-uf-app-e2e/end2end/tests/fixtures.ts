import { test as base, expect, type Page } from "@playwright/test";

export type SeedAuthKind = "anonymous" | "admin" | "outsider" | "unverified";

export type SeedFixtures = {
  script_name: string;
  job_id: string;
  job_name: string;
  run_id: string;
};

/** All Chronon Help inventory keys — seed as seen so non-tour specs stay quiet. */
const CHRONON_HELP_STEPS_SEEN = [
  { route: "/chronon", feature_highlight: "chronon-intro", spotlight: null, replay: false },
  {
    route: "/chronon",
    feature_highlight: "chronon-dashboard-stats",
    spotlight: "chronon-dashboard-stats",
    replay: false,
  },
  {
    route: "/chronon",
    feature_highlight: "chronon-run-trend",
    spotlight: "chronon-run-trend-card",
    replay: false,
  },
  {
    route: "/chronon",
    feature_highlight: "chronon-view-all-runs",
    spotlight: "chronon-run-trend-view-all",
    replay: false,
  },
  {
    route: "/chronon",
    feature_highlight: "chronon-dashboard-recent",
    spotlight: "chronon-dashboard-recent-runs",
    replay: false,
  },
  {
    route: "/chronon",
    feature_highlight: "chronon-nav",
    spotlight: "chronon-nav",
    replay: false,
  },
  {
    route: "/chronon/jobs",
    feature_highlight: "chronon-jobs-intro",
    spotlight: "chronon-jobs-page",
    replay: false,
  },
  {
    route: "/chronon/jobs",
    feature_highlight: "chronon-jobs-search",
    spotlight: "chronon-jobs-search",
    replay: false,
  },
  {
    route: "/chronon/jobs",
    feature_highlight: "chronon-jobs-table",
    spotlight: "chronon-jobs-data-table",
    replay: false,
  },
  {
    route: "/chronon/jobs",
    feature_highlight: "chronon-jobs-create",
    spotlight: "chronon-jobs-create-button",
    replay: false,
  },
  {
    route: "/chronon/jobs",
    feature_highlight: "chronon-jobs-open",
    spotlight: "chronon-jobs-data-table",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-intro",
    spotlight: "chronon-job-create-page",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-back",
    spotlight: "chronon-job-create-back",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-basic",
    spotlight: "chronon-job-create-basic",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-params",
    spotlight: "chronon-job-create-params",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-schedule",
    spotlight: "chronon-job-create-schedule",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-advanced",
    spotlight: "chronon-job-create-advanced",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-cancel",
    spotlight: "chronon-job-create-cancel",
    replay: false,
  },
  {
    route: "/chronon/jobs/new",
    feature_highlight: "chronon-create-submit",
    spotlight: "chronon-job-create-submit",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-header",
    spotlight: "chronon-job-detail-header",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-revision",
    spotlight: "chronon-job-detail-revision",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-enabled",
    spotlight: "chronon-job-detail-enabled",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-run-now",
    spotlight: "chronon-job-detail-run-now",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-edit",
    spotlight: "chronon-job-detail-edit",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-save",
    spotlight: "chronon-job-detail-save",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-cancel-edit",
    spotlight: "chronon-job-detail-cancel",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-config",
    spotlight: "chronon-job-detail-config",
    replay: false,
  },
  {
    route: "/chronon/jobs/:job_id",
    feature_highlight: "chronon-job-recent",
    spotlight: "chronon-job-detail-recent-runs",
    replay: false,
  },
  {
    route: "/chronon/runs",
    feature_highlight: "chronon-runs-intro",
    spotlight: "chronon-runs-page",
    replay: false,
  },
  {
    route: "/chronon/runs",
    feature_highlight: "chronon-runs-search",
    spotlight: "chronon-runs-search",
    replay: false,
  },
  {
    route: "/chronon/runs",
    feature_highlight: "chronon-runs-table",
    spotlight: "chronon-runs-data-table",
    replay: false,
  },
  {
    route: "/chronon/runs",
    feature_highlight: "chronon-runs-open",
    spotlight: "chronon-runs-data-table",
    replay: false,
  },
  {
    route: "/chronon/runs/:run_id",
    feature_highlight: "chronon-run-header",
    spotlight: "chronon-run-detail-header",
    replay: false,
  },
  {
    route: "/chronon/runs/:run_id",
    feature_highlight: "chronon-run-job-link",
    spotlight: "chronon-run-detail-job-link",
    replay: false,
  },
  {
    route: "/chronon/runs/:run_id",
    feature_highlight: "chronon-run-timing",
    spotlight: "chronon-run-detail-timing",
    replay: false,
  },
  {
    route: "/chronon/runs/:run_id",
    feature_highlight: "chronon-run-output",
    spotlight: "chronon-run-detail-output",
    replay: false,
  },
  {
    route: "/chronon/scripts",
    feature_highlight: "chronon-scripts-intro",
    spotlight: "chronon-scripts-page",
    replay: false,
  },
  {
    route: "/chronon/scripts",
    feature_highlight: "chronon-scripts-search",
    spotlight: "chronon-scripts-search",
    replay: false,
  },
  {
    route: "/chronon/scripts",
    feature_highlight: "chronon-scripts-table",
    spotlight: "chronon-scripts-data-table",
    replay: false,
  },
] as const;

export async function seedAuth(
  page: Page,
  auth: SeedAuthKind,
  opts?: { help_tour?: boolean },
) {
  const helpTour = opts?.help_tour ?? false;
  await page.addInitScript(
    ([enableTour, seenSteps]) => {
      try {
        if (enableTour) {
          if (!sessionStorage.getItem("uf.help.e2e_tour_cleared")) {
            localStorage.removeItem("uf.help.tour_steps");
            sessionStorage.setItem("uf.help.e2e_tour_cleared", "1");
          }
          return;
        }
        localStorage.setItem("uf.help.tour_steps", JSON.stringify(seenSteps));
      } catch {
        /* ignore */
      }
    },
    [helpTour, CHRONON_HELP_STEPS_SEEN] as const,
  );

  const res = await page.request.post("/api/test/seed-data", {
    data: { auth },
  });
  expect(res.ok()).toBeTruthy();
  return res.json() as Promise<{
    ok: boolean;
    auth: string;
    fixtures: SeedFixtures;
  }>;
}

async function bootState(page: Page): Promise<"ready" | "error" | "loading"> {
  return page.evaluate(() => {
    const html = document.documentElement;
    if (html.getAttribute("data-orbital-hydrated") === "true") {
      return "ready";
    }
    if (html.getAttribute("data-orbital-boot-state") === "error") {
      return "error";
    }
    return "loading";
  });
}

/**
 * CI evidence (chronon run 33828485186): Orbital sets boot `error` from a
 * non-WASM `unhandledrejection` matching bare `fetch`, then dismiss refuses.
 * Artifacts show SSR `main` under "Unable to load" with wasm already complete
 * — or error firing early while wasm still downloads.
 *
 * Do not gate on `e2e-auth-bootstrap` (Suspense server resource). When wasm is
 * complete, clear the error bit and re-invoke dismiss.
 */
async function clearFalsePositiveBootError(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const html = document.documentElement;
    if (html.getAttribute("data-orbital-hydrated") === "true") {
      return true;
    }
    if (html.getAttribute("data-orbital-boot-state") !== "error") {
      return false;
    }
    const progress = (
      window as unknown as {
        __orbitalBootProgress?: { steps?: { wasm?: string } };
        __orbitalBootDismissOverlay?: () => void;
      }
    );
    const wasmComplete =
      progress.__orbitalBootProgress?.steps?.wasm === "complete" ||
      document.querySelectorAll(".orbital-boot-step--complete").length >= 4;
    const shellReady = !!document.querySelector("main");
    if (!wasmComplete || !shellReady) {
      return false;
    }
    html.removeAttribute("data-orbital-boot-state");
    if (typeof progress.__orbitalBootDismissOverlay === "function") {
      progress.__orbitalBootDismissOverlay();
    }
    if (html.getAttribute("data-orbital-hydrated") !== "true") {
      html.setAttribute("data-orbital-hydrated", "true");
      document.getElementById("orbital-boot-overlay")?.remove();
    }
    return true;
  });
}

/**
 * Wait for Orbital hydrate. On terminal boot `error`, wait for wasm to finish
 * under a false-positive error before reloading. Never reload while `loading`.
 */
export async function waitForHydrated(page: Page, timeoutMs = 180_000) {
  const deadline = Date.now() + timeoutMs;
  let refreshes = 0;
  const maxRefreshes = 3;

  while (Date.now() < deadline) {
    const state = await bootState(page);
    if (state === "ready") {
      break;
    }
    if (state === "error") {
      if (await clearFalsePositiveBootError(page)) {
        break;
      }
      const waitUntil = Math.min(Date.now() + 30_000, deadline);
      let recovered = false;
      while (Date.now() < waitUntil) {
        await page.waitForTimeout(500);
        if ((await bootState(page)) === "ready") {
          recovered = true;
          break;
        }
        if (await clearFalsePositiveBootError(page)) {
          recovered = true;
          break;
        }
      }
      if (recovered) {
        break;
      }
      if (refreshes >= maxRefreshes) {
        break;
      }
      refreshes += 1;
      // Let Chromium release a failed compile before retrying the ~50–100MiB wasm.
      await page.waitForTimeout(1_500);
      await page.reload({ waitUntil: "load" });
      continue;
    }
    await page.waitForTimeout(500);
  }

  if ((await bootState(page)) === "error") {
    await clearFalsePositiveBootError(page);
  }

  await expect
    .poll(async () => bootState(page), { timeout: 10_000 })
    .toBe("ready");
  await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("e2e-auth-bootstrap")).toBeAttached({
    timeout: 30_000,
  });
}


/** Higgs / server-fn deny surfaces as an Orbital error MessageBar. */
export async function expectMutationDenied(page: Page) {
  await expect(page.locator(".orbital-message-bar--error")).toBeVisible({
    timeout: 60_000,
  });
}

export const test = base;
export { expect };
