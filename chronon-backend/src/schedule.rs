//! Create/update schedule helpers for Chronon job configuration.

use chronon_core::{Job as CoreJob, ScheduleKind};
use chronon_scheduler::CronExpr;

use crate::map::normalized_params;
use crate::types::{CreateJobRequest, CreateJobScheduleType, UpdateJobRequest};
use crate::validate::{validate_job_name, validate_script_name, ChrononIdError};

/// Missing or invalid cron / run-once schedule input rejected before Chronon IO.
///
/// Callers map this into Leptos `ServerFnError` (or equivalent) at the `#[server]`
/// boundary; the Display text stays stable for UI and contract tests.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChrononScheduleError {
    /// Cron schedule selected but expression was empty or whitespace-only.
    MissingCron,
    /// Cron expression (or timezone) failed to parse.
    InvalidCron(String),
    /// Run-once schedule selected but datetime was empty or whitespace-only.
    MissingRunOnce,
    /// Run-once datetime was not RFC3339 or `YYYY-MM-DDTHH:MM:SS`.
    InvalidRunOnce,
    /// Job or script name failed [`ChrononIdError`] validation.
    Id(ChrononIdError),
}

impl std::fmt::Display for ChrononScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCron => {
                write!(f, "Cron expression is required for cron schedules")
            }
            Self::InvalidCron(detail) => write!(f, "Invalid cron expression: {detail}"),
            Self::MissingRunOnce => {
                write!(f, "Run-once datetime is required for run-once schedules")
            }
            Self::InvalidRunOnce => write!(
                f,
                "Invalid run-once datetime. Use ISO 8601 (e.g. 2026-01-25T03:00:00Z)"
            ),
            Self::Id(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ChrononScheduleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Id(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ChrononIdError> for ChrononScheduleError {
    fn from(value: ChrononIdError) -> Self {
        Self::Id(value)
    }
}

/// Parses run-once user input into a UTC timestamp.
///
/// # Errors
///
/// Returns [`ChrononScheduleError::InvalidRunOnce`] when the input is not RFC3339
/// or `YYYY-MM-DDTHH:MM:SS`.
///
/// ```rust
/// use chronon_backend::{parse_run_once_datetime, ChrononScheduleError};
///
/// assert!(parse_run_once_datetime("2026-01-25T03:00:00Z").is_ok());
/// assert!(matches!(
///     parse_run_once_datetime("not-a-date"),
///     Err(ChrononScheduleError::InvalidRunOnce)
/// ));
/// ```
pub fn parse_run_once_datetime(
    raw: &str,
) -> Result<chrono::DateTime<chrono::Utc>, ChrononScheduleError> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S").map(|naive| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
            })
        })
        .map_err(|_| ChrononScheduleError::InvalidRunOnce)
}

/// Applies the schedule-specific fields when creating a new job.
///
/// # Errors
///
/// Returns [`ChrononScheduleError`] for missing or invalid cron / run-once inputs.
pub fn apply_create_schedule(
    job: &mut CoreJob,
    payload: &CreateJobRequest,
) -> Result<(), ChrononScheduleError> {
    match payload.schedule_type {
        CreateJobScheduleType::Cron => {
            let cron_expr = payload
                .cron_expr
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string();
            if cron_expr.is_empty() {
                return Err(ChrononScheduleError::MissingCron);
            }
            let timezone = payload
                .timezone
                .clone()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());
            let cron = CronExpr::parse(&cron_expr, timezone.as_deref())
                .map_err(|e| ChrononScheduleError::InvalidCron(e.to_string()))?;
            job.schedule_kind = ScheduleKind::Cron;
            job.cron_expr = Some(cron.expression().to_string());
            job.timezone = timezone;
            job.next_run_at = cron.next_from_now();
            job.run_once_at = None;
        }
        CreateJobScheduleType::RunOnce => {
            let run_once_raw = payload
                .run_once_at
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string();
            if run_once_raw.is_empty() {
                return Err(ChrononScheduleError::MissingRunOnce);
            }
            let run_once_at = parse_run_once_datetime(&run_once_raw)?;
            job.schedule_kind = ScheduleKind::RunOnce;
            job.cron_expr = None;
            job.timezone = None;
            job.run_once_at = Some(run_once_at);
            job.next_run_at = Some(run_once_at);
        }
        CreateJobScheduleType::Manual => {
            job.schedule_kind = ScheduleKind::Manual;
            job.cron_expr = None;
            job.timezone = None;
            job.run_once_at = None;
            job.next_run_at = None;
        }
    }

    Ok(())
}

/// Recomputes `next_run_at` for cron jobs when config changes.
///
/// # Errors
///
/// Returns [`ChrononScheduleError::InvalidCron`] when the cron expression cannot be parsed.
pub fn recompute_next_run_for_cron(
    cron_expr: Option<&str>,
    updated_job: &mut CoreJob,
) -> Result<(), ChrononScheduleError> {
    if updated_job.schedule_kind == ScheduleKind::Cron {
        if let Some(cron_expr) = cron_expr {
            let cron = CronExpr::parse(cron_expr, updated_job.timezone.as_deref())
                .map_err(|e| ChrononScheduleError::InvalidCron(e.to_string()))?;
            updated_job.next_run_at = cron.next_from_now();
        }
    }
    Ok(())
}

/// Builds a persisted Chronon job model from create payload + signature hash.
///
/// # Errors
///
/// Returns [`ChrononScheduleError`] for blank names ([`ChrononScheduleError::Id`]) or
/// invalid schedule fields.
pub fn build_create_job_model(
    payload: &CreateJobRequest,
    signature_hash: String,
) -> Result<CoreJob, ChrononScheduleError> {
    validate_job_name(&payload.job_name)?;
    validate_script_name(&payload.script_name)?;

    let job_name = payload.job_name.trim().to_string();
    let script_name = payload.script_name.trim().to_string();

    let mut job = CoreJob::new(&job_name, &script_name);
    job.script_sig_hash = signature_hash;
    job.params_json = normalized_params(payload.params.clone());
    job.concurrency = i32::try_from(payload.concurrency.clamp(1, 10)).unwrap_or(10);
    job.timeout_ms = Some(i64::from(payload.timeout_seconds.max(1)) * 1000);
    job.retry_policy_json = serde_json::json!({
        "max_attempts": payload.max_retries.min(10),
        "base_delay_ms": 1000_u64,
        "backoff_multiplier": 2.0_f64,
        "max_delay_ms": 60000_u64
    });
    job.pool = stored_pool(payload.pool.as_deref());
    job.updated_at = chrono::Utc::now();

    apply_create_schedule(&mut job, payload)?;
    Ok(job)
}

/// Trimmed pool for `Job.pool`; blank stores as `None` (the default pool).
///
/// Offer checks live in [`crate::resolve_job_pool`], which the server runs first.
fn stored_pool(pool: Option<&str>) -> Option<String> {
    pool.map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// Applies mutable update payload fields to a loaded job model.
///
/// # Errors
///
/// Returns [`ChrononScheduleError::InvalidCron`] when cron recomputation fails.
pub fn apply_update_payload_to_job(
    existing_job: CoreJob,
    payload: &UpdateJobRequest,
) -> Result<CoreJob, ChrononScheduleError> {
    let mut updated_job = existing_job;
    let cron_expr_for_recompute = payload.cron_expr.clone();
    updated_job.job_name.clone_from(&payload.job_name);
    updated_job.cron_expr.clone_from(&payload.cron_expr);
    updated_job.timezone.clone_from(&payload.timezone);
    updated_job.params_json = normalized_params(payload.params.clone());
    updated_job.enabled = payload.enabled;
    if let Some(pool) = payload.pool.as_deref() {
        updated_job.pool = stored_pool(Some(pool));
    }

    recompute_next_run_for_cron(cron_expr_for_recompute.as_deref(), &mut updated_job)?;
    updated_job.updated_at = chrono::Utc::now();
    Ok(updated_job)
}
