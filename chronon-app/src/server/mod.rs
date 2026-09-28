//! Chronon app server functions and UI-facing DTO exports.

#![allow(clippy::uninlined_format_args)]
//!
//! DTOs and pure mapping helpers live in [`chronon_backend`] so contracts stay
//! unit/integration-testable without the host UI graph. Server functions run on
//! SSR only and use [`higgs::Higgs::from_request()`] plus [`ssr_utils::require_session`]
//! on every endpoint. Mutators `create_job` / `update_job` / `run_job_now` require
//! Gauge permission `ChrononAdmin`. Job list/detail stay session-gated but omit
//! job `params` unless the session holds `ChrononAdmin`. Run history, revisions,
//! and dashboard run aggregates require `ChrononAdmin`. Job CRUD additionally
//! mirrors the UI email-verification gate via [`ssr_utils::require_email_verified`].
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

mod dashboard;
mod helpers;
mod jobs;
pub mod query;
mod runs;
mod scripts;
#[cfg(feature = "ssr")]
mod ssr_utils;
mod types;

pub use dashboard::{get_dashboard_stats, get_recent_runs, get_run_stats_series};
pub use jobs::{create_job, get_job, get_job_revisions, get_jobs, get_jobs_page, update_job};
pub use runs::{get_job_runs_page, get_run, get_runs, get_runs_page, run_job_now};
pub use scripts::{get_scripts, get_scripts_page};
pub use types::*;

/// Permission name required for Chronon admin mutators
/// (manifest: [`crate::permissions::ChrononPermission::ChrononAdmin`]).
pub const CHRONON_ADMIN_PERMISSION: &str = "ChrononAdmin";
