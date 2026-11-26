//! Run history and manual execution server functions.
//!
//! Run lists return [`RunSummary`] rows: a first attempt with its retries folded in, so a
//! failed run that retried three times is one Failed row with four attempts.

use leptos::prelude::*;
use orbital_paging::{Page, PageRequest};

#[cfg(feature = "ssr")]
use super::ssr_utils::{
    model_run_to_app_run, require_session, resolve_job_id, resolve_job_names_for_model_runs,
};
use super::{JobWeek, Run, RunAttempt, RunDetail, RunNowFacts, RunSummary};

#[cfg(feature = "ssr")]
use super::access::{can_on, keep_viewable_runs, require_job, viewable_job_ids};
#[cfg(feature = "ssr")]
use chronon_coordinator::ChrononCoordinatorBackend;

/// Runs read per list call. Folding retries needs every attempt in view, so lists scan a
/// bounded slice in memory (0.1.n).
#[cfg(feature = "ssr")]
const RUNS_FILTER_SCAN_CAP: usize = 5000;

/// Whether this session may start `job`: Run on the job, as the session actor.
#[cfg(feature = "ssr")]
async fn can_run_job(job: &chronon_coordinator::models::Job) -> bool {
    can_on(job, "Run").await
}

/// Load a job by id and require `action` on it.
#[cfg(feature = "ssr")]
async fn require_job_id_action(
    backend: &dyn ChrononCoordinatorBackend,
    job_id: &str,
    action: &str,
) -> Result<chronon_coordinator::models::Job, ServerFnError> {
    let job = backend
        .get_job(job_id)
        .await
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id)))?;
    require_job(&job, action).await?;
    Ok(job)
}

/// Require View on the job a run belongs to. A run with no job is refused.
#[cfg(feature = "ssr")]
async fn require_run_view(
    backend: &dyn ChrononCoordinatorBackend,
    run: &chronon_coordinator::models::Run,
) -> Result<Option<chronon_coordinator::models::Job>, ServerFnError> {
    match run.job_id.as_deref().filter(|j| !j.is_empty()) {
        Some(job_id) => match backend.get_job(job_id).await {
            Some(job) => {
                require_job(&job, "View").await?;
                Ok(Some(job))
            }
            None => Err(ServerFnError::new(format!("Job {} not found", job_id))),
        },
        None => Err(ServerFnError::new("This run has no job to check access on")),
    }
}

#[cfg(feature = "ssr")]
pub(super) async fn scan_runs(
    backend: &dyn ChrononCoordinatorBackend,
    job_id: Option<&str>,
) -> Result<Vec<chronon_coordinator::models::Run>, ServerFnError> {
    backend
        .list_runs(job_id, None, 0, RUNS_FILTER_SCAN_CAP)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))
}

/// Display names of the people in `runs`' actors, read as the session's actor.
#[cfg(feature = "ssr")]
async fn people_names(
    ctx: &higgs::Higgs,
    runs: &[chronon_coordinator::models::Run],
) -> Result<std::collections::HashMap<String, String>, ServerFnError> {
    let ids = chronon_backend::run_now_user_ids(runs);
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    super::people::display_names(&valence, &ids).await
}

#[cfg(feature = "ssr")]
async fn summaries_page(
    ctx: &higgs::Higgs,
    backend: &dyn ChrononCoordinatorBackend,
    job_id: Option<&str>,
    request: PageRequest,
) -> Result<Page<RunSummary>, ServerFnError> {
    let mut runs = scan_runs(backend, job_id).await?;
    let jobs = backend.list_jobs().await;
    keep_viewable_runs(&mut runs, &viewable_job_ids(&jobs).await);
    let names = people_names(ctx, &runs).await?;
    let mut rows = chronon_backend::fold_runs(&runs, &jobs, &names);
    chronon_backend::apply_run_summary_query(&mut rows, &request, chrono::Utc::now());
    let total_count = Some(rows.len() as u64);
    let sliced: Vec<RunSummary> = rows
        .into_iter()
        .skip(request.offset as usize)
        .take((request.limit + 1) as usize)
        .collect();
    Ok(Page::from_oversized(sliced, request.limit, total_count))
}

#[cfg(feature = "ssr")]
async fn require_job_id(
    backend: &dyn ChrononCoordinatorBackend,
    job_id_or_name: &str,
) -> Result<String, ServerFnError> {
    chronon_backend::validate_job_id_or_name(job_id_or_name)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    resolve_job_id(backend, job_id_or_name)
        .await?
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", job_id_or_name)))
}

/// Get runs, optionally filtered by job (by ID or name)
#[uf_product_macros::server]
pub async fn get_runs(
    /// Optional job ID or job name to restrict results to a single job's runs.
    job_id_or_name: Option<String>,
) -> Result<Vec<Run>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let mut db_runs = if let Some(jid_or_name) = &job_id_or_name {
        let actual_job_id = require_job_id(backend, jid_or_name).await?;
        require_job_id_action(backend, &actual_job_id, "View").await?;
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

    let jobs = backend.list_jobs().await;
    keep_viewable_runs(&mut db_runs, &viewable_job_ids(&jobs).await);
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

/// One page of runs (retries folded in) with search, result/time/job filters and a total.
///
/// Structured filter fields: `result` (`any`, `failed`, `worked`, `running`), `time` (`24h`,
/// `7d`, `30d`) and `job` (a job id). Quick search matches job name or run id.
#[uf_product_macros::server]
pub async fn get_runs_page(
    /// `DataTable` paging/filter/search request from the client.
    request: PageRequest,
) -> Result<Page<RunSummary>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    summaries_page(&ctx, backend.as_ref(), None, request).await
}

/// One page of a job's runs (retries folded in), with the same filters as [`get_runs_page`].
#[uf_product_macros::server]
pub async fn get_job_runs_page(
    /// Job ID or job name whose runs should be listed.
    job_id_or_name: String,
    /// `DataTable` paging/filter/search request from the client.
    request: PageRequest,
) -> Result<Page<RunSummary>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let actual_job_id = require_job_id(backend, &job_id_or_name).await?;
    require_job_id_action(backend, &actual_job_id, "View").await?;
    summaries_page(&ctx, backend, Some(actual_job_id.as_str()), request).await
}

/// Caption over the Runs table: how many runs fall in the time filter and which retried.
#[uf_product_macros::server]
pub async fn get_runs_caption(
    /// Time filter value (`24h`, `7d`, `30d`, or empty for no limit).
    time: String,
    /// Optional job id filter.
    job_id: Option<String>,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let job_id = match job_id.as_deref().filter(|j| !j.trim().is_empty()) {
        Some(j) => Some(require_job_id(backend, j).await?),
        None => None,
    };
    let mut runs = scan_runs(backend, job_id.as_deref()).await?;
    let jobs = backend.list_jobs().await;
    keep_viewable_runs(&mut runs, &viewable_job_ids(&jobs).await);
    // Counts only, so Started by names aren't needed.
    let mut rows = chronon_backend::fold_runs(&runs, &jobs, &std::collections::HashMap::new());
    let time_filter = chronon_backend::RunTimeFilter::parse(&time);
    if let Some(since) = time_filter.since(chrono::Utc::now()) {
        rows.retain(|r| {
            chrono::DateTime::parse_from_rfc3339(&r.started_at)
                .is_ok_and(|t| t.with_timezone(&chrono::Utc) >= since)
        });
    }
    Ok(chronon_backend::runs_caption(&rows, time_filter))
}

/// The first attempt of a run and its retries, oldest first. Any attempt's id works.
#[uf_product_macros::server]
pub async fn get_run_attempts(
    /// Run id of any attempt in the run.
    root_run_id: String,
) -> Result<Vec<RunAttempt>, ServerFnError> {
    chronon_backend::validate_run_id(&root_run_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let Some(run) = backend
        .get_run(&root_run_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get run: {}", e)))?
    else {
        return Ok(Vec::new());
    };
    require_run_view(backend, &run).await?;
    let runs = scan_runs(backend, run.job_id.as_deref()).await?;
    Ok(chronon_backend::find_run_group(&runs, &root_run_id)
        .map(|group| chronon_backend::run_attempts(&group))
        .unwrap_or_default())
}

/// A job's last seven days, its summary line, and the failing banner when its newest run
/// failed. A run that failed and then worked on retry counts as worked.
#[uf_product_macros::server]
pub async fn get_job_week(
    /// Job ID or job name.
    job_id_or_name: String,
) -> Result<JobWeek, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let actual_job_id = require_job_id(backend, &job_id_or_name).await?;
    require_job_id_action(backend, &actual_job_id, "View").await?;
    let runs = scan_runs(backend, Some(&actual_job_id)).await?;
    Ok(chronon_backend::job_week(&runs, chrono::Utc::now()))
}

/// Get a single run by ID
#[uf_product_macros::server]
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
            let job = require_run_view(backend, &r).await?;
            let job_name = job.map_or_else(|| r.script_name.clone(), |j| j.job_name);
            Some(model_run_to_app_run(r, job_name))
        }
    };

    Ok(out)
}

/// Everything the run detail page shows: the attempt, its title in words, who started it,
/// the parameters it used against the stored ones, and whether the viewer may run it again.
#[uf_product_macros::server]
pub async fn get_run_detail(
    /// Run id of the attempt to show.
    run_id: String,
) -> Result<Option<RunDetail>, ServerFnError> {
    chronon_backend::validate_run_id(&run_id).map_err(|e| ServerFnError::new(e.to_string()))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();

    let Some(run) = backend
        .get_run(&run_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get run: {}", e)))?
    else {
        return Ok(None);
    };
    let job = require_run_view(backend, &run).await?;
    let runs = scan_runs(backend, run.job_id.as_deref()).await?;
    let group = chronon_backend::find_run_group(&runs, &run_id).unwrap_or_default();
    let root = group.first().copied().unwrap_or(&run);
    let root_started = root.started_at.unwrap_or(root.scheduled_for);
    let names = people_names(&ctx, std::slice::from_ref(&run)).await?;
    let started_by = chronon_backend::started_by(&run, &root.run_id, job.as_ref(), &names);
    let started_by_label =
        chronon_backend::started_by_label(&started_by, job.as_ref(), root_started);
    let can_run = match job.as_ref() {
        Some(j) => {
            chronon_coordinator::validate_external_job_actor_json(&j.actor_json).is_ok()
                && can_run_job(j).await
        }
        None => false,
    };
    let now = chrono::Utc::now();
    let detail = RunDetail {
        title: chronon_backend::run_title(run.started_at.unwrap_or(run.scheduled_for), now),
        root_run_id: root.run_id.clone(),
        started_by,
        started_by_label,
        params: run.params_json.clone(),
        stored_params: job
            .as_ref()
            .map_or(serde_json::Value::Null, |j| j.params_json.clone()),
        pool: chronon_backend::effective_job_pool(run.pool_id.as_deref()).to_string(),
        retry_caption: chronon_backend::retry_caption(job.as_ref()),
        can_run,
        run: model_run_to_app_run(
            run.clone(),
            job.as_ref()
                .map_or_else(|| run.script_name.clone(), |j| j.job_name.clone()),
        ),
    };
    Ok(Some(detail))
}

/// What the Run now dialog says before it runs: script, pool, timeout, retries, the script's
/// typed parameters with the stored values, and the line about the schedule.
#[uf_product_macros::server]
pub async fn get_run_now_facts(
    /// Job ID or job name.
    job_id_or_name: String,
) -> Result<RunNowFacts, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let backend = super::ssr_utils::chronon_backend()?;
    let backend = backend.as_ref();
    let actual_job_id = require_job_id(backend, &job_id_or_name).await?;
    let job = require_job_id_action(backend, &actual_job_id, "View").await?;
    let registry = super::ssr_utils::script_registry()?;
    let script_params = registry
        .list()
        .into_iter()
        .find(|d| d.name == job.script_name)
        .map(|d| chronon_backend::parse_script_params(d.signature_json))
        .unwrap_or_default();
    let runs = backend
        .list_runs(Some(&actual_job_id), None, 0, 200)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to list runs: {}", e)))?;
    let going = runs.iter().any(|r| {
        matches!(
            r.status,
            chronon_coordinator::RunStatus::Queued
                | chronon_coordinator::RunStatus::Claimed
                | chronon_coordinator::RunStatus::Running
        )
    });
    let can_run_now = chronon_coordinator::validate_external_job_actor_json(&job.actor_json)
        .is_ok()
        && can_run_job(&job).await;
    Ok(RunNowFacts {
        job_id: job.job_id.clone(),
        job_name: job.job_name.clone(),
        script_name: job.script_name.clone(),
        script_params,
        stored_params: chronon_backend::normalized_params(job.params_json.clone()),
        pool: chronon_backend::effective_job_pool(job.pool.as_deref()).to_string(),
        timeout: job
            .timeout_ms
            .and_then(|ms| u64::try_from(ms).ok())
            .map(chronon_backend::duration_words),
        if_it_fails: chronon_backend::if_it_fails_words(&job),
        schedule_line: chronon_backend::schedule_line(&job, chrono::Utc::now()),
        waits_for_running: job.concurrency <= 1 && going,
        can_run_now,
    })
}

/// Trigger immediate execution of a job.
///
/// Needs Run on the job (Run now and Run again both come here). The run records the
/// session's actor (who started it) and the parameters it used; that person is checked
/// again when the run starts.
#[uf_product_macros::server]
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
    let job = match backend.get_job(&job_id).await {
        Some(job) => Some(job),
        None => backend.get_job_by_name(&job_id).await,
    };
    let Some(job) = job else {
        return Err(ServerFnError::new(format!("Job {} not found", job_id)));
    };
    if chronon_coordinator::validate_external_job_actor_json(&job.actor_json).is_err() {
        return Err(ServerFnError::new(
            "Platform System jobs cannot be triggered via run_now; they run on schedule only",
        ));
    }
    require_job(&job, "Run").await?;

    // Record the session actor on the run: who started it. Never elevated.
    let valence = ctx
        .valence()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let actor = chronon_coordinator::snapshot_actor_json(&valence)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    backend
        .run_now_with_params_and_actor(&job.job_id, params, Some(actor))
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to run job: {}", e)))
}
