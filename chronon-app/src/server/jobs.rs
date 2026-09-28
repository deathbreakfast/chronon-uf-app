//! Job CRUD and revision server functions.

use leptos::prelude::*;
use orbital_paging::{Page, PageRequest};

#[cfg(feature = "ssr")]
use super::helpers::{
    apply_update_payload_to_job, backend_job_to_job, build_create_job_model,
    ensure_job_name_available, load_existing_job_for_update, persist_updated_job_config,
};
#[cfg(feature = "ssr")]
use super::query::apply_jobs_page_query;
#[cfg(feature = "ssr")]
use super::ssr_utils::{require_email_verified, require_session, resolve_job_id};
#[cfg(feature = "ssr")]
use super::CHRONON_ADMIN_PERMISSION;
use super::{CreateJobRequest, Job, JobRevision, UpdateJobRequest};
#[cfg(feature = "ssr")]
use chronon_backend::redact_job_params_for_non_admin;

/// Paginated jobs list with quick-search and structured filters.
#[uf_product_macros::server]
pub async fn get_jobs_page(
    /// `DataTable` paging/filter/search/sort request from the client.
    request: PageRequest,
) -> Result<Page<Job>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let is_admin = uf_product::permissions::has_permission(CHRONON_ADMIN_PERMISSION).await;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let mut jobs: Vec<Job> = backend
        .list_jobs()
        .await
        .into_iter()
        .map(backend_job_to_job)
        .map(|j| redact_job_params_for_non_admin(j, is_admin))
        .collect();
    chronon_backend::sort_jobs_by_name(&mut jobs);
    apply_jobs_page_query(&mut jobs, &request);

    let total_count = if request.is_first_page() {
        Some(jobs.len() as u64)
    } else {
        None
    };

    let sliced: Vec<Job> = jobs
        .into_iter()
        .skip(request.offset as usize)
        .take((request.limit + 1) as usize)
        .collect();

    Ok(Page::from_oversized(sliced, request.limit, total_count))
}

/// Get all jobs
#[uf_product_macros::server]
pub async fn get_jobs() -> Result<Vec<Job>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let is_admin = uf_product::permissions::has_permission(CHRONON_ADMIN_PERMISSION).await;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let jobs = backend.list_jobs().await;

    let jobs: Vec<Job> = jobs
        .into_iter()
        .map(backend_job_to_job)
        .map(|j| redact_job_params_for_non_admin(j, is_admin))
        .collect();

    Ok(jobs)
}

/// Create a new job and persist it through Chronon backend + DB.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn create_job(
    /// Fields describing the new job to create.
    payload: CreateJobRequest,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    chronon_backend::validate_job_name(&payload.job_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    chronon_backend::validate_script_name(&payload.script_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let job_name = payload.job_name.trim().to_string();
    let script_name = payload.script_name.trim().to_string();

    let registry = super::ssr_utils::script_registry()?;
    let descriptor = registry
        .get(&script_name)
        .ok_or_else(|| ServerFnError::new(format!("Script '{}' not found", script_name)))?;

    ensure_job_name_available(backend, &job_name).await?;
    let job = build_create_job_model(&payload, descriptor.signature_hash.to_string())
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let created_id = job.job_id.clone();
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    backend
        .upsert_job_with_valence(&valence, job)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create job: {}", e)))?;
    Ok(created_id)
}

/// Get a single job by ID or name (tries ID first, then name)
#[uf_product_macros::server]
pub async fn get_job(
    /// Job ID or job name to look up; ID is tried first, then name.
    job_id_or_name: String,
) -> Result<Option<Job>, ServerFnError> {
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let is_admin = uf_product::permissions::has_permission(CHRONON_ADMIN_PERMISSION).await;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let job = if let Some(j) = backend.get_job(&job_id_or_name).await {
        Some(j)
    } else {
        backend.get_job_by_name(&job_id_or_name).await
    };

    Ok(job
        .map(backend_job_to_job)
        .map(|j| redact_job_params_for_non_admin(j, is_admin)))
}

/// Get all revisions for a job (by ID or name)
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_job_revisions(
    /// Job ID or job name whose revisions should be listed.
    job_id_or_name: String,
) -> Result<Vec<JobRevision>, ServerFnError> {
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let actual_job_id = resolve_job_id(backend, &job_id_or_name)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id_or_name)))?;

    let revisions = backend
        .list_revisions(&actual_job_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list revisions: {}", e)))?;

    let job_revisions: Vec<JobRevision> = revisions
        .into_iter()
        .map(|r| {
            chronon_backend::redact_job_revision(JobRevision {
                revision_id: r.revision_id,
                revision_number: u32::try_from(r.revision_number).unwrap_or(0),
                changed_at: r.changed_at.to_rfc3339(),
                changed_by_actor_json: r.changed_by_actor_json,
                snapshot_json: r.snapshot_json,
            })
        })
        .collect();

    Ok(job_revisions)
}

/// Update a job's configuration (creates a new revision)
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn update_job(
    /// Job ID or job name to update.
    job_id_or_name: String,
    /// Fields to change on the job; creates a new revision.
    payload: UpdateJobRequest,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let actual_job_id = resolve_job_id(backend, &job_id_or_name)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id_or_name)))?;

    let existing_job = load_existing_job_for_update(backend, &actual_job_id).await?;
    let updated_job = apply_update_payload_to_job(existing_job, &payload)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    persist_updated_job_config(backend, &valence, &actual_job_id, updated_job).await
}
