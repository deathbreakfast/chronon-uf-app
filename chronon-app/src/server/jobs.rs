//! Job CRUD and revision server functions.

use leptos::prelude::*;
use orbital_paging::{Page, PageRequest};

#[cfg(feature = "ssr")]
use super::helpers::{
    actor_display_names, apply_update_payload_to_job, backend_job_to_job, build_create_job_model,
    ensure_job_name_available, load_existing_job_for_update, persist_updated_job_config,
};
#[cfg(feature = "ssr")]
use super::query::apply_jobs_page_query;
#[cfg(feature = "ssr")]
use super::ssr_utils::{offered_pools, require_email_verified, require_session, resolve_job_id};
#[cfg(feature = "ssr")]
use super::CHRONON_ADMIN_PERMISSION;
use super::{ChrononPoolPickRow, CreateJobRequest, Job, JobRevision, UpdateJobRequest};
use chronon_backend::{PauseInfo, SchedulePreview, UpdateJobOutcome};
use serde::{Deserialize, Serialize};

/// What the signed-in person may do with Chronon jobs.
///
/// Each flag comes from one helper ([`can_create_job`], [`can_edit_job`], [`can_run_job`]),
/// so per-job access can replace today's `ChrononAdmin` check in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChrononJobAccess {
    /// May create jobs.
    pub can_create: bool,
    /// May change a job's schedule and configuration, and pause or resume it.
    pub can_edit: bool,
    /// May start a run with Run now (System-owned jobs still refuse it).
    pub can_run: bool,
}

/// Whether this person may create jobs (today: holds `ChrononAdmin`).
#[must_use]
pub const fn can_create_job(is_admin: bool) -> bool {
    is_admin
}

/// Whether this person may edit, pause or resume jobs (today: holds `ChrononAdmin`).
#[must_use]
pub const fn can_edit_job(is_admin: bool) -> bool {
    is_admin
}

/// Whether this person may use Run now (today: holds `ChrononAdmin`).
#[must_use]
pub const fn can_run_job(is_admin: bool) -> bool {
    is_admin
}

/// Non-admin wire shape: params, cron, the schedule sentence and the owner stay hidden.
#[cfg(feature = "ssr")]
fn redact_job_params_for_non_admin(job: Job, is_admin: bool) -> Job {
    let mut job = chronon_backend::redact_job_params_for_non_admin(job, is_admin);
    if !is_admin {
        job.schedule_text = String::new();
        job.owner_label = String::new();
    }
    job
}

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

    chronon_backend::resolve_job_pool(payload.pool.as_deref(), None, &offered_pools())
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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

/// Update a job's configuration (creates a new revision).
///
/// When `payload.expected_revision` is behind the stored revision nothing is written and
/// the result is [`UpdateJobOutcome::Conflict`] naming who saved last.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn update_job(
    /// Job ID or job name to update.
    job_id_or_name: String,
    /// Fields to change on the job; creates a new revision.
    payload: UpdateJobRequest,
) -> Result<UpdateJobOutcome, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    save_job_update(&ctx, &job_id_or_name, |_| Ok(payload)).await
}

/// Pause or resume a job. Saves through the same path as [`update_job`], so it makes a
/// revision recorded against the signed-in person.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn set_job_paused(
    /// Job ID or job name to pause or resume.
    job_id_or_name: String,
    /// `true` pauses, `false` resumes.
    paused: bool,
) -> Result<UpdateJobOutcome, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    save_job_update(&ctx, &job_id_or_name, |existing| {
        Ok(UpdateJobRequest {
            job_name: existing.job_name.clone(),
            cron_expr: existing.cron_expr.clone(),
            timezone: existing.timezone.clone(),
            params: existing.params_json.clone(),
            enabled: !paused,
            pool: None,
            expected_revision: None,
        })
    })
    .await
}

/// Loads the job, refuses a stale save, applies the payload and persists it.
#[cfg(feature = "ssr")]
async fn save_job_update(
    ctx: &higgs::Higgs,
    job_id_or_name: &str,
    build: impl FnOnce(&chronon_coordinator::models::Job) -> Result<UpdateJobRequest, ServerFnError>,
) -> Result<UpdateJobOutcome, ServerFnError> {
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let actual_job_id = resolve_job_id(backend, job_id_or_name)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id_or_name)))?;

    let existing_job = load_existing_job_for_update(backend, &actual_job_id).await?;
    let payload = build(&existing_job)?;
    let current = u32::try_from(existing_job.current_revision).unwrap_or(u32::MAX);
    if chronon_backend::revision_conflict(payload.expected_revision, current) {
        let latest = backend
            .list_revisions(&actual_job_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to list revisions: {}", e)))?
            .into_iter()
            .max_by_key(|r| r.revision_number);
        let (actor, at) = latest.map_or_else(
            || {
                (
                    existing_job.actor_json.clone(),
                    existing_job.updated_at.to_rfc3339(),
                )
            },
            |r| (r.changed_by_actor_json, r.changed_at.to_rfc3339()),
        );
        let valence = ctx
            .valence()
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let names = actor_display_names(&valence, [&actor]).await;
        let by = chronon_backend::actor_label_named(&actor, &names);
        return Ok(UpdateJobOutcome::Conflict {
            revision: current,
            by,
            at,
        });
    }
    chronon_backend::resolve_job_pool(
        payload.pool.as_deref(),
        existing_job.pool.as_deref(),
        &offered_pools(),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    let updated_job = apply_update_payload_to_job(existing_job, &payload)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    persist_updated_job_config(backend, &valence, &actual_job_id, updated_job).await?;
    let saved = load_existing_job_for_update(backend, &actual_job_id).await?;
    Ok(UpdateJobOutcome::Saved {
        revision: u32::try_from(saved.current_revision).unwrap_or(u32::MAX),
        next_run_at: saved.next_run_at.map(|t| t.to_rfc3339()),
    })
}

/// Sentence, next five runs and any field error for a schedule being edited.
#[uf_product_macros::server]
pub async fn preview_schedule(
    /// Cron expression as typed.
    cron: String,
    /// IANA timezone name.
    tz: String,
) -> Result<SchedulePreview, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    Ok(chronon_backend::preview_schedule_at(
        &cron,
        &tz,
        chrono::Utc::now(),
        5,
    ))
}

/// Who paused a job, when, and which runs it skipped; `None` for a running job.
///
/// Admin-only, like [`get_job_revisions`]: the pauser is a revision actor, and the
/// skipped times would rebuild the schedule non-admins can't see.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_job_pause_info(
    /// Job ID or job name.
    job_id_or_name: String,
) -> Result<Option<PauseInfo>, ServerFnError> {
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let Some(actual_job_id) = resolve_job_id(backend, &job_id_or_name).await? else {
        return Ok(None);
    };
    let job = load_existing_job_for_update(backend, &actual_job_id).await?;
    if job.enabled {
        return Ok(None);
    }
    let revisions: Vec<_> = backend
        .list_revisions(&actual_job_id)
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
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let names = actor_display_names(&valence, revisions.iter().map(|r| &r.2)).await;
    Ok(chronon_backend::pause_info_from_revisions(
        &job,
        &revisions,
        chrono::Utc::now(),
        &names,
    ))
}

/// What the signed-in person may do with jobs on this host.
#[uf_product_macros::server]
pub async fn get_job_access() -> Result<ChrononJobAccess, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let is_admin = uf_product::permissions::has_permission(CHRONON_ADMIN_PERMISSION).await;
    Ok(ChrononJobAccess {
        can_create: can_create_job(is_admin),
        can_edit: can_edit_job(is_admin),
        can_run: can_run_job(is_admin),
    })
}

/// Lists the worker pools a job can be pinned to on this host.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn list_chronon_job_pools() -> Result<Vec<ChrononPoolPickRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    Ok(offered_pools())
}
