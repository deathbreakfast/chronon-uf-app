//! Pure backend contracts for the Chronon ops UI server surface.
//!
//! DTO shapes and pure mapping/validation helpers that `chronon-app` `#[server]` functions
//! call after resolving Higgs and Chronon coordinator request context. Keeps job, run,
//! script, schedule, and dashboard contracts unit-testable without a Leptos host or UI graph.
//!
//! ## Features
//!
//! - **Id validation** — Validates job ids, names, run ids, and script names so blank,
//!   oversized, or path-unsafe values fail closed before coordinator lookups.
//!   [Get started](#validate-ids)
//! - **Job/run/schedule mapping** — Builds UI DTOs from coordinator jobs and runs and applies
//!   cron or run-once schedules on create, without performing Chronon IO.
//!   [Get started](#map-job-run-schedule)
//! - **Dashboard aggregates** — Provides KPI counters for jobs and today's runs via
//!   [`dashboard_stats_from_jobs_and_runs`]. [Get started](#dashboard-kpis)
//! - **Ops path encoding** — Builds percent-encoded path segments for `/chronon` hrefs via
//!   [`encode_ops_path_segment`], [`chronon_job_path`], [`chronon_run_path`], and related
//!   helpers.
//! - **`DataTable` query adapters** — Supports quick-search and structured filters for job, run,
//!   and script tables via [`apply_jobs_page_query`], [`apply_runs_page_query`], and
//!   [`apply_scripts_page_query`].
//! - **Job param redaction** — Clears params and schedule fields for non-admin job
//!   list/detail DTOs via [`redact_job_params_for_non_admin`].
//!   [Get started](#redact-job-params)
//!
//! ## Validate ids
//!
//! Id validation checks path and query parameters before they reach Chronon IO, so blank or
//! path-unsafe values fail closed instead of breaking routing. [`validate_job_id`],
//! [`validate_run_id`], [`validate_job_name`], and [`validate_script_name`] run in
//! `chronon-app` server functions ahead of coordinator lookups — call them in custom wrappers
//! when you add new read paths that accept path or query parameters.
//!
//! **Prerequisites:** None beyond importing this crate; validators are synchronous and
//! return [`Result<(), ChrononIdError>`].
//!
//! ```rust,ignore
//! use chronon_backend::{
//!     validate_job_id, validate_run_id, validate_job_name, ChrononIdError,
//! };
//!
//! validate_job_id("job-1").expect("valid job");
//! assert_eq!(
//!     validate_job_id("").unwrap_err(),
//!     ChrononIdError::EmptyJobId
//! );
//! validate_run_id("run-1").expect("valid run");
//! validate_job_name("nightly-sync").expect("valid name");
//! ```
//!
//! On success validators return `Ok(())` and the trimmed id is safe for lookup. Blank,
//! oversized, control-character, slash, backslash, or `.` / `..` names map to typed
//! [`ChrononIdError`] variants with operator-facing messages.
//!
//! ## Map job run schedule
//!
//! Job/run/schedule mapping turns coordinator jobs and runs into serde-friendly DTOs the UI
//! can render without touching Chronon internals. [`backend_job_to_job`] shapes job
//! list/detail rows; [`model_run_to_app_run`] builds run history with a resolved job name;
//! [`build_create_job_model`] and [`apply_create_schedule`] wire cron or run-once schedules
//! when operators create jobs. Call these after you already hold coordinator rows in memory —
//! typically inside `chronon-app` `#[server]` handlers that assemble list or detail responses.
//!
//! **Prerequisites:** Caller already loaded jobs and runs from the coordinator for mapping
//! helpers — these functions do not perform IO except schedule parsing inside
//! [`apply_create_schedule`].
//!
//! ```rust,ignore
//! use chronon_backend::{
//!     backend_job_to_job, model_run_to_app_run, build_create_job_model,
//!     apply_create_schedule, CreateJobRequest, CreateJobScheduleType,
//! };
//! use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus};
//! use chrono::Utc;
//!
//! let core_job = CoreJob::new("nightly-sync", "reports.export");
//! let job = backend_job_to_job(core_job);
//! assert_eq!(job.name, "nightly-sync");
//!
//! let core_run = CoreRun {
//!     run_id: "run-1".into(),
//!     job_id: Some("job-1".into()),
//!     status: RunStatus::Success,
//!     scheduled_for: Utc::now(),
//!     ..Default::default()
//! };
//! let run = model_run_to_app_run(core_run, "nightly-sync".into());
//! assert_eq!(run.job_name, "nightly-sync");
//!
//! let payload = CreateJobRequest {
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
//! };
//! let mut new_job = build_create_job_model(&payload, "sig-hash".into())?;
//! apply_create_schedule(&mut new_job, &payload)?;
//! assert_eq!(new_job.job_name, "nightly-sync");
//! ```
//!
//! On success helpers return populated [`Job`] or [`Run`] rows ready for JSON serialization.
//! Schedule helpers return [`ChrononScheduleError`] when cron or run-once inputs are missing
//! or invalid — match variants in tests; map Display text into `ServerFnError` at the UI
//! boundary.
//!
//! ```rust
//! use chronon_backend::{
//!     build_create_job_model, CreateJobRequest, CreateJobScheduleType, ChrononScheduleError,
//! };
//!
//! let payload = CreateJobRequest {
//!     job_name: "cron-job".into(),
//!     script_name: "reports.export".into(),
//!     schedule_type: CreateJobScheduleType::Cron,
//!     cron_expr: Some("  ".into()),
//!     timezone: None,
//!     run_once_at: None,
//!     params: serde_json::json!({}),
//!     concurrency: 1,
//!     timeout_seconds: 60,
//!     max_retries: 0,
//! };
//! assert!(matches!(
//!     build_create_job_model(&payload, "sig".into()),
//!     Err(ChrononScheduleError::MissingCron)
//! ));
//! ```
//!
//! ## Dashboard KPIs
//!
//! Dashboard aggregates package job counts and today's run outcomes into a single
//! [`DashboardStats`] value for the ops landing page, without UI-specific formatting.
//! [`dashboard_stats_from_jobs_and_runs`] takes in-memory job and run slices plus today's
//! start; chart bucketing lives in [`run_stats_series_from_runs`] after the caller loads run
//! rows. Call this when a dashboard server function has already loaded those slices from the
//! coordinator.
//!
//! **Prerequisites:** Caller supplies in-memory job and run slices from coordinator queries —
//! these helpers do not call Chronon.
//!
//! ```rust,ignore
//! use chronon_backend::{dashboard_stats_from_jobs_and_runs, DashboardStats};
//! use chronon_core::{Job as CoreJob, Run as CoreRun};
//! use chrono::Utc;
//!
//! let jobs: Vec<CoreJob> = vec![CoreJob::new("nightly-sync", "reports.export")];
//! let runs: Vec<CoreRun> = vec![];
//! let today_start = Utc::now();
//! let stats: DashboardStats = dashboard_stats_from_jobs_and_runs(&jobs, &runs, today_start);
//! assert_eq!(stats.total_jobs, 1);
//! assert_eq!(stats.active_jobs, 1);
//! ```
//!
//! On success `stats` carries `total_jobs`, `active_jobs`, `total_runs_today`, and
//! `running_now` consumed by `chronon-app` dashboard server functions.
//!
//! ## Redact job params
//!
//! Job list/detail responses keep names and script identity visible to any
//! signed-in Chronon viewer, while script parameters and cron belong to operators
//! with `ChrononAdmin`. Call [`redact_job_params_for_non_admin`] after mapping a
//! coordinator job into a UI [`Job`] and before returning it from a server
//! function. Pass `is_admin` from the session's `ChrononAdmin` check.
//!
//! **Prerequisites:** An in-memory [`Job`] DTO (no Chronon IO in this helper).
//!
//! ```rust
//! use chronon_backend::{redact_job_params_for_non_admin, Job, JobStatus};
//!
//! let job = Job {
//!     id: "job-1".into(),
//!     name: "nightly".into(),
//!     script_name: "reports.export".into(),
//!     cron: "0 0 * * * *".into(),
//!     status: JobStatus::Active,
//!     revision: 1,
//!     last_run_at: None,
//!     next_run_at: Some("2026-01-02T00:00:00Z".into()),
//!     timezone: Some("UTC".into()),
//!     params: serde_json::json!({"token": "secret"}),
//! };
//! let redacted = redact_job_params_for_non_admin(job, false);
//! assert_eq!(redacted.params, serde_json::json!({}));
//! assert!(redacted.cron.is_empty());
//! assert_eq!(redacted.name, "nightly");
//! ```
//!
//! Non-admin callers get empty `params` / `cron` and cleared schedule fields.
//! Admins (`is_admin = true`) receive the job unchanged. Related:
//! [`redact_job_revision`] strips actor and params from revision snapshots.
//! Interactive `run_now` on platform System jobs is refused in `chronon-app`
//! (schedule-only); this helper does not implement that gate.
//!
//! ## Examples
//!
//! Start with [Validate ids](#validate-ids) or [Redact job params](#redact-job-params).
//! This crate's unit and integ suites are listed in `docs/VERIFICATION.md`.
//! Runnable host: `examples/protected-chronon-host` (auth + dashboard KPIs).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod dashboard;
mod lookup;
mod map;
mod page_query;
mod revision;
mod schedule;
mod types;
mod validate;

pub use dashboard::{
    align_run_bucket, dashboard_stats_from_jobs_and_runs, run_bucket_granularity,
    run_stats_series_from_runs, utc_start_of_day, RunBucketGranularity,
};
pub use lookup::{
    find_job_by_id, find_job_by_name, find_run_by_id, find_script_by_name,
    resolve_job_id_from_list, sort_jobs_by_name, sort_scripts_by_name,
};
pub use map::{
    backend_job_to_job, chronon_run_status_to_ui, job_status_label, model_run_to_app_run,
    normalized_params, parse_script_params, recent_run_from_model, run_status_label,
};
pub use page_query::{
    apply_jobs_page_query, apply_runs_page_query, apply_scripts_page_query,
    runs_page_needs_memory_scan,
};
pub use revision::{
    redact_job_params_for_non_admin, redact_job_revision, redact_revision_snapshot,
};
pub use schedule::{
    apply_create_schedule, apply_update_payload_to_job, build_create_job_model,
    parse_run_once_datetime, recompute_next_run_for_cron, ChrononScheduleError,
};
pub use types::{
    CreateJobRequest, CreateJobScheduleType, DashboardChartPoint, DashboardChartSeries,
    DashboardStats, Job, JobRevision, JobStatus, RecentRun, Run, RunStatus, Script, ScriptParam,
    UpdateJobRequest, JOBS_PAGE_SIZE, JOB_RUNS_PAGE_SIZE, RUNS_PAGE_SIZE, SCRIPTS_PAGE_SIZE,
};
pub use validate::{
    chronon_job_path, chronon_run_path, encode_ops_path_segment, validate_job_id,
    validate_job_id_or_name, validate_job_name, validate_run_id, validate_script_name,
    ChrononIdError, MAX_CHRONON_ID_CHARS,
};

#[cfg(test)]
#[path = "unit_tests.rs"]
mod tests;
