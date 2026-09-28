//! Job revision and list/detail redaction for client-facing server responses.
//!
//! Full snapshots stay in the Chronon store. Wire responses omit actor identity
//! and sensitive snapshot fields. Non-admin job list/detail responses also clear
//! params and schedule fields via [`redact_job_params_for_non_admin`].
//!
//! Teaching entry: crate root [Redact job params](../index.html#redact-job-params).

use crate::types::{Job, JobRevision};

/// Clear job params and cron on the wire when the session lacks `ChrononAdmin`.
///
/// Used by chronon-app job list/detail server functions so non-admin viewers
/// cannot read script parameters or cron expressions from the ops UI.
#[must_use]
pub fn redact_job_params_for_non_admin(mut job: Job, is_admin: bool) -> Job {
    if !is_admin {
        job.params = serde_json::json!({});
        job.cron = String::new();
        job.next_run_at = None;
        job.timezone = None;
    }
    job
}

/// Strips sensitive fields from a revision snapshot before returning it to clients.
///
/// Nulls `actor_json` and `params_json` while preserving structural fields such as
/// `job_name`, `cron_expr`, and `timezone` needed by the job-detail revision picker.
#[must_use]
pub fn redact_revision_snapshot(mut snapshot: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert("actor_json".into(), serde_json::Value::Null);
        obj.insert("params_json".into(), serde_json::Value::Null);
    }
    snapshot
}

/// Applies client-safe redaction to a [`JobRevision`] DTO.
#[must_use]
pub fn redact_job_revision(mut revision: JobRevision) -> JobRevision {
    revision.changed_by_actor_json = serde_json::Value::Null;
    revision.snapshot_json = redact_revision_snapshot(revision.snapshot_json);
    revision
}

#[cfg(test)]
mod tests {
    use super::{redact_job_params_for_non_admin, redact_job_revision, redact_revision_snapshot};
    use crate::types::{Job, JobRevision, JobStatus};

    fn sample_job() -> Job {
        Job {
            id: "job-1".into(),
            name: "nightly".into(),
            script_name: "reports.export".into(),
            cron: "0 0 * * * *".into(),
            status: JobStatus::Active,
            revision: 1,
            last_run_at: None,
            next_run_at: Some("2026-01-02T00:00:00Z".into()),
            timezone: Some("UTC".into()),
            params: serde_json::json!({"token": "secret", "bucket": "prod"}),
        }
    }

    #[test]
    fn redact_job_params_for_non_admin_clears_params_and_cron_sad() {
        let redacted = redact_job_params_for_non_admin(sample_job(), false);
        assert_eq!(redacted.params, serde_json::json!({}));
        assert_eq!(redacted.cron, "");
        assert!(redacted.next_run_at.is_none());
        assert!(redacted.timezone.is_none());
        assert_eq!(redacted.name, "nightly");
        assert_eq!(redacted.script_name, "reports.export");
    }

    #[test]
    fn redact_job_params_for_admin_preserves_params_happy() {
        let original = sample_job();
        let kept = redact_job_params_for_non_admin(original.clone(), true);
        assert_eq!(kept, original);
    }

    #[test]
    fn redact_revision_snapshot_nulls_actor_and_params() {
        let snapshot = serde_json::json!({
            "job_name": "daily-sync",
            "cron_expr": "0 * * * *",
            "actor_json": {"role": "admin", "session": "sess-1"},
            "params_json": {"token": "super-secret"},
        });
        let redacted = redact_revision_snapshot(snapshot);
        assert!(redacted.get("actor_json").unwrap().is_null());
        assert!(redacted.get("params_json").unwrap().is_null());
        assert_eq!(
            redacted.get("job_name").and_then(|v| v.as_str()),
            Some("daily-sync")
        );
    }

    #[test]
    fn redact_job_revision_nulls_changed_by_actor_json() {
        let revision = JobRevision {
            revision_id: "rev-1".into(),
            revision_number: 2,
            changed_at: "2026-01-01T00:00:00Z".into(),
            changed_by_actor_json: serde_json::json!({"user_id": "u1"}),
            snapshot_json: serde_json::json!({
                "job_name": "daily-sync",
                "params_json": {"token": "secret"},
            }),
        };
        let redacted = redact_job_revision(revision);
        assert!(redacted.changed_by_actor_json.is_null());
        assert!(redacted.snapshot_json.get("params_json").unwrap().is_null());
        assert_eq!(redacted.revision_number, 2);
    }
}
