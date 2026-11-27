//! Dashboard statistics and run trend server functions.

use leptos::prelude::*;

#[cfg(feature = "ssr")]
use super::access::{keep_viewable_runs, viewable_job_ids};
#[cfg(feature = "ssr")]
use super::ssr_utils::{require_session, resolve_job_names_for_model_runs};
use super::{DashboardChartSeries, DashboardStats, RecentRun};

/// Get dashboard statistics
#[uf_product_macros::server]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let mut jobs = backend.list_jobs().await;
    let viewable = viewable_job_ids(&jobs).await;
    jobs.retain(|j| viewable.contains(&j.job_id));

    let today_start_utc = chronon_backend::utc_start_of_day(chrono::Utc::now());

    let mut db_runs = backend
        .list_runs(None, None, 0, 500)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?;
    keep_viewable_runs(&mut db_runs, &viewable);

    Ok(chronon_backend::dashboard_stats_from_jobs_and_runs(
        &jobs,
        &db_runs,
        today_start_utc,
    ))
}

/// Get recent runs for dashboard
#[uf_product_macros::server]
pub async fn get_recent_runs(
    /// Maximum number of recent runs to return.
    limit: u32,
) -> Result<Vec<RecentRun>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let mut db_runs = backend
        .list_runs(None, None, 0, limit as usize)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?;
    keep_viewable_runs(
        &mut db_runs,
        &viewable_job_ids(&backend.list_jobs().await).await,
    );

    let job_names = resolve_job_names_for_model_runs(backend, &db_runs).await;

    let recent: Vec<RecentRun> = db_runs
        .iter()
        .map(|r| {
            let job_name = r
                .job_id
                .clone()
                .and_then(|jid| job_names.get(&jid).cloned())
                .unwrap_or_else(|| r.script_name.clone());

            chronon_backend::recent_run_from_model(r, job_name)
        })
        .collect();

    Ok(recent)
}

/// Time-series run outcome counts for the dashboard chart.
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
