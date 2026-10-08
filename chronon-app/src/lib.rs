#![recursion_limit = "256"]
//! Chronon operations app — monitor scheduled jobs, run history, and the script
//! catalog under `/chronon`.
//!
//! Leptos UI mounted at `/chronon` so operators can see job schedules, trigger runs,
//! and browse registered scripts without building custom pages. Registers alongside
//! other product apps via `uf_app!` and requires an authenticated session with
//! `ChrononAdmin` before server functions load coordinator data.
//!
//! Orbital inventory macros (`uf_app!`, `orbital_routes_extract`) emit undocumented
//! associated items, so this crate allows `missing_docs` at the crate root while keeping
//! hand-written modules and items documented.
//!
//! ## Features
//!
//! - **Chronon admin routes** — Provides the nested `/chronon` route tree behind auth for
//!   dashboard, jobs, runs, and scripts. Mount once when the host router starts.
//!   [Get started](#mount-chronon-routes)
//! - **Dashboard KPIs** — Shows job and run counters on [`ChrononDashboardPage`] via
//!   [`get_dashboard_stats`] plus run-trend charts from [`get_run_stats_series`].
//!   [Get started](#dashboard-kpis)
//! - **Jobs browser** — Lists scheduled jobs and supports create and detail edits via
//!   [`get_jobs_page`], [`create_job`], and [`update_job`].
//!   [Get started](#browse-jobs)
//! - **Runs browser** — Lists run history and opens detail pages via [`get_runs_page`]
//!   and [`get_run`], and lets operators trigger [`run_job_now`].
//!   [Get started](#browse-runs)
//! - **Scripts catalog** — Shows registered scripts with signatures via
//!   [`get_scripts_page`] on [`ChrononScriptsPage`].
//!   [Get started](#browse-scripts)
//! - **Help spotlight tours** — Provides first-visit Orbital coaching on `/chronon`
//!   routes so operators learn Job / Script / Run screens one control at a time.
//!   Call [`ensure_help_steps_linked`] and enable `offering-help` on the product shell.
//!   [Get started](#help-spotlight-tours)
//! - **Server function wrappers** — Exposes [`mod@server`] Higgs `#[server]` fns and DTO
//!   re-exports backed by [`chronon_backend`] pure mapping helpers.
//!
//! ## Feature flags
//!
//! | Flag | What it enables |
//! |------|-----------------|
//! | `ssr` | Server-side rendering, Axum, Chronon coordinator, Higgs, Valence, Gauge, lepton-auth |
//! | `hydrate` | Client WASM hydrate + `leptos-use` poll helpers |
//! | `e2e-lab` | Process-local email-verification override for `chronon-uf-app-e2e` only; leave off in product hosts |
//!
//! ## Mount Chronon routes
//!
//! [`ChrononRoutes`] nests the full `/chronon` subtree inside a host Leptos `<Routes>` tree.
//! Operators get visibility into scheduled jobs, run history, and the script catalog.
//! Mount during host router setup at startup, alongside other `uf_app!` product routes —
//! the macro registers launcher metadata and the `/chronon` inventory entry.
//!
//! **Prerequisites:** `ssr` on this crate; authenticated session; `ChrononAdmin` permission
//! ([`CHRONON_ADMIN_PERMISSION`]); Chronon backend in Leptos request context for IO.
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//! use chronon_app::ChrononRoutes;
//!
//! view! {
//!     <Routes fallback=|| "not found">
//!         <ChrononRoutes />
//!     </Routes>
//! }
//! ```
//!
//! On success `/chronon` resolves to the dashboard, `/chronon/jobs` lists scheduled jobs,
//! and nested run and script routes load their pages. Unauthenticated sessions are rejected
//! by server functions — see root `SECURITY.md`.
//!
//! ## Dashboard KPIs
//!
//! The dashboard answers how much scheduled work is active right now: total and paused
//! jobs, runs started today, and runs currently executing.
//! [`ChrononDashboardPage`] calls [`get_dashboard_stats`] on each SSR render and
//! [`get_run_stats_series`] for trend charts — use this landing page after mounting routes
//! when operators need a quick health snapshot.
//!
//! **Prerequisites:** [`ChrononRoutes`] mounted; `ssr` feature; `ChrononAdmin` permission;
//! Chronon backend request context wired.
//!
//! ```rust,ignore
//! use chronon_app::{
//!     ChrononDashboardPage, get_dashboard_stats, get_run_stats_series, DashboardStats,
//! };
//!
//! // ChrononDashboardPage calls these on each SSR render:
//! let stats: DashboardStats = get_dashboard_stats().await?;
//! assert_eq!(stats.total_jobs, 3);
//! assert_eq!(stats.running_now, 1);
//!
//! let series = get_run_stats_series(86_400).await?;
//! assert!(!series.is_empty());
//! ```
//!
//! On success `stats` carries `total_jobs`, `active_jobs`, `total_runs_today`, and
//! `running_now`; `series` holds chart buckets for successful and failed runs. Blank or
//! unsafe path ids are rejected by `chronon_backend::validate_*` before coordinator IO.
//!
//! ## Browse jobs
//!
//! Job pages list scheduled work with cron or run-once schedules.
//! [`ChrononJobsPage`] loads [`get_jobs_page`] for the index; [`ChrononJobCreatePage`] calls
//! [`create_job`] after email verification; [`ChrononJobDetailPage`] uses [`update_job`] to
//! change cron, params, pool, or enabled state. Open these routes when operators add schedules
//! or pause a noisy job.
//!
//! Both forms pick a worker pool from [`list_chronon_job_pools`]. Hosts that pin Chronon
//! workers to Pion pools put an `Arc<dyn ChrononPoolProvider>` in request context; without
//! one the picker offers only `general`, and saves naming any other pool are rejected.
//!
//! **Prerequisites:** Routes mounted; job names must pass `chronon_backend::validate_job_name`;
//! create and edit require a verified email.
//!
//! ```rust,ignore
//! use chronon_app::{
//!     ChrononJobsPage, get_jobs_page, create_job, update_job, Job, UpdateJobRequest,
//! };
//! use chronon_backend::{CreateJobRequest, CreateJobScheduleType};
//! use orbital_paging::PageRequest;
//!
//! // ChrononJobsPage loads get_jobs_page for the index:
//! let page = get_jobs_page(PageRequest::default()).await?;
//! let first: &Job = page.items.first().expect("scheduled job");
//! assert_eq!(first.name, "nightly-sync");
//!
//! create_job(CreateJobRequest {
//!     job_name: "nightly-sync".into(),
//!     script_name: "reports.export".into(),
//!     schedule_type: CreateJobScheduleType::Manual,
//!     cron_expr: None,
//!     timezone: None,
//!     run_once_at: None,
//!     params: serde_json::json!({}),
//!     concurrency: 1,
//!     timeout_seconds: 60,
//!     max_retries: 0,
//!     pool: None,
//! }).await?;
//!
//! update_job(
//!     "job-1".into(),
//!     UpdateJobRequest {
//!         job_name: "nightly-sync".into(),
//!         cron_expr: None,
//!         timezone: None,
//!         params: serde_json::json!({}),
//!         enabled: false,
//!         pool: None,
//!     },
//! ).await?;
//! ```
//!
//! On success the index returns sorted [`Job`] rows and detail resolves one schedule or maps
//! a missing id to a server error. Updates persist through the coordinator after validation.
//!
//! ## Browse runs
//!
//! Run pages list execution attempts and full log detail on the detail view.
//! [`ChrononRunsPage`] loads [`get_runs_page`] with optional filters;
//! [`ChrononRunDetailPage`] calls [`get_run`] for one run id; operators trigger
//! [`run_job_now`] from run detail when a schedule needs an immediate attempt. Open these
//! routes when auditing failures or forcing a one-off run.
//!
//! **Prerequisites:** Routes mounted; run ids must pass `chronon_backend::validate_run_id`.
//!
//! ```rust,ignore
//! use chronon_app::{ChrononRunsPage, get_runs_page, get_run, run_job_now, Run};
//! use orbital_paging::PageRequest;
//!
//! // ChrononRunsPage loads get_runs_page with optional filters:
//! let page = get_runs_page(PageRequest::default()).await?;
//! let first: &Run = page.items.first().expect("run row");
//! assert_eq!(first.id, "run-1");
//!
//! let detail = get_run("run-1".into()).await?;
//! assert_eq!(detail.id, "run-1");
//!
//! run_job_now("job-1".into()).await?;
//! ```
//!
//! On success the index returns run preview rows and detail resolves one attempt or errors
//! when the id is unknown. Oversized or path-unsafe ids fail validation before coordinator
//! lookup.
//!
//! ## Browse scripts
//!
//! The scripts catalog lists every registered handler with signature and parameter metadata.
//! [`ChrononScriptsPage`] loads [`get_scripts_page`] so operators can pick a script name when
//! creating a job. Open this route when onboarding a new schedule or confirming parameter
//! names before editing job params.
//!
//! **Prerequisites:** Routes mounted; script names must pass `chronon_backend::validate_script_name`.
//!
//! ```rust,ignore
//! use chronon_app::{ChrononScriptsPage, get_scripts_page, Script};
//! use orbital_paging::PageRequest;
//!
//! // ChrononScriptsPage loads get_scripts_page for the catalog:
//! let page = get_scripts_page(PageRequest::default()).await?;
//! let first: &Script = page.items.first().expect("script row");
//! assert_eq!(first.name, "reports.export");
//! ```
//!
//! On success the page returns [`Script`] rows with `name`, `signature`, and declared
//! parameters ready for the job-create form.
//!
//! ## Help spotlight tours
//!
//! Help spotlight tours are first-visit Orbital coaching panels on each `/chronon`
//! route. They teach Job / Script / Run vocabulary and one control at a time so
//! operators can find schedules and history without reading the whole UI first.
//!
//! Call [`ensure_help_steps_linked`] once when the host mounts Chronon routes
//! (alongside [`ChrononRoutes`]) so `inventory` submissions from [`mod@help_steps`]
//! stay in the binary. Signed-in users see pending steps; anonymous and other
//! access gates suppress the tour via the product shell's `AccessGate` signal.
//!
//! **Prerequisites:** `uf-help` hydrate/ssr on this crate; product host with
//! `uf-integrations` `offering-help` (or `full`) so `HelpTourPlayer` mounts.
//!
//! ```rust,ignore
//! use chronon_app::{ensure_help_steps_linked, ChrononRoutes};
//!
//! ensure_help_steps_linked();
//! // Mount ChrononRoutes in the host router as usual.
//! let inventory_linked = "chronon-help-linked";
//! assert_eq!(inventory_linked, "chronon-help-linked");
//! ```
//!
//! On success, visiting `/chronon` (and other Chronon paths) can show pending
//! spotlight steps until finished. Replay restarts the tour for the current route
//! from Help. Centered intro steps omit a cutout; later steps anchor to DOM `id`s.
//! Missing `offering-help` leaves the player unmounted — no tour, no panic.
//! See [`mod@help_steps`] for the route inventory.
//!
//! ## Routes
//!
//! Mounted under `/chronon` by [`ChrononRoutes`]. Job create/edit require a verified email.
//!
//! | Path | Page | Key server fn(s) |
//! |---|---|---|
//! | `/chronon` | [`ChrononDashboardPage`] | [`get_dashboard_stats`], [`get_recent_runs`], [`get_run_stats_series`] |
//! | `/chronon/jobs` | [`ChrononJobsPage`] | [`get_jobs_page`] |
//! | `/chronon/jobs/new` (email-verified) | [`ChrononJobCreatePage`] | [`create_job`] |
//! | `/chronon/jobs/:job_id` (email-verified) | [`ChrononJobDetailPage`] | [`get_job`], [`get_job_revisions`], [`update_job`], [`get_job_runs_page`] |
//! | `/chronon/runs` | [`ChrononRunsPage`] | [`get_runs_page`] |
//! | `/chronon/runs/:run_id` | [`ChrononRunDetailPage`] | [`get_run`], [`run_job_now`] |
//! | `/chronon/scripts` | [`ChrononScriptsPage`] | [`get_scripts_page`] |
//!
//! ## Examples
//!
//! Start with [Mount Chronon routes](#mount-chronon-routes). The `chronon-backend` unit and integ
//! suites in `docs/VERIFICATION.md` cover server-fn contracts. Runnable host:
//! `examples/protected-chronon-host` (auth + dashboard KPIs; inventory `chronon` / `/chronon`).
//!
//! ## Where to look next
//!
//! - [`ChrononLayout`] — shared app bar / nav shell wrapping every route.
//! - [`mod@help_steps`] — Help spotlight tour inventory; call [`ensure_help_steps_linked`].
//! - [`mod@server`] — server functions and DTOs backing the UI.
//! - [`permissions::ChrononPermission`] — permission manifest for `ChrononAdmin`.
//! - [`live`] / [`photon_ws`] — client poll-tick and SSR route merge for live updates.
//! - `chronon_backend` — id validation and pure mapping helpers used by these server fns.

#![allow(missing_docs)]
#![cfg_attr(
    feature = "ssr",
    allow(dead_code, unused_imports, unused_variables, unknown_lints)
)]
use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

mod components;
#[cfg(feature = "e2e-lab")]
pub mod e2e_lab;
/// Help spotlight tour inventory for Chronon ops routes.
pub mod help_steps;
mod layout;
mod lazy_routes;
/// Client-side live-update hooks (poll tick, placeholder per-job live source).
pub mod live;
pub(crate) mod pages;
/// Permission manifest for Chronon admin server functions.
pub mod permissions;
#[cfg(feature = "ssr")]
pub mod photon_ws;
pub mod server;
pub(crate) mod utils;

pub use help_steps::ensure_help_steps_linked;
pub use layout::ChrononLayout;
pub use lazy_routes::{
    prefetch_family, ChrononDashboardRoute, ChrononJobsRoute, ChrononLayoutRouteView,
    ChrononRunDetailRoute, ChrononRunsRoute, ChrononScriptsRoute, ChrononVerifiedJobCreateRoute,
    ChrononVerifiedJobDetailRoute,
};
pub use pages::{
    ChrononDashboardPage, ChrononJobCreatePage, ChrononJobDetailPage, ChrononJobsPage,
    ChrononRunDetailPage, ChrononRunsPage, ChrononScriptsPage,
};
// Types, permission const, and server fn stubs (WASM-safe client stubs + SSR bodies).
pub use server::{
    create_job, get_dashboard_stats, get_job, get_job_revisions, get_job_runs_page, get_jobs,
    get_jobs_page, get_recent_runs, get_run, get_run_stats_series, get_runs, get_runs_page,
    get_scripts, get_scripts_page, list_chronon_job_pools, run_job_now, update_job,
    ChrononPoolPickRow, ChrononPoolProvider, DashboardChartPoint, DashboardChartSeries,
    DashboardStats, Job, JobRevision, JobStatus, RecentRun, Run, RunStatus, Script, ScriptParam,
    UpdateJobRequest, CHRONON_ADMIN_PERMISSION,
};

// Define the Chronon application metadata.
uf_app! {
    name: "Chronon",
    id: "chronon",
    description: "Job scheduling and script execution",
    icon: "⏱",
    version: "0.1.0",
    routes: ChrononRoutes,
    route_path: "/chronon",
    repository: "https://github.com/unified-field-dev/chronon-uf-app",
    permission_manifest: permissions::ChrononPermission,
}

/// Chronon application routes.
///
/// Leaf pages are [`LazyRoute`](leptos_router::LazyRoute) views so
/// `cargo leptos --split` can emit a separate WASM chunk for this family.
#[allow(missing_docs)]
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn ChrononRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    crate::help_steps::ensure_help_steps_linked();
    view! {
        <ParentRoute path=path!("chronon") view=ChrononLayoutRouteView>
            <Route path=path!("") view={Lazy::<ChrononDashboardRoute>::new()} />
            <Route path=path!("jobs") view={Lazy::<ChrononJobsRoute>::new()} />
            <Route path=path!("jobs/new") view={Lazy::<ChrononVerifiedJobCreateRoute>::new()} />
            <Route path=path!("jobs/:job_id") view={Lazy::<ChrononVerifiedJobDetailRoute>::new()} />
            <Route path=path!("runs") view={Lazy::<ChrononRunsRoute>::new()} />
            <Route path=path!("runs/:run_id") view={Lazy::<ChrononRunDetailRoute>::new()} />
            <Route path=path!("scripts") view={Lazy::<ChrononScriptsRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}
