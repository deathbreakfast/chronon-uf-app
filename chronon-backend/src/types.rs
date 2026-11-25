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
    /// Final result of the job's newest run (retries folded in), if it has ever run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_status: Option<RunStatus>,
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
    /// Plain-English schedule: a cron sentence, `Once on …`, or `Manual`.
    #[serde(default)]
    pub schedule_text: String,
    /// Attempts the last run took, when it was retried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_attempts: Option<u32>,
    /// Display label for the job's owner (the actor that saved it).
    #[serde(default)]
    pub owner_label: String,
    /// True when the job belongs to the platform System actor (schedule-only).
    #[serde(default)]
    pub system_owned: bool,
    /// Runs allowed at once.
    #[serde(default)]
    pub concurrency: u32,
    /// Per-run timeout in milliseconds, if set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Retry attempts after a failure.
    #[serde(default)]
    pub retry_max_attempts: u32,
    /// Delay before the first retry, in milliseconds.
    #[serde(default)]
    pub retry_base_delay_ms: u64,
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

/// Who or what started a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartedBy {
    /// The job's schedule fired it (System actor, or a fire time of the job's schedule).
    Schedule,
    /// A retry of an earlier attempt that failed.
    Retry {
        /// Run id of the first attempt this retry belongs to.
        of: String,
    },
    /// A person (or service account) started it with Run now.
    Person {
        /// Display label, such as `admin (Run now)`.
        label: String,
    },
}

impl StartedBy {
    /// Short label for tables: `Schedule`, `Retry`, or the person's label.
    #[must_use]
    pub fn short_label(&self) -> String {
        match self {
            Self::Schedule => "Schedule".to_string(),
            Self::Retry { .. } => "Retry".to_string(),
            Self::Person { label } => label.clone(),
        }
    }
}

/// One run as the runs table shows it: a first attempt with its retries folded in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// Run id of the first attempt; run links point here.
    pub root_run_id: String,
    /// Job the run belongs to.
    pub job_id: String,
    /// Job name (script name when the job is gone).
    pub job_name: String,
    /// Status of the final attempt.
    pub result: RunStatus,
    /// Number of attempts, first try included.
    pub attempts: u32,
    /// When the first attempt started (RFC 3339).
    pub started_at: String,
    /// Duration of the run; for retried runs, the average per attempt.
    pub duration_ms: Option<u64>,
    /// Who or what started the first attempt.
    pub started_by: StartedBy,
    /// Parameters the run used.
    pub params: serde_json::Value,
}

/// One attempt of a run (first try or a retry), for the run detail Attempts card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunAttempt {
    /// Run id of this attempt.
    pub run_id: String,
    /// `First try`, `Retry 1`, `Retry 2`, …
    pub label: String,
    /// When the attempt started (RFC 3339).
    pub started_at: String,
    /// Result of the attempt.
    pub result: RunStatus,
    /// Duration of the attempt.
    pub duration_ms: Option<u64>,
}

/// How a job's day went: its last run's final attempt worked, failed, or it didn't run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayResult {
    /// The day's last run worked (after retries, if any).
    Worked,
    /// The day's last run failed on its final attempt.
    Failed,
    /// No finished run that day.
    None,
}

/// One day of a job's last seven days.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobWeekDay {
    /// Calendar day (UTC), `YYYY-MM-DD`.
    pub date: String,
    /// Short label such as `Mon 24 Nov`.
    pub label: String,
    /// How the day went.
    pub result: DayResult,
}

/// A job's newest run when it failed, for the job page's failing banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastRunFailure {
    /// Run id of the failed run's first attempt.
    pub run_id: String,
    /// Banner title, such as `The 03:00 run failed`.
    pub title: String,
    /// What happened, such as `It stopped with an error after 4 attempts: …`.
    pub detail: String,
}

/// A job's last seven days and the line that sums up its recent runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobWeek {
    /// Oldest day first, today last.
    pub days: Vec<JobWeekDay>,
    /// Summary line, such as `Last run: failed today at 03:00 after 4 attempts.`
    pub summary: String,
    /// Set when the newest run failed.
    pub failing: Option<LastRunFailure>,
}

/// Everything the run detail page shows about one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDetail {
    /// The attempt itself.
    pub run: Run,
    /// Page title, such as `Run at 03:00 today`.
    pub title: String,
    /// Run id of the first attempt.
    pub root_run_id: String,
    /// Who or what started the run.
    pub started_by: StartedBy,
    /// Facts-grid label, such as `Schedule (every day at 03:00 UTC)`.
    pub started_by_label: String,
    /// Parameters this attempt used.
    pub params: serde_json::Value,
    /// The job's stored parameters, to show what differed.
    pub stored_params: serde_json::Value,
    /// Worker pool the run was placed in.
    pub pool: String,
    /// Caption under the Attempts card, such as `Retries run 30 s apart. The job allows 3.`
    pub retry_caption: String,
    /// True when the viewer may start the job again (Run again).
    pub can_run: bool,
}

/// What the Run now dialog needs to say before it runs a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunNowFacts {
    /// Job id.
    pub job_id: String,
    /// Job name.
    pub job_name: String,
    /// Script the job runs.
    pub script_name: String,
    /// The script's parameters from the registry signature.
    pub script_params: Vec<ScriptParam>,
    /// The job's stored parameters.
    pub stored_params: serde_json::Value,
    /// Worker pool label.
    pub pool: String,
    /// `Times out after` value, such as `5 min`; `None` when the job has no timeout.
    pub timeout: Option<String>,
    /// `If it fails` value, such as `Retries up to 3 times`.
    pub if_it_fails: String,
    /// Line about the schedule, such as `The schedule doesn't change. …`
    pub schedule_line: String,
    /// True when the job runs one at a time and a run is already going.
    pub waits_for_running: bool,
    /// False for platform jobs that run on schedule only.
    pub can_run_now: bool,
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
    /// Revision the editor opened; a newer stored revision makes the save a conflict.
    #[serde(default)]
    pub expected_revision: Option<u32>,
}

/// Result of saving a job configuration change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateJobOutcome {
    /// The change was saved as `revision`.
    Saved {
        /// Revision number the save produced.
        revision: u32,
        /// Next scheduled run after the save (RFC 3339), if any.
        next_run_at: Option<String>,
    },
    /// Someone saved a newer revision after the editor opened the job; nothing was written.
    Conflict {
        /// Latest stored revision number.
        revision: u32,
        /// Display label for who saved it.
        by: String,
        /// RFC 3339 time it was saved.
        at: String,
    },
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
