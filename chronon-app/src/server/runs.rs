//! Run history and manual execution server functions.

use leptos::prelude::*;
use orbital_paging::{Page, PageRequest};

#[cfg(feature = "ssr")]
use super::query::{apply_runs_page_query, runs_page_needs_memory_scan};
#[cfg(feature = "ssr")]
use super::ssr_utils::{
    model_run_to_app_run, require_session, resolve_job_id, resolve_job_names_for_model_runs,
};
use super::Run;

#[cfg(feature = "ssr")]
use chronon_coordinator::ChrononCoordinatorBackend;

/// When quick-search or structured filters are active, scan a bounded slice in memory (0.1.n).
#[cfg(feature = "ssr")]
const RUNS_FILTER_SCAN_CAP: usize = 5000;

#[cfg(feature = "ssr")]
async fn fetch_db_runs_for_page(
    backend: &dyn ChrononCoordinatorBackend,
    job_id: Option<&str>,
    request: &PageRequest,
) -> Result<Vec<chronon_coordinator::models::Run>, ServerFnError> {
    if runs_page_needs_memory_scan(request) {
        backend
            .list_runs(job_id, None, 0, RUNS_FILTER_SCAN_CAP)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))
    } else {
        let offset = request.offset as usize;
        let limit = (request.limit + 1) as usize;
        backend
            .list_runs(job_id, None, offset, limit)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))
    }
}

#[cfg(feature = "ssr")]
async fn runs_page_from_request(
    backend: &dyn ChrononCoordinatorBackend,
    job_id: Option<&str>,
    job_name_override: Option<String>,
    request: PageRequest,
) -> Result<Page<Run>, ServerFnError> {
    let needs_memory_scan = runs_page_needs_memory_scan(&request);
    let db_runs = fetch_db_runs_for_page(backend, job_id, &request).await?;

    let job_names = if job_name_override.is_some() {
        std::collections::HashMap::new()
    } else {
        resolve_job_names_for_model_runs(backend, &db_runs).await
    };

    let mut runs: Vec<Run> = db_runs
        .into_iter()
        .map(|r| {
            let job_name = job_name_override.clone().unwrap_or_else(|| {
                r.job_id
                    .clone()
                    .and_then(|jid| job_names.get(&jid).cloned())
                    .unwrap_or_else(|| r.script_name.clone())
            });
            model_run_to_app_run(r, job_name)
        })
        .collect();

    if needs_memory_scan {
        apply_runs_page_query(&mut runs, &request);
        let total_count = if request.is_first_page() {
            Some(runs.len() as u64)
        } else {
            None
        };
        let sliced: Vec<Run> = runs
            .into_iter()
            .skip(request.offset as usize)
            .take((request.limit + 1) as usize)
            .collect();
        Ok(Page::from_oversized(sliced, request.limit, total_count))
    } else {
        // DB-offset path has no cheap total; still mark empty first pages so the
        // DataTable empty overlay does not stay in "waiting for total" limbo.
        let total_count = if request.is_first_page() && runs.is_empty() {
            Some(0)
        } else {
            None
        };
        Ok(Page::from_oversized(runs, request.limit, total_count))
    }
}

/// Get runs, optionally filtered by job (by ID or name)
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_runs(
    /// Optional job ID or job name to restrict results to a single job's runs.
    job_id_or_name: Option<String>,
) -> Result<Vec<Run>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let db_runs = if let Some(jid_or_name) = &job_id_or_name {
        chronon_backend::validate_job_id_or_name(jid_or_name)
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let actual_job_id = resolve_job_id(backend, jid_or_name)
            .await?
            .ok_or_else(|| ServerFnError::new(format!("Job {} not found", jid_or_name)))?;
        backend
            .list_runs(Some(&actual_job_id), None, 0, 500)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?
    } else {
        backend
            .list_runs(None, None, 0, 100)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?
    };

    let job_names = resolve_job_names_for_model_runs(backend, &db_runs).await;

    let runs: Vec<Run> = db_runs
        .into_iter()
        .map(|r| {
            let job_name = r
                .job_id
                .clone()
                .and_then(|jid| job_names.get(&jid).cloned())
                .unwrap_or_else(|| r.script_name.clone());

            model_run_to_app_run(r, job_name)
        })
        .collect();

    Ok(runs)
}

/// Paginated run history with quick-search and structured filters.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_runs_page(
    /// `DataTable` paging/filter/search/sort request from the client.
    request: PageRequest,
) -> Result<Page<Run>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    runs_page_from_request(backend, None, None, request).await
}

/// Get a single run by ID
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_run(
    /// Unique identifier of the run to look up.
    run_id: String,
) -> Result<Option<Run>, ServerFnError> {
    chronon_backend::validate_run_id(&run_id).map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let db_run = backend
        .get_run(&run_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get run: {}", e)))?;

    let out = match db_run {
        None => None,
        Some(r) => {
            let jid = r.job_id.clone().unwrap_or_default();
            let job_name = if jid.is_empty() {
                r.script_name.clone()
            } else if let Some(j) = backend.get_job(&jid).await {
                j.job_name
            } else {
                r.script_name.clone()
            };
            Some(model_run_to_app_run(r, job_name))
        }
    };

    Ok(out)
}

/// Trigger immediate execution of a job.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn run_job_now(
    /// Unique identifier of the job to run immediately.
    job_id: String,
    /// Optional JSON parameter overrides for this run.
    params: Option<serde_json::Value>,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    chronon_backend::validate_job_id(&job_id).map_err(|e| ServerFnError::new(e.to_string()))?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    // Published Chronon runtime still inherits stored job actor on run_now. Refuse
    // interactive run_now for System-shaped platform jobs so ChrononAdmin cannot
    // fire deletion/TTL scripts as System with caller-controlled params.
    if let Some(job) = backend.get_job(&job_id).await {
        if chronon_coordinator::validate_external_job_actor_json(&job.actor_json).is_err() {
            return Err(ServerFnError::new(
                "Platform System jobs cannot be triggered via run_now; they run on schedule only",
            ));
        }
    } else if let Some(job) = backend.get_job_by_name(&job_id).await {
        if chronon_coordinator::validate_external_job_actor_json(&job.actor_json).is_err() {
            return Err(ServerFnError::new(
                "Platform System jobs cannot be triggered via run_now; they run on schedule only",
            ));
        }
    }

    backend
        .run_now_with_params(&job_id, params)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run job: {}", e)))
}

/// Paginated runs for a specific job with quick-search and structured filters.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_job_runs_page(
    /// Job ID or job name whose runs should be listed.
    job_id_or_name: String,
    /// `DataTable` paging/filter/search/sort request from the client.
    request: PageRequest,
) -> Result<Page<Run>, ServerFnError> {
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let actual_job_id = resolve_job_id(backend, &job_id_or_name)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id_or_name)))?;

    let job_name = backend
        .get_job(&actual_job_id)
        .await
        .map_or_else(|| "Unknown".to_string(), |j| j.job_name);

    runs_page_from_request(
        backend,
        Some(actual_job_id.as_str()),
        Some(job_name),
        request,
    )
    .await
}
