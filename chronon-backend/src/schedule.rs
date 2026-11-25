//! Create/update schedule helpers for Chronon job configuration.

use chronon_core::{Job as CoreJob, ScheduleKind};
use chronon_scheduler::CronExpr;
use std::fmt::Write as _;

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

// ── Schedule sentences, previews and field errors ─────────────────────────────

/// One field of a schedule form, named so a [`FieldError`] can sit on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleField {
    /// Optional leading seconds field of a six-field cron.
    Second,
    /// Minute field.
    Minute,
    /// Hour field.
    Hour,
    /// Day-of-month field.
    DayOfMonth,
    /// Month field.
    Month,
    /// Day-of-week field.
    DayOfWeek,
    /// The cron as a whole (field count, syntax the scheduler refuses).
    Cron,
    /// IANA timezone name.
    Timezone,
}

impl ScheduleField {
    const fn label(self) -> &'static str {
        match self {
            Self::Second => "Second",
            Self::Minute => "Minute",
            Self::Hour => "Hour",
            Self::DayOfMonth => "Day of month",
            Self::Month => "Month",
            Self::DayOfWeek => "Day of week",
            Self::Cron => "Cron",
            Self::Timezone => "Timezone",
        }
    }

    const fn range(self) -> (u32, u32) {
        match self {
            Self::Second | Self::Minute => (0, 59),
            Self::Hour => (0, 23),
            Self::DayOfMonth => (1, 31),
            Self::Month => (1, 12),
            Self::DayOfWeek => (0, 7),
            Self::Cron | Self::Timezone => (0, 0),
        }
    }
}

/// A schedule input problem tied to the field that caused it.
///
/// ```rust
/// use chronon_backend::{validate_cron, ScheduleField};
///
/// let err = validate_cron("0 24 * * *", "UTC").unwrap_err();
/// assert_eq!(err.field, ScheduleField::Hour);
/// assert_eq!(err.message, "Hour must be 0 to 23. You entered 24.");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldError {
    /// Field the message belongs on.
    pub field: ScheduleField,
    /// Plain-English reason, written for the person filling in the form.
    pub message: String,
}

impl FieldError {
    fn new(field: ScheduleField, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FieldError {}

/// Sentence, next runs and any field error for a cron the form is editing.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct SchedulePreview {
    /// Plain-English schedule in the job's timezone; empty when `error` is set.
    pub sentence: String,
    /// Upcoming run times as RFC 3339 UTC strings.
    pub next: Vec<String>,
    /// First problem found, tied to its field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<FieldError>,
}

/// Label used for jobs that only run from Run now.
pub const MANUAL_SCHEDULE_TEXT: &str = "Manual";

/// Who paused a job, when, and which runs it has skipped since.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PauseInfo {
    /// Display label for the actor that saved the pausing revision.
    pub by: String,
    /// RFC 3339 time of the pausing revision.
    pub at: String,
    /// Scheduled times skipped since the pause (RFC 3339, oldest first, at most five).
    pub skipped: Vec<String>,
}

const MONTH_NAMES: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
const DAY_NAMES: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const DAY_WORDS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_WORDS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Expanded cron field: every value it matches.
#[derive(Debug, Clone)]
struct Parsed {
    values: std::collections::BTreeSet<u32>,
}

impl Parsed {
    fn single(&self) -> Option<u32> {
        if self.values.len() == 1 {
            self.values.iter().next().copied()
        } else {
            None
        }
    }

    fn is_all(&self, field: ScheduleField) -> bool {
        let (lo, hi) = field.range();
        let hi = if field == ScheduleField::DayOfWeek {
            6
        } else {
            hi
        };
        (lo..=hi).all(|v| self.values.contains(&v))
    }

    /// `Some(n)` when the values are `lo, lo+n, lo+2n, …` across the whole range.
    fn every(&self, field: ScheduleField) -> Option<u32> {
        let (lo, hi) = field.range();
        let mut iter = self.values.iter().copied();
        let first = iter.next()?;
        if first != lo {
            return None;
        }
        let second = iter.next()?;
        let step = second - first;
        let expected: std::collections::BTreeSet<u32> = (lo..=hi).step_by(step as usize).collect();
        (expected == self.values).then_some(step)
    }

    fn contiguous(&self) -> Option<(u32, u32)> {
        let lo = *self.values.iter().next()?;
        let hi = *self.values.iter().next_back()?;
        (u32::try_from(self.values.len()).ok()? == hi - lo + 1).then_some((lo, hi))
    }
}

fn parse_number(raw: &str, field: ScheduleField) -> Result<u32, FieldError> {
    let upper = raw.to_ascii_uppercase();
    let named = match field {
        ScheduleField::Month => MONTH_NAMES.iter().position(|m| *m == upper).map(|i| i + 1),
        ScheduleField::DayOfWeek => DAY_NAMES.iter().position(|d| *d == upper),
        _ => None,
    };
    if let Some(n) = named {
        return u32::try_from(n).map_err(|_| FieldError::new(field, "Value is too large."));
    }
    let (lo, hi) = field.range();
    let label = field.label();
    let value: u32 = raw.parse().map_err(|_| {
        FieldError::new(
            field,
            format!("{label} must be a number from {lo} to {hi}. You entered {raw}."),
        )
    })?;
    if value < lo || value > hi {
        return Err(FieldError::new(
            field,
            format!("{label} must be {lo} to {hi}. You entered {value}."),
        ));
    }
    Ok(value)
}

fn parse_field(raw: &str, field: ScheduleField) -> Result<Parsed, FieldError> {
    let (lo, hi) = field.range();
    let label = field.label();
    let mut values = std::collections::BTreeSet::new();
    for part in raw.split(',') {
        if part.is_empty() {
            return Err(FieldError::new(
                field,
                format!("{label} has an empty entry. Remove the extra comma."),
            ));
        }
        let (base, step) = match part.split_once('/') {
            Some((base, step)) => {
                let step: u32 = step.parse().map_err(|_| {
                    FieldError::new(
                        field,
                        format!("{label} step must be a whole number. You entered {step}."),
                    )
                })?;
                if step == 0 {
                    return Err(FieldError::new(
                        field,
                        format!("{label} step must be at least 1."),
                    ));
                }
                (base, Some(step))
            }
            None => (part, None),
        };
        let (start, end) = if base == "*" {
            (lo, hi)
        } else if let Some((a, b)) = base.split_once('-') {
            let a = parse_number(a, field)?;
            let b = parse_number(b, field)?;
            if a > b {
                return Err(FieldError::new(
                    field,
                    format!("{label} range must go from low to high. You entered {base}."),
                ));
            }
            (a, b)
        } else {
            let a = parse_number(base, field)?;
            (a, if step.is_some() { hi } else { a })
        };
        let step = step.unwrap_or(1);
        values.extend((start..=end).step_by(step as usize));
    }
    if field == ScheduleField::DayOfWeek && values.remove(&7) {
        values.insert(0);
    }
    Ok(Parsed { values })
}

/// Cron fields in order, with the optional seconds field dropped.
struct Fields {
    second: Option<Parsed>,
    minute: Parsed,
    hour: Parsed,
    dom: Parsed,
    month: Parsed,
    dow: Parsed,
}

/// Uses scheduler-only syntax (`L`, `W`, `#`, `?`) that the field checks don't model.
fn has_extended_syntax(cron: &str) -> bool {
    cron.chars().any(|c| matches!(c, 'L' | 'W' | '#' | '?'))
        && !cron.to_ascii_uppercase().contains("JUL")
}

fn parse_fields(cron: &str) -> Result<Fields, FieldError> {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    let (second, rest) = match parts.len() {
        5 => (None, &parts[..]),
        6 => (
            Some(parse_field(parts[0], ScheduleField::Second)?),
            &parts[1..],
        ),
        0 => {
            return Err(FieldError::new(
                ScheduleField::Cron,
                "Enter a cron, such as 0 3 * * * for every day at 03:00.",
            ))
        }
        n => {
            return Err(FieldError::new(
                ScheduleField::Cron,
                format!(
                    "A cron needs five fields (minute hour day month weekday), or six with \
                     seconds first. You entered {n}."
                ),
            ))
        }
    };
    Ok(Fields {
        second,
        minute: parse_field(rest[0], ScheduleField::Minute)?,
        hour: parse_field(rest[1], ScheduleField::Hour)?,
        dom: parse_field(rest[2], ScheduleField::DayOfMonth)?,
        month: parse_field(rest[3], ScheduleField::Month)?,
        dow: parse_field(rest[4], ScheduleField::DayOfWeek)?,
    })
}

fn parse_cron(cron: &str, tz: &str) -> Result<CronExpr, FieldError> {
    if tz.trim().is_empty() {
        return Err(FieldError::new(ScheduleField::Timezone, "Pick a timezone."));
    }
    CronExpr::parse(cron.trim(), Some(tz.trim())).map_err(|e| match e {
        chronon_core::ChrononError::InvalidTimezone(name) => FieldError::new(
            ScheduleField::Timezone,
            format!("{name} isn't a timezone we know. Pick one from the list, such as UTC."),
        ),
        other => FieldError::new(ScheduleField::Cron, other.to_string()),
    })
}

/// Checks every cron field and the timezone, naming the first field that's wrong.
///
/// # Errors
///
/// Returns a [`FieldError`] for a wrong field count, an out-of-range or malformed field,
/// an unknown timezone, or anything else the Chronon scheduler refuses.
pub fn validate_cron(cron: &str, tz: &str) -> Result<(), FieldError> {
    if !has_extended_syntax(cron) {
        parse_fields(cron)?;
    }
    parse_cron(cron, tz).map(|_| ())
}

fn hhmm(hour: u32, minute: u32) -> String {
    format!("{hour:02}:{minute:02}")
}

fn join_words(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn day_word(d: u32) -> String {
    DAY_WORDS[d as usize % 7].to_string()
}

/// Weekday restriction as words, or `None` when every day is allowed.
fn weekday_words(dow: &Parsed) -> Option<String> {
    if dow.is_all(ScheduleField::DayOfWeek) {
        return None;
    }
    if let Some((a, b)) = dow.contiguous() {
        if b > a + 1 {
            return Some(format!("{} to {}", day_word(a), day_word(b)));
        }
    }
    let words: Vec<String> = dow.values.iter().map(|d| day_word(*d)).collect();
    Some(join_words(&words))
}

fn month_words(month: &Parsed) -> Option<String> {
    if month.is_all(ScheduleField::Month) {
        return None;
    }
    let words: Vec<String> = month
        .values
        .iter()
        .map(|m| MONTH_WORDS[(*m as usize).saturating_sub(1) % 12].to_string())
        .collect();
    Some(join_words(&words))
}

fn dom_words(dom: &Parsed) -> Option<String> {
    if dom.is_all(ScheduleField::DayOfMonth) {
        return None;
    }
    let words: Vec<String> = dom.values.iter().map(|d| ordinal(*d)).collect();
    Some(join_words(&words))
}

/// Time-of-day part: `(text, names a clock time, is a repeating interval)`.
fn time_phrase(f: &Fields) -> (String, bool, bool) {
    let minute = &f.minute;
    let hour = &f.hour;
    let hour_all = hour.is_all(ScheduleField::Hour);
    if let (Some(m), Some(h)) = (minute.single(), hour.single()) {
        return (format!("at {}", hhmm(h, m)), true, false);
    }
    if let Some(m) = minute.single() {
        if hour_all {
            return if m == 0 {
                ("Every hour".into(), false, true)
            } else {
                (format!("Every hour at {m} minutes past"), false, true)
            };
        }
        if let Some(n) = hour.every(ScheduleField::Hour) {
            let at = if m == 0 {
                String::new()
            } else {
                format!(" at {m} minutes past")
            };
            return (format!("Every {n} hours{at}"), false, true);
        }
        if let Some((a, b)) = hour.contiguous() {
            return (
                format!("Every hour from {} to {}", hhmm(a, m), hhmm(b, m)),
                true,
                true,
            );
        }
        if hour.values.len() <= 6 {
            let times: Vec<String> = hour.values.iter().map(|h| hhmm(*h, m)).collect();
            return (format!("at {}", join_words(&times)), true, false);
        }
    }
    if let Some(n) = minute.every(ScheduleField::Minute) {
        let every = if n == 1 {
            "Every minute".to_string()
        } else {
            format!("Every {n} minutes")
        };
        if hour_all {
            return (every, false, true);
        }
        if let Some((a, b)) = hour.contiguous() {
            let last_minute = minute.values.iter().next_back().copied().unwrap_or(0);
            return (
                format!("{every} from {} to {}", hhmm(a, 0), hhmm(b, last_minute)),
                true,
                true,
            );
        }
    }
    let minutes: Vec<String> = minute.values.iter().map(u32::to_string).collect();
    let hours = if hour_all {
        "every hour".to_string()
    } else {
        let hs: Vec<String> = hour.values.iter().map(|h| format!("{h:02}")).collect();
        format!("hour {}", join_words(&hs))
    };
    (
        format!("At minute {} past {hours}", join_words(&minutes)),
        !hour_all,
        true,
    )
}

fn describe_fields(f: &Fields, tz: &str) -> String {
    let (time, names_clock, interval) = time_phrase(f);
    let weekdays = weekday_words(&f.dow);
    let days = dom_words(&f.dom);
    let months = month_words(&f.month);
    let weekday_only = f.dow.values == (1..=5).collect();

    let mut sentence = if interval {
        let mut s = time;
        if let Some(d) = &days {
            let _ = write!(s, ", on the {d} of the month");
        }
        if let Some(w) = &weekdays {
            let _ = write!(s, ", {w}");
        }
        s
    } else {
        match (&days, &weekdays) {
            (None, None) => format!("Every day {time}"),
            (None, Some(_)) if weekday_only => format!("Every weekday {time}"),
            (None, Some(w)) => format!("Every {w} {time}"),
            (Some(d), None) => format!("Every month on the {d} {time}"),
            (Some(d), Some(w)) => format!("On the {d} of the month and every {w} {time}"),
        }
    };
    if let Some(m) = months {
        let _ = write!(sentence, ", in {m}");
    }
    if let Some(sec) = f.second.as_ref().filter(|s| s.single() != Some(0)) {
        let secs: Vec<String> = sec.values.iter().map(u32::to_string).collect();
        let _ = write!(sentence, ", at second {}", join_words(&secs));
    }
    if names_clock {
        if sentence.ends_with(|c: char| c.is_ascii_digit()) {
            sentence.push(' ');
        } else {
            sentence.push_str(", ");
        }
        sentence.push_str(tz);
    }
    sentence
}

/// Plain-English sentence for a five- or six-field cron in the job's timezone.
///
/// Clock times stay in `tz`, so the sentence names the zone whenever it mentions one.
///
/// # Errors
///
/// Returns the same [`FieldError`] as [`validate_cron`].
///
/// ```rust
/// use chronon_backend::describe;
///
/// assert_eq!(describe("0 3 * * *", "UTC").unwrap(), "Every day at 03:00 UTC");
/// assert_eq!(describe("*/5 * * * *", "UTC").unwrap(), "Every 5 minutes");
/// assert_eq!(describe("0 9 * * 1", "UTC").unwrap(), "Every Monday at 09:00 UTC");
/// ```
pub fn describe(cron: &str, tz: &str) -> Result<String, FieldError> {
    validate_cron(cron, tz)?;
    if has_extended_syntax(cron) {
        return Ok(format!("Custom schedule ({}), {}", cron.trim(), tz.trim()));
    }
    let fields = parse_fields(cron)?;
    Ok(describe_fields(&fields, tz.trim()))
}

/// The next `n` run times after `from`, computed by the Chronon scheduler itself.
///
/// Returns an empty list when the cron or timezone is invalid.
///
/// ```rust
/// use chrono::{TimeZone, Utc};
/// use chronon_backend::next_runs;
///
/// let from = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
/// let runs = next_runs("0 3 * * *", "UTC", from, 2);
/// assert_eq!(runs[0], Utc.with_ymd_and_hms(2026, 1, 2, 3, 0, 0).unwrap());
/// assert_eq!(runs.len(), 2);
/// ```
#[must_use]
pub fn next_runs(
    cron: &str,
    tz: &str,
    from: chrono::DateTime<chrono::Utc>,
    n: usize,
) -> Vec<chrono::DateTime<chrono::Utc>> {
    let Ok(expr) = parse_cron(cron, tz) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(n);
    let mut cursor = from;
    while out.len() < n {
        match expr.next_after(cursor) {
            Some(next) => {
                out.push(next);
                cursor = next;
            }
            None => break,
        }
    }
    out
}

/// Sentence, next `n` runs and any field error, for the schedule form preview.
#[must_use]
pub fn preview_schedule_at(
    cron: &str,
    tz: &str,
    from: chrono::DateTime<chrono::Utc>,
    n: usize,
) -> SchedulePreview {
    match describe(cron, tz) {
        Ok(sentence) => SchedulePreview {
            sentence,
            next: next_runs(cron, tz, from, n)
                .into_iter()
                .map(|t| t.to_rfc3339())
                .collect(),
            error: None,
        },
        Err(error) => SchedulePreview {
            error: Some(error),
            ..SchedulePreview::default()
        },
    }
}

/// Label for a run-once job: `Once on Wed, Oct 28 at 02:00 UTC`.
#[must_use]
pub fn describe_run_once(at: chrono::DateTime<chrono::Utc>) -> String {
    format!("Once on {}", at.format("%a, %b %-d at %H:%M UTC"))
}

/// Schedule sentence for a stored job (cron, run-once or manual).
#[must_use]
pub fn schedule_text_for_job(job: &CoreJob) -> String {
    match job.schedule_kind {
        ScheduleKind::Cron => {
            let Some(cron) = job.cron_expr.as_deref().filter(|c| !c.trim().is_empty()) else {
                return MANUAL_SCHEDULE_TEXT.to_string();
            };
            let tz = job.timezone.as_deref().unwrap_or("UTC");
            describe(cron, tz).unwrap_or_else(|_| cron.to_string())
        }
        ScheduleKind::RunOnce => job
            .run_once_at
            .map_or_else(|| MANUAL_SCHEDULE_TEXT.to_string(), describe_run_once),
        ScheduleKind::Manual => MANUAL_SCHEDULE_TEXT.to_string(),
    }
}

/// Short display label for a stored Valence actor JSON value.
///
/// System actors read "the platform"; users show their user id.
///
/// ```rust
/// use chronon_backend::actor_label;
///
/// assert_eq!(actor_label(&serde_json::json!({"System": {"operation": "x"}})), "the platform");
/// assert_eq!(actor_label(&serde_json::json!({"User": {"user_id": "u1"}})), "u1");
/// ```
#[must_use]
pub fn actor_label(actor: &serde_json::Value) -> String {
    if actor.get("System").is_some() {
        return "the platform".to_string();
    }
    if let Some(id) = actor
        .get("User")
        .and_then(|u| u.get("user_id"))
        .and_then(serde_json::Value::as_str)
    {
        return id.to_string();
    }
    if let Some(name) = actor
        .get("ServiceUser")
        .and_then(|u| u.get("service_name"))
        .and_then(serde_json::Value::as_str)
    {
        return name.to_string();
    }
    "someone".to_string()
}

/// True when a job's actor is the platform System actor.
#[must_use]
pub fn is_system_actor(actor: &serde_json::Value) -> bool {
    actor.get("System").is_some()
}

/// True when the job moved past the revision an editor opened.
#[must_use]
pub const fn revision_conflict(expected: Option<u32>, current: u32) -> bool {
    match expected {
        Some(expected) => current > expected,
        None => false,
    }
}

/// Finds who paused a job and which runs it skipped.
///
/// `revisions` are `(revision_number, changed_at, actor_json, snapshot_json)` in any order.
/// Returns `None` when the job is enabled or no revision records the pause.
#[must_use]
pub fn pause_info_from_revisions(
    job: &CoreJob,
    revisions: &[(
        u32,
        chrono::DateTime<chrono::Utc>,
        serde_json::Value,
        serde_json::Value,
    )],
    now: chrono::DateTime<chrono::Utc>,
) -> Option<PauseInfo> {
    if job.enabled {
        return None;
    }
    let mut sorted: Vec<_> = revisions.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.0));
    let snapshot_enabled =
        |snap: &serde_json::Value| snap.get("enabled").and_then(serde_json::Value::as_bool);
    let mut pausing = None;
    for rev in &sorted {
        match snapshot_enabled(&rev.3) {
            Some(false) => pausing = Some(*rev),
            Some(true) => break,
            None => {}
        }
    }
    let (_, at, actor, _) = pausing?;
    let skipped = match (&job.schedule_kind, job.cron_expr.as_deref()) {
        (ScheduleKind::Cron, Some(cron)) => {
            next_runs(cron, job.timezone.as_deref().unwrap_or("UTC"), *at, 50)
                .into_iter()
                .take_while(|t| *t <= now)
                .collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
    let start = skipped.len().saturating_sub(5);
    Some(PauseInfo {
        by: actor_label(actor),
        at: at.to_rfc3339(),
        skipped: skipped[start..]
            .iter()
            .map(chrono::DateTime::to_rfc3339)
            .collect(),
    })
}

#[cfg(test)]
mod describe_tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn describe_common_sentences_happy() {
        let cases = [
            ("0 3 * * *", "UTC", "Every day at 03:00 UTC"),
            ("*/5 * * * *", "UTC", "Every 5 minutes"),
            ("0 9 * * 1", "UTC", "Every Monday at 09:00 UTC"),
            (
                "*/30 1-5 * * 1-5",
                "UTC",
                "Every 30 minutes from 01:00 to 05:30, Monday to Friday, UTC",
            ),
            ("0 * * * *", "UTC", "Every hour"),
            ("* * * * *", "UTC", "Every minute"),
            ("0 9 * * 1-5", "UTC", "Every weekday at 09:00 UTC"),
            ("0 3 1 * *", "UTC", "Every month on the 1st at 03:00 UTC"),
            ("0 0 3 * * *", "UTC", "Every day at 03:00 UTC"),
            (
                "0 2 * * *",
                "America/Los_Angeles",
                "Every day at 02:00 America/Los_Angeles",
            ),
            ("0 */6 * * *", "UTC", "Every 6 hours"),
            (
                "0 9 * * MON,THU",
                "UTC",
                "Every Monday and Thursday at 09:00 UTC",
            ),
            (
                "0 9 1 JAN *",
                "UTC",
                "Every month on the 1st at 09:00, in January, UTC",
            ),
        ];
        for (cron, tz, want) in cases {
            assert_eq!(describe(cron, tz).unwrap(), want, "{cron}");
        }
    }

    #[test]
    fn field_range_errors_sad() {
        let cases = [
            (
                "60 3 * * *",
                ScheduleField::Minute,
                "Minute must be 0 to 59. You entered 60.",
            ),
            (
                "0 24 * * *",
                ScheduleField::Hour,
                "Hour must be 0 to 23. You entered 24.",
            ),
            (
                "0 3 32 * *",
                ScheduleField::DayOfMonth,
                "Day of month must be 1 to 31. You entered 32.",
            ),
            (
                "0 3 0 * *",
                ScheduleField::DayOfMonth,
                "Day of month must be 1 to 31. You entered 0.",
            ),
            (
                "0 3 * 13 *",
                ScheduleField::Month,
                "Month must be 1 to 12. You entered 13.",
            ),
            (
                "0 3 * * 8",
                ScheduleField::DayOfWeek,
                "Day of week must be 0 to 7. You entered 8.",
            ),
            (
                "61 0 3 * * *",
                ScheduleField::Second,
                "Second must be 0 to 59. You entered 61.",
            ),
            (
                "*/0 * * * *",
                ScheduleField::Minute,
                "Minute step must be at least 1.",
            ),
            (
                "0 5-2 * * *",
                ScheduleField::Hour,
                "Hour range must go from low to high. You entered 5-2.",
            ),
            (
                "x 3 * * *",
                ScheduleField::Minute,
                "Minute must be a number from 0 to 59. You entered x.",
            ),
        ];
        for (cron, field, msg) in cases {
            let err = validate_cron(cron, "UTC").unwrap_err();
            assert_eq!(err.field, field, "{cron}");
            assert_eq!(err.message, msg, "{cron}");
        }
        let err = validate_cron("0 3 * *", "UTC").unwrap_err();
        assert_eq!(err.field, ScheduleField::Cron);
        assert!(err.message.contains("You entered 4."));
        assert_eq!(
            validate_cron("", "UTC").unwrap_err().field,
            ScheduleField::Cron
        );
    }

    #[test]
    fn invalid_timezone_maps_to_timezone_field_sad() {
        let err = describe("0 3 * * *", "Mars/Olympus").unwrap_err();
        assert_eq!(err.field, ScheduleField::Timezone);
        assert!(err.message.contains("Mars/Olympus"));
        assert_eq!(
            validate_cron("0 3 * * *", " ").unwrap_err().field,
            ScheduleField::Timezone
        );
    }

    #[test]
    fn next_runs_follow_dst_in_los_angeles_happy() {
        // 2026-03-08 springs forward; 02:30 local doesn't exist that day.
        let from = Utc.with_ymd_and_hms(2026, 3, 6, 12, 0, 0).unwrap();
        let runs = next_runs("0 9 * * *", "America/Los_Angeles", from, 3);
        assert_eq!(runs[0], Utc.with_ymd_and_hms(2026, 3, 6, 17, 0, 0).unwrap());
        assert_eq!(runs[1], Utc.with_ymd_and_hms(2026, 3, 7, 17, 0, 0).unwrap());
        // After the change 09:00 PDT is 16:00 UTC.
        assert_eq!(runs[2], Utc.with_ymd_and_hms(2026, 3, 8, 16, 0, 0).unwrap());

        let from = Utc.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap();
        let runs = next_runs("0 9 * * *", "America/Los_Angeles", from, 2);
        // 09:00 PDT on Oct 31, then 09:00 PST once the clocks fall back on Nov 1.
        assert_eq!(
            runs[0],
            Utc.with_ymd_and_hms(2026, 10, 31, 16, 0, 0).unwrap()
        );
        assert_eq!(
            runs[1],
            Utc.with_ymd_and_hms(2026, 11, 1, 17, 0, 0).unwrap()
        );
    }

    #[test]
    fn next_runs_invalid_input_is_empty_sad() {
        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(next_runs("0 24 * * *", "UTC", from, 3).len(), 0);
        assert_eq!(next_runs("0 3 * * *", "Nowhere/City", from, 3).len(), 0);
    }

    #[test]
    fn preview_reports_sentence_runs_or_error() {
        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let ok = preview_schedule_at("0 4 * * *", "UTC", from, 5);
        assert_eq!(ok.sentence, "Every day at 04:00 UTC");
        assert_eq!(ok.next.len(), 5);
        assert_eq!(ok.next[0], "2026-01-01T04:00:00+00:00");
        let bad = preview_schedule_at("0 24 * * *", "UTC", from, 5);
        assert_eq!(bad.error.unwrap().field, ScheduleField::Hour);
        assert_eq!(bad.next.len(), 0);
    }

    #[test]
    fn job_schedule_text_labels_run_once_and_manual() {
        let mut job = CoreJob::new("once", "script");
        job.schedule_kind = ScheduleKind::RunOnce;
        job.run_once_at = Some(Utc.with_ymd_and_hms(2026, 10, 28, 2, 0, 0).unwrap());
        assert_eq!(
            schedule_text_for_job(&job),
            "Once on Wed, Oct 28 at 02:00 UTC"
        );
        job.schedule_kind = ScheduleKind::Manual;
        assert_eq!(schedule_text_for_job(&job), "Manual");
        job.schedule_kind = ScheduleKind::Cron;
        job.cron_expr = Some("0 3 * * *".into());
        job.timezone = Some("UTC".into());
        assert_eq!(schedule_text_for_job(&job), "Every day at 03:00 UTC");
    }

    #[test]
    fn actor_label_and_conflict() {
        assert_eq!(
            actor_label(&serde_json::json!({"System": {"operation": "boot"}})),
            "the platform"
        );
        assert_eq!(
            actor_label(&serde_json::json!({"User": {"user_id": "u-1"}})),
            "u-1"
        );
        assert_eq!(actor_label(&serde_json::Value::Null), "someone");
        assert!(revision_conflict(Some(3), 4));
        assert!(!revision_conflict(Some(4), 4));
        assert!(!revision_conflict(None, 9));
    }

    #[test]
    fn pause_info_names_actor_and_skipped_runs() {
        let mut job = CoreJob::new("purge", "script");
        job.schedule_kind = ScheduleKind::Cron;
        job.cron_expr = Some("0 3 * * *".into());
        job.timezone = Some("UTC".into());
        job.enabled = false;
        let t = |d, h| Utc.with_ymd_and_hms(2026, 1, d, h, 0, 0).unwrap();
        let user = serde_json::json!({"User": {"user_id": "ops-1"}});
        let revs = vec![
            (
                1,
                t(1, 0),
                user.clone(),
                serde_json::json!({"enabled": true}),
            ),
            (2, t(1, 12), user, serde_json::json!({"enabled": false})),
            (
                3,
                t(2, 1),
                serde_json::json!({"User": {"user_id": "ops-2"}}),
                serde_json::json!({"enabled": false}),
            ),
        ];
        let info = pause_info_from_revisions(&job, &revs, t(4, 12)).unwrap();
        assert_eq!(info.by, "ops-1");
        assert_eq!(info.at, t(1, 12).to_rfc3339());
        assert_eq!(info.skipped.len(), 3);
        job.enabled = true;
        assert!(pause_info_from_revisions(&job, &revs, t(4, 12)).is_none());
    }
}
