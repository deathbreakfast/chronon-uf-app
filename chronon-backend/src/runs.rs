//! Run summaries for the Chronon runs views.
//!
//! The UI shows a run as its first attempt with every retry folded in. These helpers decide
//! who or what started a run, group attempts into [`RunSummary`] rows, sum up a job's last
//! seven days, and build the sentences the runs views show. They are pure: callers load runs
//! and jobs from the coordinator first.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::hash::BuildHasher;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus, ScheduleKind};
use chronon_scheduler::CronExpr;
use orbital_paging::{FilterQuery, PageRequest};
use serde_json::Value;

use crate::map::chronon_run_status_to_ui;
use crate::schedule::actor_user_id;
use crate::types::{
    DayResult, JobWeek, JobWeekDay, LastRunFailure, RunAttempt, RunSummary, StartedBy,
};

/// A first attempt followed by its retries, oldest first. Never empty.
pub type RunGroup<'a> = Vec<&'a CoreRun>;

/// True when an actor snapshot belongs to the platform rather than a person (null or System).
#[must_use]
pub fn actor_is_system(actor_json: &Value) -> bool {
    actor_json.is_null() || actor_json.get("System").is_some()
}

/// The user id (or service name) of a person-shaped actor snapshot.
#[must_use]
pub fn actor_person_id(actor_json: &Value) -> Option<String> {
    actor_json
        .get("User")
        .and_then(|u| u.get("user_id"))
        .or_else(|| {
            actor_json
                .get("ServiceUser")
                .and_then(|u| u.get("service_name"))
        })
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// True when `at` is a fire time of the job's schedule: a cron tick, or its run-once time.
#[must_use]
pub fn is_schedule_fire_time(job: &CoreJob, at: DateTime<Utc>) -> bool {
    match job.schedule_kind {
        ScheduleKind::Cron => {
            if at.nanosecond() != 0 {
                return false;
            }
            let Some(expr) = job.cron_expr.as_deref() else {
                return false;
            };
            CronExpr::parse(expr, job.timezone.as_deref())
                .ok()
                .and_then(|cron| cron.next_after(at - Duration::seconds(1)))
                == Some(at)
        }
        ScheduleKind::RunOnce => job.run_once_at == Some(at),
        ScheduleKind::Manual => false,
    }
}

const fn is_retry(run: &CoreRun) -> bool {
    run.attempt > 1 || run.parent_run_id.is_some()
}

/// Decides who or what started one attempt.
///
/// A retry (attempt above 1, or a `parent_run_id`) is [`StartedBy::Retry`] of `root_run_id`.
/// A System or null actor is the schedule. A person's actor on a fire time of the job's
/// schedule is the schedule too, because scheduled runs of a job a person created carry that
/// person's actor. Any other run is that person's Run now, labelled with their display name
/// from `names` (keyed by user id), or their user id when `names` has none.
#[must_use]
pub fn started_by<S: BuildHasher>(
    run: &CoreRun,
    root_run_id: &str,
    job: Option<&CoreJob>,
    names: &HashMap<String, String, S>,
) -> StartedBy {
    if is_retry(run) {
        return StartedBy::Retry {
            of: root_run_id.to_string(),
        };
    }
    if actor_is_system(&run.actor_json) {
        return StartedBy::Schedule;
    }
    if job.is_some_and(|j| is_schedule_fire_time(j, run.scheduled_for)) {
        return StartedBy::Schedule;
    }
    let who = actor_user_id(&run.actor_json)
        .and_then(|id| names.get(id).cloned())
        .or_else(|| actor_person_id(&run.actor_json))
        .unwrap_or_else(|| "Someone".to_string());
    StartedBy::Person {
        label: format!("{who} (Run now)"),
    }
}

/// The distinct user ids in the runs' actor snapshots, for looking up display names.
///
/// Service users and System actors are left out: a service user's label is its name already.
#[must_use]
pub fn run_now_user_ids(runs: &[CoreRun]) -> Vec<String> {
    let mut ids: Vec<String> = runs
        .iter()
        .filter_map(|r| actor_user_id(&r.actor_json).map(str::to_string))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Groups attempts into runs: each first attempt with the retries that followed it.
///
/// A retry joins its `root_run_id`'s group, else its `parent_run_id`'s group, else the newest
/// earlier group of the same job whose last attempt is the one before it and failed. Groups
/// come back newest first.
#[must_use]
pub fn group_runs(runs: &[CoreRun]) -> Vec<RunGroup<'_>> {
    let mut sorted: Vec<&CoreRun> = runs.iter().collect();
    sorted.sort_by(|a, b| {
        a.scheduled_for
            .cmp(&b.scheduled_for)
            .then(a.attempt.cmp(&b.attempt))
    });

    let mut groups: Vec<RunGroup<'_>> = Vec::new();
    let mut group_of_run: HashMap<&str, usize> = HashMap::new();
    let mut groups_of_job: HashMap<&str, Vec<usize>> = HashMap::new();

    for run in sorted {
        let linked = run
            .root_run_id
            .as_deref()
            .filter(|root| *root != run.run_id)
            .and_then(|root| group_of_run.get(root).copied())
            .or_else(|| {
                run.parent_run_id
                    .as_deref()
                    .and_then(|parent| group_of_run.get(parent).copied())
            });
        let job_key = run.job_id.as_deref().unwrap_or("");
        let target = linked.or_else(|| {
            (run.attempt > 1)
                .then(|| {
                    groups_of_job.get(job_key).and_then(|indexes| {
                        indexes.iter().rev().copied().find(|&i| {
                            groups[i].last().is_some_and(|last| {
                                last.attempt == run.attempt - 1 && is_failed(last.status)
                            })
                        })
                    })
                })
                .flatten()
        });
        let index = target.unwrap_or_else(|| {
            groups.push(Vec::new());
            let i = groups.len() - 1;
            groups_of_job.entry(job_key).or_default().push(i);
            i
        });
        groups[index].push(run);
        group_of_run.insert(run.run_id.as_str(), index);
        if let Some(root) = run.root_run_id.as_deref() {
            group_of_run.entry(root).or_insert(index);
        }
    }

    groups.reverse();
    groups
}

/// The group (first attempt and retries) that contains `run_id`.
#[must_use]
pub fn find_run_group<'a>(runs: &'a [CoreRun], run_id: &str) -> Option<RunGroup<'a>> {
    group_runs(runs)
        .into_iter()
        .find(|group| group.iter().any(|r| r.run_id == run_id))
}

const fn is_failed(status: CoreRunStatus) -> bool {
    matches!(
        status,
        CoreRunStatus::Failed | CoreRunStatus::Timeout | CoreRunStatus::Canceled
    )
}

fn started_time(run: &CoreRun) -> DateTime<Utc> {
    run.started_at.unwrap_or(run.scheduled_for)
}

fn duration_u64(ms: Option<i64>) -> Option<u64> {
    ms.and_then(|d| u64::try_from(d).ok())
}

/// Builds the runs-table row for one group.
///
/// `result` is the final attempt's status. A retried run's duration is the average over the
/// attempts that recorded one. `names` labels a person's Run now, as in [`started_by`].
#[must_use]
pub fn summarize_group<S: BuildHasher>(
    group: &[&CoreRun],
    job: Option<&CoreJob>,
    names: &HashMap<String, String, S>,
) -> Option<RunSummary> {
    let root = *group.first()?;
    let last = *group.last()?;
    let durations: Vec<u64> = group
        .iter()
        .filter_map(|r| duration_u64(r.duration_ms))
        .collect();
    let duration_ms = if group.len() == 1 {
        duration_u64(root.duration_ms)
    } else if durations.is_empty() {
        None
    } else {
        Some(durations.iter().sum::<u64>() / durations.len() as u64)
    };
    Some(RunSummary {
        root_run_id: root.run_id.clone(),
        job_id: root.job_id.clone().unwrap_or_default(),
        job_name: job.map_or_else(|| root.script_name.clone(), |j| j.job_name.clone()),
        result: chronon_run_status_to_ui(last.status),
        attempts: u32::try_from(group.len()).unwrap_or(u32::MAX),
        started_at: started_time(root).to_rfc3339(),
        duration_ms,
        started_by: started_by(root, &root.run_id, job, names),
        params: root.params_json.clone(),
    })
}

/// Folds attempts into runs-table rows, newest first. Jobs resolve names and schedules;
/// `names` maps user ids to display names for Started by.
#[must_use]
pub fn fold_runs<S: BuildHasher>(
    runs: &[CoreRun],
    jobs: &[CoreJob],
    names: &HashMap<String, String, S>,
) -> Vec<RunSummary> {
    let by_id: HashMap<&str, &CoreJob> = jobs.iter().map(|j| (j.job_id.as_str(), j)).collect();
    group_runs(runs)
        .iter()
        .filter_map(|group| {
            let job = group
                .first()
                .and_then(|r| r.job_id.as_deref())
                .and_then(|id| by_id.get(id).copied());
            summarize_group(group, job, names)
        })
        .collect()
}

/// Lists a group's attempts for the run detail Attempts card: `First try`, `Retry 1`, …
#[must_use]
pub fn run_attempts(group: &[&CoreRun]) -> Vec<RunAttempt> {
    group
        .iter()
        .enumerate()
        .map(|(i, r)| RunAttempt {
            run_id: r.run_id.clone(),
            label: if i == 0 {
                "First try".to_string()
            } else {
                format!("Retry {i}")
            },
            started_at: started_time(r).to_rfc3339(),
            result: chronon_run_status_to_ui(r.status),
            duration_ms: duration_u64(r.duration_ms),
        })
        .collect()
}

/// Result filter on the runs table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunResultFilter {
    /// Every run.
    #[default]
    Any,
    /// Runs whose final attempt failed, timed out or was canceled.
    Failed,
    /// Runs whose final attempt worked.
    Worked,
    /// Runs still queued or running.
    Running,
}

impl RunResultFilter {
    /// Parses the URL / filter value (`any`, `failed`, `worked`, `running`).
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "failed" => Self::Failed,
            "worked" => Self::Worked,
            "running" => Self::Running,
            _ => Self::Any,
        }
    }

    /// True when a run with this final result passes the filter.
    #[must_use]
    pub const fn matches(self, result: crate::RunStatus) -> bool {
        use crate::RunStatus::{Cancelled, Completed, Failed, Pending, Running};
        match self {
            Self::Any => true,
            Self::Failed => matches!(result, Failed | Cancelled),
            Self::Worked => matches!(result, Completed),
            Self::Running => matches!(result, Running | Pending),
        }
    }
}

/// Time filter on the runs table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunTimeFilter {
    /// No time limit.
    #[default]
    Any,
    /// Runs that started in the last 24 hours.
    Last24Hours,
    /// Runs that started in the last 7 days.
    Last7Days,
    /// Runs that started in the last 30 days.
    Last30Days,
}

impl RunTimeFilter {
    /// Parses the URL / filter value (`24h`, `7d`, `30d`; anything else is no limit).
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "24h" => Self::Last24Hours,
            "7d" => Self::Last7Days,
            "30d" => Self::Last30Days,
            _ => Self::Any,
        }
    }

    /// Earliest start time that passes the filter.
    #[must_use]
    pub fn since(self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Any => None,
            Self::Last24Hours => Some(now - Duration::hours(24)),
            Self::Last7Days => Some(now - Duration::days(7)),
            Self::Last30Days => Some(now - Duration::days(30)),
        }
    }

    /// Phrase for captions: `in the last 24 hours`, or `in all` with no limit.
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Any => "in all",
            Self::Last24Hours => "in the last 24 hours",
            Self::Last7Days => "in the last 7 days",
            Self::Last30Days => "in the last 30 days",
        }
    }
}

/// Structured filters the runs table sends in [`PageRequest::filter`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunsTableFilters {
    /// `result` rule.
    pub result: RunResultFilter,
    /// `time` rule.
    pub time: RunTimeFilter,
    /// `job` rule: a job id.
    pub job_id: Option<String>,
}

impl RunsTableFilters {
    /// Reads the `result`, `time` and `job` rules from a filter query.
    #[must_use]
    pub fn from_filter(filter: Option<&FilterQuery>) -> Self {
        let mut out = Self::default();
        for rule in filter.iter().flat_map(|f| f.items.iter()) {
            let value = rule.value.display_string();
            match rule.field.as_str() {
                "result" => out.result = RunResultFilter::parse(&value),
                "time" => out.time = RunTimeFilter::parse(&value),
                "job" if !value.trim().is_empty() => out.job_id = Some(value.trim().to_string()),
                _ => {}
            }
        }
        out
    }
}

/// Applies quick search (job name or run id) and the structured filters to runs-table rows.
pub fn apply_run_summary_query(
    rows: &mut Vec<RunSummary>,
    request: &PageRequest,
    now: DateTime<Utc>,
) {
    let filters = RunsTableFilters::from_filter(request.filter.as_ref());
    let since = filters.time.since(now);
    let quick = request
        .quick_search
        .as_deref()
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    rows.retain(|row| {
        let quick_ok = quick.as_deref().is_none_or(|q| {
            row.job_name.to_lowercase().contains(q) || row.root_run_id.to_lowercase().contains(q)
        });
        let time_ok = since.is_none_or(|since| {
            DateTime::parse_from_rfc3339(&row.started_at)
                .is_ok_and(|t| t.with_timezone(&Utc) >= since)
        });
        let job_ok = filters
            .job_id
            .as_deref()
            .is_none_or(|job| row.job_id == job);
        quick_ok && time_ok && job_ok && filters.result.matches(row.result)
    });
}

/// Caption over the runs table, such as `117 runs in the last 24 hours. The 03:00 purge run
/// was retried 3 times.`
#[must_use]
pub fn runs_caption(rows: &[RunSummary], time: RunTimeFilter) -> String {
    let count = rows.len();
    let noun = if count == 1 { "run" } else { "runs" };
    let mut out = format!("{count} {noun} {}.", time.phrase());
    let retried: Vec<&RunSummary> = rows.iter().filter(|r| r.attempts > 1).collect();
    match retried.as_slice() {
        [] => {}
        [one] => {
            let at = DateTime::parse_from_rfc3339(&one.started_at)
                .map(|t| t.with_timezone(&Utc).format("%H:%M").to_string())
                .unwrap_or_default();
            let _ = write!(
                out,
                " The {at} {} run was retried {}.",
                one.job_name,
                times(one.attempts - 1)
            );
        }
        many => {
            let _ = write!(
                out,
                " {} of them were retried; each shows once, with its attempts.",
                many.len()
            );
        }
    }
    out
}

fn times(n: u32) -> String {
    match n {
        1 => "once".to_string(),
        2 => "twice".to_string(),
        n => format!("{n} times"),
    }
}

/// `today at 03:00`, `yesterday at 03:00`, `tomorrow at 03:00`, or `on 21 Nov at 03:00` (UTC).
#[must_use]
pub fn when_words(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = at.format("%H:%M");
    match (at.date_naive() - now.date_naive()).num_days() {
        0 => format!("today at {time}"),
        -1 => format!("yesterday at {time}"),
        1 => format!("tomorrow at {time}"),
        _ => format!("on {} at {time}", at.format("%-d %b")),
    }
}

/// Run detail title: `Run at 03:00 today`, `Run at 03:00 yesterday`, `Run at 03:00 on 21 Nov`.
#[must_use]
pub fn run_title(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = at.format("%H:%M");
    match (at.date_naive() - now.date_naive()).num_days() {
        0 => format!("Run at {time} today"),
        -1 => format!("Run at {time} yesterday"),
        _ => format!("Run at {time} on {}", at.format("%-d %b")),
    }
}

/// `15 ms`, `1.2 s`, `3 min 4 s`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn duration_words(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 60_000 && ms.is_multiple_of(1000) {
        format!("{} s", ms / 1000)
    } else if ms < 60_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        if secs == 0 {
            format!("{mins} min")
        } else {
            format!("{mins} min {secs} s")
        }
    }
}

/// Plain words for common cron schedules: `every day at 03:00 UTC`, `every hour`, …
///
/// Falls back to `on the schedule <expr>` for anything else.
#[must_use]
pub fn describe_cron(expr: &str, timezone: Option<&str>) -> String {
    let tz = timezone.filter(|t| !t.trim().is_empty()).unwrap_or("UTC");
    let fields: Vec<&str> = expr.split_whitespace().collect();
    // Drop a leading seconds field when it is a fixed `0`.
    let fields: &[&str] = match fields.as_slice() {
        [sec, rest @ ..] if rest.len() == 5 && *sec == "0" => rest,
        all => all,
    };
    let fallback = || format!("on the schedule {}", expr.trim());
    let [minute, hour, dom, month, dow] = fields else {
        return fallback();
    };
    if (*dom, *month, *dow) != ("*", "*", "*") {
        return fallback();
    }
    match (minute.parse::<u32>(), hour.parse::<u32>()) {
        (Ok(m), Ok(h)) if m < 60 && h < 24 => format!("every day at {h:02}:{m:02} {tz}"),
        (Ok(0), Err(_)) if *hour == "*" => "every hour".to_string(),
        (Ok(m), Err(_)) if *hour == "*" && m < 60 => format!("every hour at :{m:02}"),
        _ => minute
            .strip_prefix("*/")
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|_| *hour == "*")
            .map_or_else(fallback, |n| format!("every {n} minutes")),
    }
}

/// Facts-grid label for Started by: `Schedule (every day at 03:00 UTC)`, `Retry of the 03:00
/// run`, or the person.
#[must_use]
pub fn started_by_label(
    started_by: &StartedBy,
    job: Option<&CoreJob>,
    root_started: DateTime<Utc>,
) -> String {
    match started_by {
        StartedBy::Schedule => job
            .and_then(|j| j.cron_expr.as_deref().map(|c| (c, j.timezone.as_deref())))
            .map_or_else(
                || "Schedule".to_string(),
                |(cron, tz)| format!("Schedule ({})", describe_cron(cron, tz)),
            ),
        StartedBy::Retry { .. } => {
            format!("Retry of the {} run", root_started.format("%H:%M"))
        }
        StartedBy::Person { label } => label.clone(),
    }
}

/// `If it fails` value for the Run now dialog: `Retries up to 3 times`, `Retries once`, or
/// `Doesn't retry`.
#[must_use]
pub fn if_it_fails_words(job: &CoreJob) -> String {
    match job.retry_policy().max_attempts {
        0 => "Doesn't retry".to_string(),
        1 => "Retries once".to_string(),
        n => format!("Retries up to {n} times"),
    }
}

/// Caption under the Attempts card: `Retries run 30 s apart. The job allows 3.`
#[must_use]
pub fn retry_caption(job: Option<&CoreJob>) -> String {
    let Some(job) = job else {
        return String::new();
    };
    let policy = job.retry_policy();
    if policy.max_attempts == 0 {
        return "This job doesn't retry a failed run.".to_string();
    }
    let gap = duration_words(policy.base_delay_ms);
    let spacing = if (policy.backoff_multiplier - 1.0).abs() < f64::EPSILON {
        format!("Retries run {gap} apart.")
    } else {
        format!("The first retry waits {gap}; each later one waits longer.")
    };
    format!("{spacing} The job allows {}.", policy.max_attempts)
}

/// Line about the schedule in the Run now dialog.
#[must_use]
pub fn schedule_line(job: &CoreJob, now: DateTime<Utc>) -> String {
    let head = "The schedule doesn't change.";
    if !job.enabled {
        return format!("{head} The job is paused, so nothing else is scheduled.");
    }
    job.next_run_at.map_or_else(
        || format!("{head} This job only runs when someone starts it."),
        |next| {
            format!(
                "{head} The next scheduled run is still {} UTC.",
                when_words(next, now)
            )
        },
    )
}

fn day_result(group: &[&CoreRun]) -> Option<DayResult> {
    let last = group.last()?;
    match last.status {
        CoreRunStatus::Success => Some(DayResult::Worked),
        s if is_failed(s) => Some(DayResult::Failed),
        _ => None,
    }
}

/// Each day's result: the final attempt of that day's last finished run.
fn results_by_day(groups: &[RunGroup<'_>]) -> BTreeMap<NaiveDate, DayResult> {
    let mut latest: BTreeMap<NaiveDate, (DateTime<Utc>, DayResult)> = BTreeMap::new();
    for group in groups {
        let (Some(root), Some(result)) = (group.first(), day_result(group)) else {
            continue;
        };
        let at = root.scheduled_for;
        let entry = latest.entry(at.date_naive()).or_insert((at, result));
        if at >= entry.0 {
            *entry = (at, result);
        }
    }
    latest.into_iter().map(|(d, (_, r))| (d, r)).collect()
}

/// `Mon 24 Nov`.
#[must_use]
pub fn day_label(date: NaiveDate) -> String {
    format!("{} {} {}", date.weekday(), date.day(), date.format("%b"))
}

/// A job's last seven days (oldest first), the summary line and the failing banner.
///
/// `runs` are the job's attempts. A day worked when its last run's final attempt worked, so a
/// run that failed and then worked on retry counts as worked.
#[must_use]
pub fn job_week(runs: &[CoreRun], now: DateTime<Utc>) -> JobWeek {
    let groups = group_runs(runs);
    let by_day = results_by_day(&groups);
    let today = now.date_naive();
    let days = (0..7)
        .rev()
        .map(|back| {
            let date = today - Duration::days(back);
            JobWeekDay {
                date: date.to_string(),
                label: day_label(date),
                result: by_day.get(&date).copied().unwrap_or(DayResult::None),
            }
        })
        .collect();

    let Some(last) = groups.first() else {
        return JobWeek {
            days,
            summary: "This job hasn't run yet.".to_string(),
            failing: None,
        };
    };
    let root = last[0];
    let final_attempt = last[last.len() - 1];
    let when = when_words(root.scheduled_for, now);
    let attempts = last.len();
    let after = if attempts > 1 {
        format!(" after {attempts} attempts")
    } else {
        String::new()
    };
    let last_day = root.scheduled_for.date_naive();
    let streak_before = |from: NaiveDate| {
        let mut n = 0;
        let mut day = from;
        while by_day.get(&day) == Some(&DayResult::Worked) {
            n += 1;
            day -= Duration::days(1);
        }
        n
    };
    let (summary, failing) = match day_result(last) {
        Some(DayResult::Failed) => {
            let streak = streak_before(last_day - Duration::days(1));
            let mut s = format!("Last run: failed {when}{after}.");
            if streak > 0 {
                let _ = write!(
                    s,
                    " Before that it worked {streak} {} in a row.",
                    if streak == 1 { "day" } else { "days" }
                );
            }
            let error = final_attempt
                .error_json
                .as_ref()
                .and_then(|v| v.get("message"))
                .and_then(Value::as_str);
            let detail = match (error, attempts > 1) {
                (Some(e), true) => {
                    format!("It stopped with an error on all {attempts} attempts: {e}")
                }
                (Some(e), false) => format!("It stopped with an error: {e}"),
                (None, true) => format!("It failed on all {attempts} attempts."),
                (None, false) => "It failed.".to_string(),
            };
            let failing = LastRunFailure {
                run_id: root.run_id.clone(),
                title: format!("The {} run failed", root.scheduled_for.format("%H:%M")),
                detail,
            };
            (s, Some(failing))
        }
        Some(_) => {
            let streak = streak_before(last_day);
            let mut s = format!("Last run: worked {when}{after}.");
            if streak > 1 {
                let _ = write!(s, " It has worked {streak} days in a row.");
            }
            (s, None)
        }
        None => (
            format!("Last run: started {when} and is still going."),
            None,
        ),
    };
    JobWeek {
        days,
        summary,
        failing,
    }
}

/// Parses one typed script parameter from a form field.
///
/// Integers, floats and bools must parse; anything else is kept as text. A blank field is
/// refused when the parameter is required. Errors are field messages, such as
/// `Enter a whole number, such as 30.`
///
/// # Errors
///
/// Returns the field message when the value doesn't fit the parameter's type.
pub fn coerce_script_param(
    param_type: &str,
    raw: &str,
    required: bool,
) -> Result<Option<Value>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return if required {
            Err("Enter a value.".to_string())
        } else {
            Ok(None)
        };
    }
    let value = match param_type {
        "bool" => raw
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|_| "Enter true or false.".to_string())?,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => raw
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| "Enter a whole number, such as 30.".to_string())?,
        "f32" | "f64" => raw
            .parse::<f64>()
            .map(Value::from)
            .map_err(|_| "Enter a number, such as 1.5.".to_string())?,
        _ => Value::String(raw.to_string()),
    };
    Ok(Some(value))
}

/// A stored parameter value as a form field shows it (`30`, `true`, `cache`).
#[must_use]
pub fn param_field_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// When the job last ran and how its newest run ended (retries folded in).
#[must_use]
pub fn last_run_of(runs: &[CoreRun]) -> Option<(String, crate::RunStatus)> {
    let groups = group_runs(runs);
    let group = groups.first()?;
    let root = group.first()?;
    let last = group.last()?;
    Some((
        started_time(root).to_rfc3339(),
        chronon_run_status_to_ui(last.status),
    ))
}

#[cfg(test)]
#[path = "runs_tests.rs"]
mod tests;
