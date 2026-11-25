//! Job CRUD and revision server functions.

use leptos::prelude::*;
use orbital_paging::{Page, PageRequest};

#[cfg(feature = "ssr")]
use super::access::{held_on, require_job, Held};
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

/// What the signed-in person may do on this host, and with one job.
///
/// `can_create` comes from `ChrononAdmin`; the rest come from the job's own access
/// (View, Run, Edit), checked as the session actor.
#[allow(clippy::struct_excessive_bools)] // one flag per thing the page can offer
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChrononJobAccess {
    /// May create jobs.
    pub can_create: bool,
    /// May see the job's runs, parameters and output.
    pub can_view: bool,
    /// May change the job's schedule and configuration, and pause or resume it.
    pub can_edit: bool,
    /// May start a run with Run now (System-owned jobs still refuse it).
    pub can_run: bool,
    /// The reader's role on the job ("Owner", "Can run", "No access", …); empty without a job.
    pub role: String,
}

/// Whether this person may create jobs (holds `ChrononAdmin`).
#[must_use]
pub const fn can_create_job(is_admin: bool) -> bool {
    is_admin
}

/// One choice in the create form's "Owned by" field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedByOption {
    /// Gauge group id to make an owner; `None` means "Only me".
    pub group_id: Option<String>,
    /// What the field shows, such as "Platform ops (team)".
    pub label: String,
}

/// The create form's "Owned by" choices and the one it starts on.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OwnedByOptions {
    /// The creator's teams, then the Chronon admin group, then "Only me".
    pub options: Vec<OwnedByOption>,
    /// The creator's team when they're on exactly one, otherwise the admin group.
    pub default_group_id: Option<String>,
}

/// The wire shape of one job for this reader: params stay hidden without View, and the
/// reader's role rides along for the jobs list.
#[cfg(feature = "ssr")]
fn job_for_reader(job: Job, held: Held) -> Job {
    let mut job = job;
    if !held.view {
        job.params = serde_json::json!({});
        job.last_run_at = None;
        job.last_run_status = None;
        job.last_run_attempts = None;
    }
    job.your_access = held.label().to_string();
    job
}

/// Maps a coordinator job and fills Last run from `runs` (which may hold other jobs' runs).
#[cfg(feature = "ssr")]
fn with_last_run(
    job: chronon_coordinator::models::Job,
    runs: &[chronon_coordinator::models::Run],
) -> Job {
    let mut job = backend_job_to_job(job);
    chronon_backend::fill_last_run(&mut job, runs);
    job
}

/// Map stored jobs to wire jobs for this reader, with Last run filled from `runs`.
#[cfg(feature = "ssr")]
async fn jobs_for_reader(
    jobs: Vec<chronon_coordinator::models::Job>,
    runs: &[chronon_coordinator::models::Run],
) -> Vec<Job> {
    let mut out = Vec::with_capacity(jobs.len());
    for job in jobs {
        let held = held_on(&job).await;
        out.push(job_for_reader(with_last_run(job, runs), held));
    }
    out
}

/// Paginated jobs list with quick-search and structured filters.
#[uf_product_macros::server]
pub async fn get_jobs_page(
    /// `DataTable` paging/filter/search/sort request from the client.
    request: PageRequest,
) -> Result<Page<Job>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let runs = super::runs::scan_runs(backend, None).await?;
    // Every job is listed with the reader's role, so people can see what exists and ask.
    let mut jobs: Vec<Job> = jobs_for_reader(backend.list_jobs().await, &runs).await;
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
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let runs = super::runs::scan_runs(backend, None).await?;
    Ok(jobs_for_reader(backend.list_jobs().await, &runs).await)
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

    // The job's access exists before the job does: the creator maintains it, and the chosen
    // group owns it. Both writes run as the session actor.
    let owned_by = payload
        .owned_by
        .as_deref()
        .map(str::trim)
        .filter(|g| !g.is_empty());
    if let Some(group) = owned_by {
        let offered = owned_by_options_for(&ctx, &valence).await;
        if !offered
            .options
            .iter()
            .any(|o| o.group_id.as_deref() == Some(group))
        {
            return Err(ServerFnError::new(format!(
                "You can't make '{group}' the owner of this job"
            )));
        }
    }
    crate::permissions::ensure_created_job_bundle(&valence, &created_id, &job_name, owned_by)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to set up the job's access: {e}")))?;

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
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let job = if let Some(j) = backend.get_job(&job_id_or_name).await {
        Some(j)
    } else {
        backend.get_job_by_name(&job_id_or_name).await
    };
    // The name, owner and schedule stay readable without View, so people can ask for it.
    let Some(job) = job else {
        return Ok(None);
    };
    let held = held_on(&job).await;
    let runs = super::runs::scan_runs(backend, Some(&job.job_id)).await?;
    Ok(Some(job_for_reader(with_last_run(job, &runs), held)))
}

/// Get all revisions for a job (by ID or name), newest first. Needs View on the job.
///
/// Each revision names who saved it, sums up what it changed, and carries its settings
/// (parameters included) as `key: value` lines for the History diff. The raw actor JSON
/// and the snapshot's actor and params stay on the server.
#[uf_product_macros::server]
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

    let job = load_existing_job_for_update(backend, &actual_job_id).await?;
    require_job(&job, "View").await?;

    let revisions = backend
        .list_revisions(&actual_job_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list revisions: {}", e)))?;

    let job_revisions: Vec<JobRevision> = revisions
        .into_iter()
        .map(|r| JobRevision {
            revision_id: r.revision_id,
            revision_number: u32::try_from(r.revision_number).unwrap_or(0),
            changed_at: r.changed_at.to_rfc3339(),
            changed_by_actor_json: r.changed_by_actor_json,
            snapshot_json: r.snapshot_json,
            saved_by: String::new(),
            summary: String::new(),
            settings_text: String::new(),
        })
        .collect();

    // saved_by and settings_text carry what History shows; raw actor identity stays here.
    Ok(chronon_backend::describe_revisions(job_revisions)
        .into_iter()
        .map(|mut r| {
            r.changed_by_actor_json = serde_json::Value::Null;
            r.snapshot_json = chronon_backend::redact_revision_snapshot(r.snapshot_json);
            r
        })
        .collect())
}

/// Update a job's configuration (creates a new revision).
///
/// When `payload.expected_revision` is behind the stored revision nothing is written and
/// the result is [`UpdateJobOutcome::Conflict`] naming who saved last. Needs Edit on the job.
#[uf_product_macros::server]
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
/// revision recorded against the signed-in person. Needs Edit on the job.
#[uf_product_macros::server]
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
    require_job(&existing_job, "Edit").await?;
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
/// Needs View on the job, like [`get_job_revisions`]: the pauser is a revision actor.
#[uf_product_macros::server]
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
    require_job(&job, "View").await?;
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

/// What the signed-in person may do on this host, and with `job_id_or_name` when given.
#[uf_product_macros::server]
pub async fn get_job_access(
    /// Job ID or job name; `None` asks only whether the reader may create jobs.
    job_id_or_name: Option<String>,
) -> Result<ChrononJobAccess, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let is_admin = uf_product::permissions::has_permission(CHRONON_ADMIN_PERMISSION).await;
    let mut access = ChrononJobAccess {
        can_create: can_create_job(is_admin),
        ..ChrononJobAccess::default()
    };
    let Some(job_id_or_name) = job_id_or_name else {
        return Ok(access);
    };
    chronon_backend::validate_job_id_or_name(&job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let Some(actual_job_id) = resolve_job_id(backend, &job_id_or_name).await? else {
        return Ok(access);
    };
    let job = load_existing_job_for_update(backend, &actual_job_id).await?;
    let held = held_on(&job).await;
    access.can_view = held.view;
    access.can_run = held.run;
    access.can_edit = held.edit;
    access.role = held.label().to_string();
    Ok(access)
}

/// The "Owned by" choices for a job this person creates.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn get_owned_by_options() -> Result<OwnedByOptions, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(owned_by_options_for(&ctx, &valence).await)
}

/// The creator's managed team groups (teams mirrored into Gauge), the admin group, and
/// "Only me". Reads run as the session actor; groups the creator can't read are left out.
#[cfg(feature = "ssr")]
async fn owned_by_options_for(ctx: &higgs::Higgs, valence: &valence::Valence) -> OwnedByOptions {
    use valence::Model as _;

    let me = ctx
        .session_user_id()
        .map(|id| id.trim_start_matches("user:").to_string())
        .unwrap_or_default();
    let mut teams = Vec::new();
    if let Ok(groups) = gauge::service::list_groups(valence, None).await {
        for group in groups {
            let member = group
                .members
                .iter()
                .any(|m| m.id.trim_start_matches("user:") == me);
            if !member {
                continue;
            }
            let managed = gauge::generated::PermissionGroup::get(&group.id, valence, valence::use_!(r"When someone **creates a Chronon job**, we **load each Gauge group they belong to** to see whether it is a team, so the job's **Owned by** field can offer their teams."))
                .await
                .ok()
                .flatten()
                .is_some_and(|g| g.managed_by().is_some());
            if managed {
                teams.push(OwnedByOption {
                    group_id: Some(group.id),
                    label: format!("{} (team)", group.name),
                });
            }
        }
    }
    let admin = super::access::admin_group();
    let default_group_id = if teams.len() == 1 {
        teams[0].group_id.clone()
    } else {
        admin.as_ref().map(|a| a.id.clone())
    };
    let mut options = teams;
    if let Some(admin) = admin {
        options.push(OwnedByOption {
            group_id: Some(admin.id),
            label: admin.label,
        });
    }
    options.push(OwnedByOption {
        group_id: None,
        label: "Only me".to_string(),
    });
    OwnedByOptions {
        options,
        default_group_id,
    }
}

/// Lists the worker pools a job can be pinned to on this host.
#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn list_chronon_job_pools() -> Result<Vec<ChrononPoolPickRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_email_verified().await?;
    Ok(offered_pools())
}
