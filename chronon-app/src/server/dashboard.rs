//! Dashboard server functions: stats, Needs attention, Coming up, recent runs, and the run
//! trend chart on the Runs page. Each one covers only the jobs the reader can view.

use leptos::prelude::*;

#[cfg(feature = "ssr")]
use super::access::{keep_viewable_runs, viewable_job_ids};
#[cfg(feature = "ssr")]
use super::helpers::actor_display_names;
#[cfg(feature = "ssr")]
use super::ssr_utils::require_session;
use super::{Attention, ComingUpRun, DashboardChartSeries, DashboardStats, RunSummary};

/// Runs the dashboard reads per call. Folding retries needs every attempt in view, so the
/// dashboard scans a bounded slice of the newest runs (0.1.n).
#[cfg(feature = "ssr")]
const DASHBOARD_RUN_SCAN: usize = 5000;

/// The jobs the reader can view and their newest runs; everything else stays hidden.
#[cfg(feature = "ssr")]
async fn viewable_jobs_and_runs(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
) -> Result<
    (
        Vec<chronon_coordinator::models::Job>,
        Vec<chronon_coordinator::models::Run>,
    ),
    ServerFnError,
> {
    let mut jobs = backend.list_jobs().await;
    let viewable = viewable_job_ids(&jobs).await;
    jobs.retain(|j| viewable.contains(&j.job_id));
    let mut runs = backend
        .list_runs(None, None, 0, DASHBOARD_RUN_SCAN)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?;
    keep_viewable_runs(&mut runs, &viewable);
    Ok((jobs, runs))
}

/// Get dashboard statistics (retries folded in).
#[uf_product_macros::server]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let (jobs, db_runs) = viewable_jobs_and_runs(backend).await?;

    Ok(chronon_backend::dashboard_stats_from_jobs_and_runs(
        &jobs,
        &db_runs,
        chrono::Utc::now(),
    ))
}

/// The dashboard's Needs attention list: jobs whose last run failed (retries folded in),
/// paused jobs with who paused them, and jobs that missed a run.
#[uf_product_macros::server]
pub async fn get_attention() -> Result<Attention, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let (jobs, db_runs) = viewable_jobs_and_runs(backend).await?;
    let now = chrono::Utc::now();

    // Who paused each paused job comes from its revisions, by display name where known.
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let mut pauses = std::collections::HashMap::new();
    for job in jobs.iter().filter(|j| !j.enabled) {
        let revisions: Vec<_> = backend
            .list_revisions(&job.job_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list revisions: {}", e)))?
            .into_iter()
            .map(|r| {
                (
                    u32::try_from(r.revision_number).unwrap_or(0),
                    r.changed_at,
                    r.changed_by_actor_json,
                    r.snapshot_json,
                )
            })
            .collect();
        let names = actor_display_names(&valence, revisions.iter().map(|r| &r.2)).await;
        if let Some(info) = chronon_backend::pause_info_from_revisions(job, &revisions, now, &names)
        {
            pauses.insert(job.job_id.clone(), info);
        }
    }

    Ok(chronon_backend::attention_from(
        &jobs,
        &db_runs,
        &pauses,
        now,
        chronon_backend::scheduler_tick(),
    ))
}

/// The next `limit` runs across every active job, soonest first, with schedules in words.
#[uf_product_macros::server]
pub async fn get_coming_up(
    /// How many upcoming runs to return.
    limit: u32,
) -> Result<Vec<ComingUpRun>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let (jobs, db_runs) = viewable_jobs_and_runs(backend).await?;

    Ok(chronon_backend::coming_up_from(
        &jobs,
        &db_runs,
        chrono::Utc::now(),
        limit as usize,
    ))
}

/// The newest runs for the dashboard, retries folded in.
#[uf_product_macros::server]
pub async fn get_recent_runs(
    /// Maximum number of recent runs to return.
    limit: u32,
) -> Result<Vec<RunSummary>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let (jobs, db_runs) = viewable_jobs_and_runs(backend).await?;
    let names = super::runs::people_names(&ctx, &db_runs).await?;
    let mut rows = chronon_backend::fold_runs(&db_runs, &jobs, &names);
    rows.truncate(limit as usize);
    Ok(rows)
}

/// Time-series run outcome counts for the Runs page chart.
///
/// Buckets runs into hourly (≤24h range) or daily buckets. Uses a bounded
/// backend fetch (0.1.n limitation).
#[uf_product_macros::server]
pub async fn get_run_stats_series(
    /// Width of the trailing time window, in seconds, to aggregate run outcomes over.
    range_secs: i64,
) -> Result<Vec<DashboardChartSeries>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let now = chrono::Utc::now();
    let mut db_runs = backend
        .list_runs(None, None, 0, 50_000)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?;
    keep_viewable_runs(
        &mut db_runs,
        &viewable_job_ids(&backend.list_jobs().await).await,
    );

    Ok(chronon_backend::run_stats_series_from_runs(
        &db_runs, now, range_secs,
    ))
}
