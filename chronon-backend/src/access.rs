//! Per-job access: which Gauge bundle a job uses, and runs refused when they started.
//!
//! Every job has its own permission bundle of kind [`CHRONON_JOB_KIND`]. A job a person made
//! in the app uses its job id; a job the platform declares in code (System job actor) uses
//! its script name, so the bundle exists from host boot. See [`job_resource_id`].
//!
//! A run started by a person is checked again, as that person, when it starts. A failed
//! check ends the run with the error text `refused: missing Run on chronon_job.<id>`;
//! [`run_refusal`] reads it back so the run shows as Refused with the missing action.

use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};

use crate::schedule::is_system_actor;

/// Gauge resource kind prefix for one Chronon job.
pub const CHRONON_JOB_KIND: &str = "chronon_job";

/// Start of the error text a refused run records.
pub const REFUSED_PREFIX: &str = "refused: ";

/// The resource id of a job's permission bundle.
///
/// System-owned jobs are declared in code and share their script's bundle; any other job
/// was made by a person and has its own, keyed by job id.
#[must_use]
pub fn job_resource_id(system_owned: bool, job_id: &str, script_name: &str) -> String {
    if system_owned {
        script_name.to_string()
    } else {
        job_id.to_string()
    }
}

/// [`job_resource_id`] for a stored Chronon job.
#[must_use]
pub fn core_job_resource_id(job: &CoreJob) -> String {
    job_resource_id(
        is_system_actor(&job.actor_json),
        &job.job_id,
        &job.script_name,
    )
}

/// Why a run was refused when it started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRefusal {
    /// The missing action, such as `Run`.
    pub action: String,
    /// The job's resource id.
    pub resource: String,
}

/// The error text a run refused for lacking `action` on `resource` records.
#[must_use]
pub fn refused_text(action: &str, resource: &str) -> String {
    format!("{REFUSED_PREFIX}missing {action} on {CHRONON_JOB_KIND}.{resource}")
}

/// Read a refusal back from a run's error text; `None` for any other error.
#[must_use]
pub fn parse_refused(error: &str) -> Option<RunRefusal> {
    let rest = error
        .strip_prefix(REFUSED_PREFIX)?
        .strip_prefix("missing ")?;
    let (action, target) = rest.split_once(" on ")?;
    let resource = target.strip_prefix(CHRONON_JOB_KIND)?.strip_prefix('.')?;
    if action.is_empty() || resource.is_empty() {
        return None;
    }
    Some(RunRefusal {
        action: action.to_string(),
        resource: resource.to_string(),
    })
}

/// The run's error message (`error_json.message`), if any.
#[must_use]
pub fn run_error_message(run: &CoreRun) -> Option<&str> {
    run.error_json
        .as_ref()
        .and_then(|v| v.get("message"))
        .and_then(serde_json::Value::as_str)
}

/// The refusal of a failed run that was refused when it started.
#[must_use]
pub fn run_refusal(run: &CoreRun) -> Option<RunRefusal> {
    if run.status != CoreRunStatus::Failed {
        return None;
    }
    run_error_message(run).and_then(parse_refused)
}

/// The run-history line for a refused run, such as `no longer holds Run on billing-rollup`.
#[must_use]
pub fn refused_note(action: &str, job_name: &str) -> String {
    format!("no longer holds {action} on {job_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn resource_id_is_script_for_system_jobs_and_job_id_otherwise() {
        assert_eq!(job_resource_id(true, "j1", "ttl_sweep"), "ttl_sweep");
        assert_eq!(job_resource_id(false, "j1", "ttl_sweep"), "j1");

        let mut job = CoreJob::new("nightly", "e2e_echo");
        job.actor_json = json!({"System": {"operation": "boot"}});
        assert_eq!(core_job_resource_id(&job), "e2e_echo");
        job.actor_json = json!({"User": {"user_id": "ines"}});
        assert_eq!(core_job_resource_id(&job), job.job_id);
    }

    #[test]
    fn refused_text_round_trips() {
        let text = refused_text("Run", "billing-rollup");
        assert_eq!(text, "refused: missing Run on chronon_job.billing-rollup");
        assert_eq!(
            parse_refused(&text),
            Some(RunRefusal {
                action: "Run".into(),
                resource: "billing-rollup".into(),
            })
        );
    }

    #[test]
    fn parse_refused_ignores_other_errors() {
        assert_eq!(parse_refused("Valence refused the connection"), None);
        assert_eq!(parse_refused("refused: missing Run on boson_queue.x"), None);
        assert_eq!(parse_refused("refused: missing  on chronon_job.x"), None);
    }

    #[test]
    fn run_refusal_needs_a_failed_run_with_the_refused_text() {
        let mut run = CoreRun::new("e2e_echo".to_string(), Utc::now());
        run.error_json = Some(json!({"message": refused_text("Run", "j1")}));
        assert_eq!(run_refusal(&run), None, "a queued run isn't refused");
        run.status = CoreRunStatus::Failed;
        assert_eq!(run_refusal(&run).map(|r| r.action), Some("Run".into()));
        run.error_json = Some(json!({"message": "boom"}));
        assert_eq!(run_refusal(&run), None);
    }

    #[test]
    fn refused_note_names_the_action_and_job() {
        assert_eq!(
            refused_note("Run", "billing-rollup"),
            "no longer holds Run on billing-rollup"
        );
    }
}
