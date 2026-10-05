//! DTO mappers and pure aggregates backing Chronon job/run server fns.

use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};

use crate::types::{Job, JobStatus, RecentRun, Run, RunStatus, ScriptParam};

/// Maps a core Chronon job onto the UI-facing job DTO.
#[must_use]
pub fn backend_job_to_job(j: CoreJob) -> Job {
    Job {
        id: j.job_id,
        name: j.job_name,
        script_name: j.script_name,
        cron: j.cron_expr.unwrap_or_else(|| "manual".to_string()),
        status: if j.enabled {
            JobStatus::Active
        } else {
            JobStatus::Paused
        },
        revision: u32::try_from(j.current_revision).unwrap_or(u32::MAX),
        // TODO: Populate from run lookup once run-store hydration is wired.
        last_run_at: None,
        next_run_at: j.next_run_at.map(|dt| dt.to_rfc3339()),
        timezone: j.timezone,
        params: j.params_json,
        pool: j.pool,
    }
}

/// Maps a core run status onto the UI wire enum.
#[must_use]
pub const fn chronon_run_status_to_ui(s: CoreRunStatus) -> RunStatus {
    match s {
        CoreRunStatus::Queued | CoreRunStatus::Claimed => RunStatus::Pending,
        CoreRunStatus::Running => RunStatus::Running,
        CoreRunStatus::Success => RunStatus::Completed,
        CoreRunStatus::Failed | CoreRunStatus::Timeout => RunStatus::Failed,
        CoreRunStatus::Canceled => RunStatus::Cancelled,
    }
}

fn duration_ms_u64(duration_ms: Option<i64>) -> Option<u64> {
    duration_ms.and_then(|d| u64::try_from(d).ok())
}

/// Maps a core run onto the UI-facing run DTO with a resolved job name.
#[must_use]
pub fn model_run_to_app_run(m: CoreRun, job_name: String) -> Run {
    Run {
        id: m.run_id,
        job_id: m.job_id.unwrap_or_default(),
        job_name,
        status: chronon_run_status_to_ui(m.status),
        started_at: m
            .started_at
            .map_or_else(|| m.scheduled_for.to_rfc3339(), |t| t.to_rfc3339()),
        finished_at: m.finished_at.map(|t| t.to_rfc3339()),
        duration_ms: duration_ms_u64(m.duration_ms),
        logs: m.stdout_text,
        stderr: m.stderr_text,
        error_message: m
            .error_json
            .as_ref()
            .and_then(|v| v.get("message").and_then(|x| x.as_str()).map(String::from)),
        parent_run_id: m.parent_run_id,
    }
}

/// Maps a core run onto a dashboard recent-run row.
#[must_use]
pub fn recent_run_from_model(m: &CoreRun, job_name: String) -> RecentRun {
    RecentRun {
        id: m.run_id.clone(),
        job_name,
        status: chronon_run_status_to_ui(m.status),
        started_at: m
            .started_at
            .map_or_else(|| m.scheduled_for.to_rfc3339(), |dt| dt.to_rfc3339()),
        duration_ms: duration_ms_u64(m.duration_ms),
    }
}

/// Ensures request params are always persisted as a JSON object, never null.
#[must_use]
pub fn normalized_params(params: serde_json::Value) -> serde_json::Value {
    if params.is_null() {
        serde_json::json!({})
    } else {
        params
    }
}

/// Parses script signature JSON into displayable script parameter metadata.
#[must_use]
pub fn parse_script_params(signature_json: &str) -> Vec<ScriptParam> {
    serde_json::from_str::<serde_json::Value>(signature_json)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.iter()
                .map(|(name, value)| {
                    let param_type = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        other => other.to_string().trim_matches('"').to_string(),
                    };
                    ScriptParam {
                        name: name.clone(),
                        param_type,
                        default: None,
                        required: true,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Lowercase label for a UI job status (`DataTable` filters).
#[must_use]
pub const fn job_status_label(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Active => "active",
        JobStatus::Paused => "paused",
        JobStatus::Disabled => "disabled",
    }
}

/// Lowercase label for a UI run status (`DataTable` filters).
#[must_use]
pub const fn run_status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Pending => "pending",
        RunStatus::Running => "running",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
    }
}
