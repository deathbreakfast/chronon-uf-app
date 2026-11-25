//! Job revision history and list/detail redaction for client-facing server responses.
//!
//! [`describe_revisions`] turns stored revisions into History rows: who saved each one, a
//! one-line summary of what changed, and the settings as `key: value` lines for a diff.
//! Readers who can't view a job get [`redact_job_revision`] instead, which drops actor
//! identity and parameters. Non-admin job list/detail responses also clear params and
//! schedule fields via [`redact_job_params_for_non_admin`].
//!
//! Teaching entry: crate root [Redact job params](../index.html#redact-job-params).

use serde_json::Value;

use crate::pools::effective_job_pool;
use crate::schedule::{actor_label, describe, describe_run_once, MANUAL_SCHEDULE_TEXT};
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
///
/// Also clears `saved_by` and `settings_text`, which carry the actor and parameters.
#[must_use]
pub fn redact_job_revision(mut revision: JobRevision) -> JobRevision {
    revision.changed_by_actor_json = serde_json::Value::Null;
    revision.snapshot_json = redact_revision_snapshot(revision.snapshot_json);
    revision.saved_by = String::new();
    revision.settings_text = String::new();
    revision
}

/// The parts of a stored job snapshot that History compares.
#[derive(PartialEq)]
struct Settings {
    name: String,
    script: String,
    enabled: bool,
    schedule: String,
    pool: String,
    params: Vec<(String, String)>,
    limits: [Value; 3],
}

fn str_field<'a>(snapshot: &'a Value, key: &str) -> Option<&'a str> {
    snapshot
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "none".to_string(),
        other => other.to_string(),
    }
}

/// Schedule as its sentence, with the cron in brackets: `Every day at 02:00 UTC (0 2 * * *)`.
fn schedule_text(snapshot: &Value) -> String {
    match str_field(snapshot, "schedule_kind").unwrap_or("cron") {
        "run_once" => str_field(snapshot, "run_once_at")
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map_or_else(
                || MANUAL_SCHEDULE_TEXT.to_string(),
                |t| describe_run_once(t.with_timezone(&chrono::Utc)),
            ),
        "manual" => MANUAL_SCHEDULE_TEXT.to_string(),
        _ => str_field(snapshot, "cron_expr").map_or_else(
            || MANUAL_SCHEDULE_TEXT.to_string(),
            |cron| {
                let tz = str_field(snapshot, "timezone").unwrap_or("UTC");
                describe(cron, tz).map_or_else(
                    |_| cron.to_string(),
                    |sentence| format!("{sentence} ({cron})"),
                )
            },
        ),
    }
}

impl Settings {
    fn from_snapshot(snapshot: &Value) -> Self {
        let mut params: Vec<(String, String)> = snapshot
            .get("params_json")
            .and_then(Value::as_object)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), value_text(v))).collect())
            .unwrap_or_default();
        params.sort();
        Self {
            name: str_field(snapshot, "job_name")
                .unwrap_or_default()
                .to_string(),
            script: str_field(snapshot, "script_name")
                .unwrap_or_default()
                .to_string(),
            enabled: snapshot
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            schedule: schedule_text(snapshot),
            pool: effective_job_pool(str_field(snapshot, "pool")).to_string(),
            params,
            limits: ["concurrency", "timeout_ms", "retry_policy_json"]
                .map(|k| snapshot.get(k).cloned().unwrap_or(Value::Null)),
        }
    }
}

/// A revision's settings, one `key: value` line each, for the History diff.
///
/// Lines are the schedule (sentence and cron), the script, the worker pool, each parameter
/// in name order, then whether the job is paused.
///
/// ```rust
/// use chronon_backend::revision_settings_text;
///
/// let snapshot = serde_json::json!({
///     "script_name": "image_thumbnails",
///     "cron_expr": "0 2 * * *",
///     "pool": "thumbs-gpu",
///     "enabled": true,
///     "params_json": {"max_width": 640, "bucket": "demo-uploads"},
/// });
/// assert_eq!(
///     revision_settings_text(&snapshot),
///     "schedule: Every day at 02:00 UTC (0 2 * * *)\n\
///      script: image_thumbnails\n\
///      worker pool: thumbs-gpu\n\
///      bucket: demo-uploads\n\
///      max_width: 640\n\
///      paused: no"
/// );
/// ```
#[must_use]
pub fn revision_settings_text(snapshot: &Value) -> String {
    let settings = Settings::from_snapshot(snapshot);
    let mut lines = vec![
        format!("schedule: {}", settings.schedule),
        format!("script: {}", settings.script),
        format!("worker pool: {}", settings.pool),
    ];
    lines.extend(settings.params.iter().map(|(k, v)| format!("{k}: {v}")));
    lines.push(format!(
        "paused: {}",
        if settings.enabled { "no" } else { "yes" }
    ));
    lines.join("\n")
}

/// One-line summary of what a revision changed from the one before it.
///
/// `previous` is `None` for the first revision (`Created the job`). One change reads as a
/// sentence, such as `Moved to the thumbs-gpu worker pool`; several read as the first one
/// and a count.
///
/// ```rust
/// use chronon_backend::revision_summary;
///
/// let before = serde_json::json!({"script_name": "image_thumbnails", "pool": null});
/// let after = serde_json::json!({"script_name": "image_thumbnails", "pool": "thumbs-gpu"});
/// assert_eq!(revision_summary(None, &before), "Created the job");
/// assert_eq!(revision_summary(Some(&before), &after), "Moved to the thumbs-gpu worker pool");
/// ```
#[must_use]
pub fn revision_summary(previous: Option<&Value>, current: &Value) -> String {
    let Some(previous) = previous else {
        return "Created the job".to_string();
    };
    let (old, new) = (
        Settings::from_snapshot(previous),
        Settings::from_snapshot(current),
    );
    let mut changes = Vec::new();
    if old.enabled != new.enabled {
        changes.push(if new.enabled {
            "Resumed the job".to_string()
        } else {
            "Paused the job".to_string()
        });
    }
    if old.schedule != new.schedule {
        changes.push(format!("Changed the schedule to {}", new.schedule));
    }
    if old.pool != new.pool {
        changes.push(format!("Moved to the {} worker pool", new.pool));
    }
    if old.script != new.script {
        changes.push(format!("Switched to the {} script", new.script));
    }
    if old.name != new.name {
        changes.push(format!("Renamed it to {}", new.name));
    }
    let changed_params: Vec<&(String, String)> = new
        .params
        .iter()
        .filter(|p| !old.params.contains(p))
        .collect();
    let removed_params = old
        .params
        .iter()
        .filter(|(k, _)| !new.params.iter().any(|(nk, _)| nk == k))
        .count();
    match (changed_params.as_slice(), removed_params) {
        ([], 0) => {}
        ([(k, v)], 0) => changes.push(format!("Set {k} to {v}")),
        _ => changes.push("Changed the parameters".to_string()),
    }
    if old.limits != new.limits {
        changes.push("Changed how it runs and retries".to_string());
    }
    match changes.len() {
        0 => "Saved without changes".to_string(),
        1 => changes.remove(0),
        n => {
            let rest = n - 1;
            format!(
                "{} and {rest} more {}",
                changes[0],
                if rest == 1 { "change" } else { "changes" }
            )
        }
    }
}

/// Fills in who saved each revision, what it changed and its settings text, newest first.
///
/// For readers who can view the job; everyone else gets [`redact_job_revision`].
///
/// ```rust
/// use chronon_backend::{describe_revisions, JobRevision};
///
/// let rev = |n: u32, pool: &str| JobRevision {
///     revision_id: format!("rev-{n}"),
///     revision_number: n,
///     changed_at: "2026-10-27T10:05:00Z".into(),
///     changed_by_actor_json: serde_json::json!({"User": {"user_id": "ines"}}),
///     snapshot_json: serde_json::json!({"script_name": "image_thumbnails", "pool": pool}),
///     saved_by: String::new(),
///     summary: String::new(),
///     settings_text: String::new(),
/// };
/// let rows = describe_revisions(vec![rev(1, "general"), rev(2, "thumbs-gpu")]);
/// assert_eq!(rows[0].revision_number, 2);
/// assert_eq!(rows[0].saved_by, "ines");
/// assert_eq!(rows[0].summary, "Moved to the thumbs-gpu worker pool");
/// assert_eq!(rows[1].summary, "Created the job");
/// ```
#[must_use]
pub fn describe_revisions(mut revisions: Vec<JobRevision>) -> Vec<JobRevision> {
    revisions.sort_by_key(|r| r.revision_number);
    let snapshots: Vec<Value> = revisions.iter().map(|r| r.snapshot_json.clone()).collect();
    for (i, revision) in revisions.iter_mut().enumerate() {
        let previous = i.checked_sub(1).map(|p| &snapshots[p]);
        revision.saved_by = actor_label(&revision.changed_by_actor_json);
        revision.summary = revision_summary(previous, &snapshots[i]);
        revision.settings_text = revision_settings_text(&snapshots[i]);
    }
    revisions.reverse();
    revisions
}

#[cfg(test)]
mod tests {
    use super::{
        describe_revisions, redact_job_params_for_non_admin, redact_job_revision,
        redact_revision_snapshot, revision_settings_text, revision_summary,
    };
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
            last_run_status: None,
            next_run_at: Some("2026-01-02T00:00:00Z".into()),
            timezone: Some("UTC".into()),
            params: serde_json::json!({"token": "secret", "bucket": "prod"}),
            pool: None,
            schedule_text: String::new(),
            last_run_attempts: None,
            owner_label: String::new(),
            system_owned: false,
            your_access: String::new(),
            concurrency: 1,
            timeout_ms: None,
            retry_max_attempts: 0,
            retry_base_delay_ms: 0,
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
            saved_by: "u1".into(),
            summary: "Created the job".into(),
            settings_text: "token: secret".into(),
        };
        let redacted = redact_job_revision(revision);
        assert!(redacted.changed_by_actor_json.is_null());
        assert_eq!(redacted.saved_by, "");
        assert_eq!(redacted.settings_text, "");
        assert!(redacted.snapshot_json.get("params_json").unwrap().is_null());
        assert_eq!(redacted.revision_number, 2);
    }

    fn snapshot(pool: &str, cron: &str, enabled: bool) -> serde_json::Value {
        serde_json::json!({
            "job_name": "nightly-thumbnails",
            "script_name": "image_thumbnails",
            "schedule_kind": "cron",
            "cron_expr": cron,
            "timezone": "UTC",
            "pool": pool,
            "enabled": enabled,
            "params_json": {"bucket": "demo-uploads", "max_width": 640},
            "concurrency": 1,
            "timeout_ms": null,
            "retry_policy_json": {"max_attempts": 0},
        })
    }

    fn revision(n: u32, actor: serde_json::Value, snap: serde_json::Value) -> JobRevision {
        JobRevision {
            revision_id: format!("rev-{n}"),
            revision_number: n,
            changed_at: "2026-10-27T10:05:00Z".into(),
            changed_by_actor_json: actor,
            snapshot_json: snap,
            saved_by: String::new(),
            summary: String::new(),
            settings_text: String::new(),
        }
    }

    #[test]
    fn describe_revisions_names_saver_and_pool_move_happy() {
        let ines = serde_json::json!({"User": {"user_id": "ines"}});
        let rows = describe_revisions(vec![
            revision(2, ines.clone(), snapshot("thumbs-gpu", "0 2 * * *", true)),
            revision(1, ines, snapshot("general", "0 2 * * *", true)),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].revision_number, 2);
        assert_eq!(rows[0].saved_by, "ines");
        assert_eq!(rows[0].summary, "Moved to the thumbs-gpu worker pool");
        assert!(rows[0].settings_text.contains("worker pool: thumbs-gpu"));
        assert!(rows[0].settings_text.contains("bucket: demo-uploads"));
        assert_eq!(rows[1].summary, "Created the job");
        assert!(rows[1].settings_text.contains("worker pool: general"));
    }

    #[test]
    fn describe_revisions_system_saver_reads_the_platform() {
        let rows = describe_revisions(vec![revision(
            1,
            serde_json::json!({"System": {"operation": "boot"}}),
            snapshot("general", "0 2 * * *", true),
        )]);
        assert_eq!(rows[0].saved_by, "the platform");
    }

    #[test]
    fn revision_settings_text_lines_happy() {
        assert_eq!(
            revision_settings_text(&snapshot("general", "0 2 * * *", false)),
            "schedule: Every day at 02:00 UTC (0 2 * * *)\nscript: image_thumbnails\n\
             worker pool: general\nbucket: demo-uploads\nmax_width: 640\npaused: yes"
        );
    }

    #[test]
    fn revision_summary_names_each_kind_of_change() {
        let base = snapshot("general", "0 2 * * *", true);
        assert_eq!(
            revision_summary(Some(&base), &snapshot("general", "0 2 * * *", false)),
            "Paused the job"
        );
        assert_eq!(
            revision_summary(Some(&base), &snapshot("general", "0 4 * * *", true)),
            "Changed the schedule to Every day at 04:00 UTC (0 4 * * *)"
        );
        let mut width = base.clone();
        width["params_json"]["max_width"] = serde_json::json!(1024);
        assert_eq!(
            revision_summary(Some(&base), &width),
            "Set max_width to 1024"
        );
        assert_eq!(
            revision_summary(Some(&base), &snapshot("gpu", "0 4 * * *", true)),
            "Changed the schedule to Every day at 04:00 UTC (0 4 * * *) and 1 more change"
        );
        assert_eq!(
            revision_summary(Some(&base), &base),
            "Saved without changes"
        );
    }
}
