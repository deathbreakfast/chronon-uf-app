//! Chronon app server functions and UI-facing DTO exports.

#![allow(clippy::uninlined_format_args)]
//!
//! DTOs and pure mapping helpers live in [`chronon_backend`] so contracts stay
//! unit/integration-testable without the host UI graph. Server functions run on
//! SSR only and use [`higgs::Higgs::from_request()`] plus [`ssr_utils::require_session`]
//! on every endpoint. Each job has its own access, checked as the session actor through
//! `uf_product::access::require`: View for its runs, revisions and parameters, Run for
//! `run_job_now` (Run now and Run again), Edit for `update_job` and `set_job_paused`.
//! `create_job` needs Gauge permission `ChrononAdmin`. The jobs list shows every job with
//! the reader's role and hides parameters without View; run lists and dashboard
//! aggregates keep only jobs the reader can view. Job CRUD additionally mirrors the UI
//! email-verification gate via [`ssr_utils::require_email_verified`].
//!
//! ## Errors
//!
//! Fallible ops return [`ServerFnError`](leptos::prelude::ServerFnError) (Leptos
//! boundary). Blank, oversized, and path-unsafe ids are rejected by
//! `chronon_backend::validate_*` as [`chronon_backend::ChrononIdError`] and mapped
//! with operation context. Detail hrefs use `chronon_backend::chronon_*_path`
//! helpers so Orbital `paths::*` format strings cannot smuggle extra segments.
//! Missing session, missing Chronon coordinator context, and coordinator IO
//! failures are also `ServerFnError` strings at this boundary.
//! A job pool the host's [`chronon_backend::ChrononPoolProvider`] does not offer
//! fails create/update with `Pool '…' is not offered by this host`.

#[cfg(feature = "ssr")]
mod access;
mod dashboard;
mod helpers;
mod jobs;
#[cfg(feature = "ssr")]
mod people;
pub mod query;
mod runs;
mod scripts;
#[cfg(feature = "ssr")]
mod ssr_utils;
mod types;

#[cfg(feature = "ssr")]
pub use access::ChrononAdminGroup;
pub use dashboard::{
    get_attention, get_coming_up, get_dashboard_stats, get_recent_runs, get_run_stats_series,
};
pub use jobs::{
    can_create_job, create_job, get_job, get_job_access, get_job_pause_info, get_job_revisions,
    get_jobs, get_jobs_page, get_owned_by_options, list_chronon_job_pools, preview_schedule,
    set_job_paused, update_job, ChrononJobAccess, OwnedByOption, OwnedByOptions,
};
pub use runs::{
    get_job_runs_page, get_job_week, get_run, get_run_attempts, get_run_detail, get_run_now_facts,
    get_runs, get_runs_caption, get_runs_page, run_job_now,
};
pub use scripts::{get_scripts, get_scripts_page};
pub use types::*;

/// Permission name required to create jobs
/// (manifest: [`crate::permissions::ChrononPermission::ChrononAdmin`]).
pub const CHRONON_ADMIN_PERMISSION: &str = "ChrononAdmin";
