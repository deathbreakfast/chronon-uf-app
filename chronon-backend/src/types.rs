//! UI-facing DTOs and paging constants for Chronon server contracts.

use serde::{Deserialize, Serialize};

/// Represents the current lifecycle state of a Chronon job in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// The job is enabled and will run on its configured schedule.
    Active,
    /// The job is temporarily suspended; existing runs are unaffected but no new runs fire.
    Paused,
    /// The job is disabled and will not be scheduled until re-enabled.
    Disabled,
}

/// Represents the execution status for a Chronon run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run has been scheduled/queued but has not started executing.
    Pending,
    /// The run is currently executing.
    Running,
    /// The run finished successfully.
    Completed,
    /// The run finished with an error.
    Failed,
    /// The run was cancelled before completion.
    Cancelled,
}

/// Parameter metadata for a registered script signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptParam {
    /// Parameter name as declared in the script signature.
    pub name: String,
    /// Display name of the parameter's expected type.
    pub param_type: String,
    /// Default value shown/used when the caller doesn't supply one, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Whether the caller must supply a value for this parameter.
    pub required: bool,
}

/// Script registry entry shown in the scripts page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Script {
    /// Registered script name.
    pub name: String,
    /// Human-readable signature string (name + parameter list).
    pub signature: String,
    /// Declared parameters for this script.
    pub params: Vec<ScriptParam>,
    /// Optional human-readable description of what the script does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Job DTO consumed by Chronon UI pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: String,
    /// Human-readable job name.
    pub name: String,
    /// Name of the registered script this job executes.
    pub script_name: String,
    /// Cron expression describing the job's schedule.
    pub cron: String,
    /// Current lifecycle status of the job.
    pub status: JobStatus,
    /// Monotonically increasing revision number for the job's configuration.
    pub revision: u32,
    /// Timestamp of the job's last run, if it has ever run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    /// Timestamp of the job's next scheduled run, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at: Option<String>,
    /// IANA timezone the cron schedule is evaluated in, if overridden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// JSON parameters passed to the script on each run.
    pub params: serde_json::Value,
    /// Worker pool the job's runs are claimed from; `None` means [`crate::DEFAULT_POOL`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
}

/// Run DTO consumed by Chronon UI pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// Unique run identifier.
    pub id: String,
    /// Identifier of the job this run belongs to.
    pub job_id: String,
    /// Name of the job this run belongs to, denormalized for display.
    pub job_name: String,
    /// Current execution status of the run.
    pub status: RunStatus,
    /// Timestamp the run started executing.
    pub started_at: String,
    /// Timestamp the run finished, if it has finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Wall-clock duration of the run in milliseconds, if finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Captured stdout/log output, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    /// Captured stderr output, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    /// Error message describing why the run failed, if it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Identifier of the run this run was manually retried/re-run from, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
}

/// Dashboard KPI aggregate values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardStats {
    /// Total number of configured jobs.
    pub total_jobs: u32,
    /// Number of jobs currently active (scheduled to run).
    pub active_jobs: u32,
    /// Number of jobs currently paused.
    pub paused_jobs: u32,
    /// Total number of runs that have started today.
    pub total_runs_today: u32,
    /// Number of runs that completed successfully today.
    pub successful_runs_today: u32,
    /// Number of runs that failed today.
    pub failed_runs_today: u32,
    /// Number of runs currently executing.
    pub running_now: u32,
}

/// Run preview row for dashboard recent activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentRun {
    /// Unique run identifier.
    pub id: String,
    /// Name of the job this run belongs to.
    pub job_name: String,
    /// Current execution status of the run.
    pub status: RunStatus,
    /// Timestamp the run started executing.
    pub started_at: String,
    /// Wall-clock duration of the run in milliseconds, if finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Immutable job revision view-model used by job detail history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRevision {
    /// Unique identifier for this revision record.
    pub revision_id: String,
    /// Sequential revision number for the job at the time of this snapshot.
    pub revision_number: u32,
    /// Timestamp when this revision was recorded.
    pub changed_at: String,
    /// Actor JSON identifying who/what made the change.
    pub changed_by_actor_json: serde_json::Value,
    /// Full JSON snapshot of the job's configuration at this revision.
    pub snapshot_json: serde_json::Value,
}

/// Payload used to update mutable job configuration fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateJobRequest {
    /// New human-readable job name.
    pub job_name: String,
    /// New cron expression, or `None` to leave unchanged.
    pub cron_expr: Option<String>,
    /// New IANA timezone override, or `None` to leave unchanged.
    pub timezone: Option<String>,
    /// New JSON parameters to pass to the script on each run.
    #[serde(default = "default_params")]
    pub params: serde_json::Value,
    /// Whether the job should be enabled after this update.
    pub enabled: bool,
    /// Worker pool for the job's runs; `None` keeps the stored pool, blank resets to
    /// [`crate::DEFAULT_POOL`].
    #[serde(default)]
    pub pool: Option<String>,
}

/// Schedule kind chosen when creating a new job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateJobScheduleType {
    /// Recurring schedule driven by a cron expression.
    Cron,
    /// One-shot execution at a specific time.
    RunOnce,
    /// No automatic schedule; the job only runs when triggered manually.
    Manual,
}

/// Payload used to create a new job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    /// Human-readable name for the new job.
    pub job_name: String,
    /// Name of the registered script the job should execute.
    pub script_name: String,
    /// Which kind of schedule this job uses.
    pub schedule_type: CreateJobScheduleType,
    /// Cron expression, required when `schedule_type` is [`CreateJobScheduleType::Cron`].
    pub cron_expr: Option<String>,
    /// IANA timezone the cron schedule is evaluated in, if overridden.
    pub timezone: Option<String>,
    /// Timestamp to run at once, required when `schedule_type` is
    /// [`CreateJobScheduleType::RunOnce`].
    pub run_once_at: Option<String>,
    /// JSON parameters passed to the script on each run.
    #[serde(default = "default_params")]
    pub params: serde_json::Value,
    /// Maximum number of concurrent runs allowed for this job.
    pub concurrency: u32,
    /// Per-run timeout, in seconds.
    pub timeout_seconds: u32,
    /// Maximum number of automatic retries on failure.
    pub max_retries: u32,
    /// Worker pool for the job's runs; `None` or blank means [`crate::DEFAULT_POOL`].
    #[serde(default)]
    pub pool: Option<String>,
}

fn default_params() -> serde_json::Value {
    serde_json::json!({})
}

/// Time-series point for dashboard charts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardChartPoint {
    /// Timestamp of this data point.
    pub ts: chrono::DateTime<chrono::Utc>,
    /// Value of this data point.
    pub value: f64,
}

/// Named time series for dashboard charts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardChartSeries {
    /// Stable identifier for this series.
    pub id: String,
    /// Display label for this series (e.g. shown in a chart legend).
    pub label: String,
    /// Ordered data points making up the series.
    pub points: Vec<DashboardChartPoint>,
}

/// Number of jobs per page in the jobs list view.
pub const JOBS_PAGE_SIZE: u32 = 20;

/// Number of runs per page in the run history view.
pub const RUNS_PAGE_SIZE: u32 = 20;

/// Number of scripts per page in the registered scripts view.
pub const SCRIPTS_PAGE_SIZE: u32 = 20;

/// Number of runs per page in the job detail recent runs section.
pub const JOB_RUNS_PAGE_SIZE: u32 = 10;
