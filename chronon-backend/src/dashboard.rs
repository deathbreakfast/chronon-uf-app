//! Dashboard helpers for Chronon server functions: KPIs, Needs attention, Coming up and the
//! run-trend chart on the Runs page.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::hash::BuildHasher;

use chrono::{DateTime, Duration, NaiveTime, Timelike, Utc};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus, ScheduleKind};

use crate::pools::effective_job_pool;
use crate::runs::{group_runs, is_failed, started_time, when_words, RunGroup};
use crate::schedule::{next_runs, schedule_text_for_job, PauseInfo};
use crate::types::{
    Attention, AttentionItem, AttentionKind, ComingUpRun, DashboardChartPoint,
    DashboardChartSeries, DashboardStats,
};

fn count_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// UTC midnight at the start of `now`'s calendar day.
///
/// Uses [`NaiveTime::MIN`] so midnight construction cannot fail.
#[must_use]
pub fn utc_start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(now.date_naive().and_time(NaiveTime::MIN), Utc)
}

/// Aggregates dashboard KPIs from in-memory job + run lists.
///
/// A run and its retries count once, by the final attempt. "Today" starts at UTC midnight
/// of `now`; Worked (24h) covers the runs whose first attempt was in the 24 hours before
/// `now`.
#[must_use]
pub fn dashboard_stats_from_jobs_and_runs(
    jobs: &[CoreJob],
    runs: &[CoreRun],
    now: DateTime<Utc>,
) -> DashboardStats {
    let today_start = utc_start_of_day(now);
    let total_jobs = count_u32(jobs.len());
    let active_jobs = count_u32(jobs.iter().filter(|j| j.enabled).count());
    let paused_jobs = count_u32(jobs.iter().filter(|j| !j.enabled).count());

    // A run and its retries count once, by the final attempt (G13).
    let groups = group_runs(runs);
    let runs_today: Vec<&CoreRun> = groups
        .iter()
        .filter(|g| {
            g.first()
                .is_some_and(|root| root.scheduled_for >= today_start)
        })
        .filter_map(|g| g.last().copied())
        .collect();

    let total_runs_today = count_u32(runs_today.len());
    let successful_runs_today = count_u32(
        runs_today
            .iter()
            .filter(|r| matches!(r.status, CoreRunStatus::Success))
            .count(),
    );
    let failed_runs_today = count_u32(
        runs_today
            .iter()
            // A refused run never started, so it isn't a failure of the job.
            .filter(|r| {
                matches!(r.status, CoreRunStatus::Failed | CoreRunStatus::Timeout)
                    && crate::access::run_refusal(r).is_none()
            })
            .count(),
    );
    let running_now = count_u32(
        runs_today
            .iter()
            .filter(|r| matches!(r.status, CoreRunStatus::Running))
            .count(),
    );

    let newest = newest_run_by_job(&groups);
    let failing_jobs = count_u32(
        jobs.iter()
            .filter(|j| {
                newest
                    .get(j.job_id.as_str())
                    .is_some_and(|g| g.last().is_some_and(|r| is_failed(r.status)))
            })
            .count(),
    );

    DashboardStats {
        total_jobs,
        active_jobs,
        paused_jobs,
        total_runs_today,
        successful_runs_today,
        failed_runs_today,
        running_now,
        failing_jobs,
        worked_24h_permille: worked_permille(&groups, now - Duration::hours(24)),
    }
}

/// Share of finished runs since `since` that worked, in tenths of a percent, rounded.
fn worked_permille(groups: &[RunGroup<'_>], since: DateTime<Utc>) -> Option<u32> {
    let (mut worked, mut finished) = (0_u64, 0_u64);
    for group in groups {
        let (Some(root), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        if root.scheduled_for < since {
            continue;
        }
        if matches!(last.status, CoreRunStatus::Success) {
            worked += 1;
            finished += 1;
        } else if is_failed(last.status) {
            finished += 1;
        }
    }
    (finished > 0).then(|| u32::try_from((worked * 1000 + finished / 2) / finished).unwrap_or(1000))
}

/// Each job's newest run (first attempt and retries), keyed by job id.
fn newest_run_by_job<'a>(groups: &'a [RunGroup<'a>]) -> HashMap<&'a str, &'a RunGroup<'a>> {
    let mut newest = HashMap::new();
    // Groups come newest first, so the first one seen for a job is its newest.
    for group in groups {
        if let Some(job_id) = group.first().and_then(|r| r.job_id.as_deref()) {
            newest.entry(job_id).or_insert(group);
        }
    }
    newest
}

/// `at 03:00` today, `yesterday at 03:00`, or `on Oct 20 at 03:00` (UTC).
fn at_words(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = at.format("%H:%M");
    match (at.date_naive() - now.date_naive()).num_days() {
        0 => format!("at {time}"),
        -1 => format!("yesterday at {time}"),
        _ => format!("on {} at {time}", at.format("%b %-d")),
    }
}

/// `today's 09:00 run`, `yesterday's 09:00 run`, `Monday's 09:00 run` within the last week,
/// else `the Oct 20 09:00 run` (UTC).
fn run_of_day(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = at.format("%H:%M");
    match (now.date_naive() - at.date_naive()).num_days() {
        0 => format!("today's {time} run"),
        1 => format!("yesterday's {time} run"),
        2..=6 => format!("{}'s {time} run", at.format("%A")),
        _ => format!("the {} {time} run", at.format("%b %-d")),
    }
}

/// `today`, `yesterday`, or `on Oct 20` (UTC).
fn day_words(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    match (now.date_naive() - at.date_naive()).num_days() {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        _ => format!("on {}", at.format("%b %-d")),
    }
}

fn error_message(run: &CoreRun) -> Option<String> {
    run.error_json
        .as_ref()
        .and_then(|v| v.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(|m| m.trim().trim_end_matches('.').to_string())
        .filter(|m| !m.is_empty())
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The job's next scheduled run after `now`, from `next_run_at` or its schedule.
fn next_scheduled_run(job: &CoreJob, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !job.enabled {
        return None;
    }
    job.next_run_at
        .filter(|n| *n > now)
        .or_else(|| match job.schedule_kind {
            ScheduleKind::Cron => job.cron_expr.as_deref().and_then(|cron| {
                next_runs(cron, job.timezone.as_deref().unwrap_or("UTC"), now, 1)
                    .into_iter()
                    .next()
            }),
            ScheduleKind::RunOnce => job
                .run_once_at
                .filter(|at| *at > now && job.run_once_completed_at.is_none()),
            ScheduleKind::Manual => None,
        })
}

/// The Failed line for a job whose newest run failed on every attempt.
fn failed_item(job: &CoreJob, group: &RunGroup<'_>, now: DateTime<Utc>) -> Option<AttentionItem> {
    let root = *group.first()?;
    let last = *group.last()?;
    if !is_failed(last.status) {
        return None;
    }
    let retries = group.len() - 1;
    let errors: Vec<Option<String>> = group.iter().map(|r| error_message(r)).collect();
    let same_error = errors.windows(2).all(|w| w[0] == w[1]);
    let mut detail = match (error_message(last), retries) {
        (Some(e), 0) => format!("It stopped with an error: {e}."),
        (None, 0) => "It failed.".to_string(),
        (Some(e), 1) if same_error => format!("The retry failed the same way: {e}."),
        (Some(e), n) if same_error => format!("All {n} retries failed the same way: {e}."),
        (Some(e), _) => format!(
            "It failed on all {} attempts. The last one stopped with: {e}.",
            group.len()
        ),
        (None, _) => format!("It failed on all {} attempts.", group.len()),
    };
    match next_scheduled_run(job, now) {
        Some(next) => {
            let _ = write!(
                detail,
                " The next try is {} unless someone runs it.",
                when_words(next, now)
            );
        }
        None => detail.push_str(" It won't try again until someone runs it."),
    }
    Some(AttentionItem {
        kind: AttentionKind::Failed,
        job_id: job.job_id.clone(),
        job_name: job.job_name.clone(),
        title: format!(
            "{} failed {}",
            job.job_name,
            at_words(started_time(root), now)
        ),
        detail,
        run_id: Some(root.run_id.clone()),
    })
}

/// The Paused line: who paused the job, when, and the runs it skipped.
fn paused_item(job: &CoreJob, pause: Option<&PauseInfo>, now: DateTime<Utc>) -> AttentionItem {
    let detail = pause.map_or_else(
        || "It won't run until someone resumes it.".to_string(),
        |info| {
            let paused_at = DateTime::parse_from_rfc3339(&info.at)
                .map(|t| t.with_timezone(&Utc))
                .unwrap_or(now);
            let mut detail = format!(
                "{} paused it {}.",
                capitalize(&info.by),
                day_words(paused_at, now)
            );
            let skipped: Vec<DateTime<Utc>> = info
                .skipped
                .iter()
                .filter_map(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|t| t.with_timezone(&Utc))
                .collect();
            match (skipped.last(), skipped.len()) {
                (Some(last), 1) => {
                    let _ = write!(detail, " It skipped {}.", run_of_day(*last, now));
                }
                (Some(last), n) => {
                    let count = if n >= 5 {
                        "at least 5".to_string()
                    } else {
                        n.to_string()
                    };
                    let _ = write!(
                        detail,
                        " It skipped {count} runs; the last was {}.",
                        run_of_day(*last, now)
                    );
                }
                (None, _) if matches!(job.schedule_kind, ScheduleKind::Cron) => {
                    detail.push_str(" It hasn't skipped a run yet.");
                }
                (None, _) => {}
            }
            detail
        },
    );
    AttentionItem {
        kind: AttentionKind::Paused,
        job_id: job.job_id.clone(),
        job_name: job.job_name.clone(),
        title: format!("{} is paused", job.job_name),
        detail,
        run_id: None,
    }
}

/// The Missed line for an active job whose `next_run_at` is more than one tick in the past
/// with no run at or after it.
fn missed_item(
    job: &CoreJob,
    groups: &[RunGroup<'_>],
    now: DateTime<Utc>,
    tick: Duration,
) -> Option<AttentionItem> {
    if !job.enabled
        || matches!(job.schedule_kind, ScheduleKind::Manual)
        || job.run_once_completed_at.is_some()
    {
        return None;
    }
    let due = job.next_run_at?;
    if due + tick >= now {
        return None;
    }
    let ran = groups.iter().any(|g| {
        g.first().is_some_and(|root| {
            root.job_id.as_deref() == Some(job.job_id.as_str()) && root.scheduled_for >= due
        })
    });
    if ran {
        return None;
    }
    Some(AttentionItem {
        kind: AttentionKind::Missed,
        job_id: job.job_id.clone(),
        job_name: job.job_name.clone(),
        title: format!("{} missed {}", job.job_name, run_of_day(due, now)),
        detail: format!(
            "It should have started {} and nothing ran. Check that the scheduler is up and a worker serves the {} pool.",
            when_words(due, now),
            effective_job_pool(job.pool.as_deref())
        ),
        run_id: None,
    })
}

/// The dashboard's Needs attention list.
///
/// A job is listed when its newest run failed on every attempt (a run that worked on a
/// retry is not), when it's paused, or when it missed a run: it's active and its
/// `next_run_at` is more than `tick` in the past with no run for it. Failed jobs come
/// first, then paused, then missed; each group is sorted by job name.
///
/// `pauses` maps job id to who paused it (from [`crate::pause_info_from_revisions`]).
///
/// ```rust
/// use std::collections::HashMap;
/// use chrono::{Duration, TimeZone, Utc};
/// use chronon_backend::{attention_from, AttentionKind};
/// use chronon_core::Job;
///
/// let mut job = Job::new("export-weekly-report", "reports.export");
/// job.enabled = false;
/// let now = Utc.with_ymd_and_hms(2026, 11, 24, 9, 0, 0).unwrap();
/// let attention = attention_from(&[job], &[], &HashMap::new(), now, Duration::seconds(1));
/// assert_eq!(attention.items[0].kind, AttentionKind::Paused);
/// assert_eq!(attention.items[0].title, "export-weekly-report is paused");
/// ```
#[must_use]
pub fn attention_from<S: BuildHasher>(
    jobs: &[CoreJob],
    runs: &[CoreRun],
    pauses: &HashMap<String, PauseInfo, S>,
    now: DateTime<Utc>,
    tick: Duration,
) -> Attention {
    let groups = group_runs(runs);
    let newest = newest_run_by_job(&groups);
    let mut sorted: Vec<&CoreJob> = jobs.iter().collect();
    sorted.sort_by(|a, b| a.job_name.cmp(&b.job_name));

    let failed_lines = sorted.iter().filter_map(|job| {
        newest
            .get(job.job_id.as_str())
            .and_then(|group| failed_item(job, group, now))
    });
    let paused_lines = sorted
        .iter()
        .filter(|job| !job.enabled)
        .map(|job| paused_item(job, pauses.get(&job.job_id), now));
    let missed_lines = sorted
        .iter()
        .filter_map(|job| missed_item(job, &groups, now, tick));

    let today_start = utc_start_of_day(now);
    Attention {
        items: failed_lines
            .chain(paused_lines)
            .chain(missed_lines)
            .collect(),
        runs_today: count_u32(
            groups
                .iter()
                .filter(|g| g.first().is_some_and(|r| r.scheduled_for >= today_start))
                .count(),
        ),
    }
}

/// The scheduler's tick period (`CHRONON_TICK_INTERVAL_MS`, as the scheduler reads it), the
/// grace [`attention_from`] gives a run before calling it missed.
#[must_use]
pub fn scheduler_tick() -> Duration {
    Duration::milliseconds(
        i64::try_from(chronon_scheduler::tick_interval_ms_from_env()).unwrap_or(i64::MAX),
    )
}

/// `Today, 09:40`, `Yesterday, 03:00`, `Tomorrow, 02:00`, or `Wed, Nov 26, 03:00` (UTC).
///
/// ```rust
/// use chrono::{TimeZone, Utc};
/// use chronon_backend::day_time_label;
///
/// let now = Utc.with_ymd_and_hms(2026, 11, 24, 9, 36, 0).unwrap();
/// let at = Utc.with_ymd_and_hms(2026, 11, 25, 2, 0, 0).unwrap();
/// assert_eq!(day_time_label(at, now), "Tomorrow, 02:00");
/// ```
#[must_use]
pub fn day_time_label(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = at.format("%H:%M");
    match (at.date_naive() - now.date_naive()).num_days() {
        0 => format!("Today, {time}"),
        -1 => format!("Yesterday, {time}"),
        1 => format!("Tomorrow, {time}"),
        _ => format!("{}, {time}", at.format("%a, %b %-d")),
    }
}

/// `in 4 min`, `in 14 h`, `in 3 days`.
fn in_words(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (at - now).num_seconds().max(0);
    if secs < 3_600 {
        format!("in {} min", (secs / 60).max(1))
    } else if secs < 48 * 3_600 {
        format!("in {} h", secs / 3_600)
    } else {
        format!("in {} days", secs / 86_400)
    }
}

/// The next `n` runs across every active job, soonest first.
///
/// Cron jobs use the scheduler's own [`next_runs`]; a run-once job counts until it has run.
/// Ties sort by job name.
///
/// ```rust
/// use chrono::{TimeZone, Utc};
/// use chronon_backend::coming_up_from;
/// use chronon_core::Job;
///
/// let mut job = Job::new("purge-expired-sessions", "purge_sessions");
/// job.cron_expr = Some("0 3 * * *".into());
/// let now = Utc.with_ymd_and_hms(2026, 11, 24, 9, 0, 0).unwrap();
/// let next = coming_up_from(&[job], &[], now, 5);
/// assert_eq!(next.len(), 5);
/// assert_eq!(next[0].when, "Tomorrow, 03:00");
/// assert_eq!(next[0].schedule_text, "Every day at 03:00 UTC");
/// assert!(next[0].first_run);
/// ```
#[must_use]
pub fn coming_up_from(
    jobs: &[CoreJob],
    runs: &[CoreRun],
    now: DateTime<Utc>,
    n: usize,
) -> Vec<ComingUpRun> {
    let has_run: HashSet<&str> = runs.iter().filter_map(|r| r.job_id.as_deref()).collect();
    let mut upcoming: Vec<(DateTime<Utc>, &CoreJob)> = jobs
        .iter()
        .filter(|job| job.enabled)
        .flat_map(|job| {
            let times = match job.schedule_kind {
                ScheduleKind::Cron => job.cron_expr.as_deref().map_or_else(Vec::new, |cron| {
                    next_runs(cron, job.timezone.as_deref().unwrap_or("UTC"), now, n)
                }),
                ScheduleKind::RunOnce => job
                    .run_once_at
                    .filter(|at| *at > now && job.run_once_completed_at.is_none())
                    .into_iter()
                    .collect(),
                ScheduleKind::Manual => Vec::new(),
            };
            times.into_iter().map(move |at| (at, job))
        })
        .collect();
    upcoming.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.job_name.cmp(&b.1.job_name)));
    upcoming
        .into_iter()
        .take(n)
        .map(|(at, job)| ComingUpRun {
            at: at.to_rfc3339(),
            when: day_time_label(at, now),
            in_words: in_words(at, now),
            job_id: job.job_id.clone(),
            job_name: job.job_name.clone(),
            schedule_text: schedule_text_for_job(job),
            first_run: !has_run.contains(job.job_id.as_str()),
        })
        .collect()
}

/// Bucket width for the dashboard run-outcomes chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunBucketGranularity {
    /// Hourly buckets for ≤24h ranges.
    Hourly,
    /// Daily buckets for longer ranges.
    Daily,
}

/// Chooses hourly buckets for ≤24h ranges and daily buckets otherwise.
#[must_use]
pub const fn run_bucket_granularity(range_secs: i64) -> RunBucketGranularity {
    if range_secs <= 86_400 {
        RunBucketGranularity::Hourly
    } else {
        RunBucketGranularity::Daily
    }
}

/// Floors a timestamp to the start of its chart bucket.
#[must_use]
pub fn align_run_bucket(ts: DateTime<Utc>, bucket: RunBucketGranularity) -> DateTime<Utc> {
    let naive = ts.naive_utc();
    match bucket {
        RunBucketGranularity::Hourly => {
            let hour = naive
                .date()
                .and_hms_opt(naive.hour(), 0, 0)
                .unwrap_or(naive);
            DateTime::<Utc>::from_naive_utc_and_offset(hour, Utc)
        }
        RunBucketGranularity::Daily => {
            let day = naive.date().and_hms_opt(0, 0, 0).unwrap_or(naive);
            DateTime::<Utc>::from_naive_utc_and_offset(day, Utc)
        }
    }
}

/// Builds successful/failed chart series from a bounded run list.
///
/// A run and its retries count once, by the final attempt, in the first attempt's bucket.
///
/// Failed series includes Failed / Timeout / Canceled outcomes. Running and
/// queued/claimed attempts are ignored.
#[must_use]
pub fn run_stats_series_from_runs(
    runs: &[CoreRun],
    now: DateTime<Utc>,
    range_secs: i64,
) -> Vec<DashboardChartSeries> {
    let since = now - chrono::Duration::seconds(range_secs);
    let bucket = run_bucket_granularity(range_secs);

    let mut success_buckets: BTreeMap<DateTime<Utc>, u32> = BTreeMap::new();
    let mut failed_buckets: BTreeMap<DateTime<Utc>, u32> = BTreeMap::new();

    // Bucket each run (retries folded in) by its first attempt, counted by its final one.
    let groups = group_runs(runs);
    for group in &groups {
        let (Some(root), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        if root.scheduled_for < since {
            continue;
        }
        let bucket_ts = align_run_bucket(root.scheduled_for, bucket);
        match last.status {
            CoreRunStatus::Success => {
                *success_buckets.entry(bucket_ts).or_insert(0) += 1;
            }
            CoreRunStatus::Failed | CoreRunStatus::Timeout | CoreRunStatus::Canceled
                if crate::access::run_refusal(last).is_none() =>
            {
                *failed_buckets.entry(bucket_ts).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let success_points: Vec<DashboardChartPoint> = success_buckets
        .into_iter()
        .map(|(ts, value)| DashboardChartPoint {
            ts,
            value: f64::from(value),
        })
        .collect();
    let failed_points: Vec<DashboardChartPoint> = failed_buckets
        .into_iter()
        .map(|(ts, value)| DashboardChartPoint {
            ts,
            value: f64::from(value),
        })
        .collect();

    vec![
        DashboardChartSeries {
            id: "successful".into(),
            label: "Successful".into(),
            points: success_points,
        },
        DashboardChartSeries {
            id: "failed".into(),
            label: "Failed".into(),
            points: failed_points,
        },
    ]
}

#[cfg(test)]
mod attention_tests {
    use std::collections::HashMap;

    use chrono::{DateTime, Duration, TimeZone, Utc};
    use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};
    use serde_json::json;

    use super::{attention_from, coming_up_from, dashboard_stats_from_jobs_and_runs};
    use crate::schedule::PauseInfo;
    use crate::types::AttentionKind;

    const TICK: Duration = Duration::seconds(1);

    fn at(day: u32, h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, h, m, 0).unwrap()
    }

    fn now() -> DateTime<Utc> {
        at(24, 9, 36)
    }

    fn job(id: &str, cron: &str) -> CoreJob {
        let mut job = CoreJob::new(id, "script");
        job.job_id = id.into();
        job.cron_expr = Some(cron.into());
        job.next_run_at = crate::next_runs(cron, "UTC", now(), 1).first().copied();
        job
    }

    fn attempt(job_id: &str, id: &str, when: DateTime<Utc>, n: i32, ok: bool) -> CoreRun {
        let mut run = CoreRun::for_job(job_id, "script", when);
        run.run_id = id.into();
        run.attempt = n;
        run.started_at = Some(when);
        if ok {
            run.status = CoreRunStatus::Success;
        } else {
            run.status = CoreRunStatus::Failed;
            run.error_json = Some(json!({"message": "Valence refused the connection."}));
        }
        run
    }

    /// The 03:00 purge failed on its first try and on all three retries.
    fn failed_purge() -> Vec<CoreRun> {
        (1..=4)
            .map(|n| {
                attempt(
                    "purge-expired-sessions",
                    &format!("purge-{n}"),
                    at(24, 3, 0) + Duration::seconds(30 * i64::from(n - 1)),
                    n,
                    false,
                )
            })
            .collect()
    }

    #[test]
    fn attention_failed_run_folds_retries_happy() {
        let jobs = [job("purge-expired-sessions", "0 3 * * *")];
        let attention = attention_from(&jobs, &failed_purge(), &HashMap::new(), now(), TICK);
        assert_eq!(attention.items.len(), 1);
        let item = &attention.items[0];
        assert_eq!(item.kind, AttentionKind::Failed);
        assert_eq!(item.title, "purge-expired-sessions failed at 03:00");
        assert_eq!(
            item.detail,
            "All 3 retries failed the same way: Valence refused the connection. \
             The next try is tomorrow at 03:00 unless someone runs it."
        );
        assert_eq!(item.run_id.as_deref(), Some("purge-1"));
        assert_eq!(attention.runs_today, 1);
    }

    #[test]
    fn attention_failed_then_worked_on_retry_is_not_listed_sad() {
        let jobs = [job("purge-expired-sessions", "0 3 * * *")];
        let mut runs = failed_purge();
        runs.truncate(2);
        runs.push(attempt(
            "purge-expired-sessions",
            "purge-3",
            at(24, 3, 1),
            3,
            true,
        ));
        let attention = attention_from(&jobs, &runs, &HashMap::new(), now(), TICK);
        assert!(attention.items.is_empty(), "{:?}", attention.items);
    }

    #[test]
    fn attention_paused_names_who_and_skipped_run_happy() {
        let mut paused = job("export-weekly-report", "0 9 * * 1");
        paused.enabled = false;
        let records = HashMap::from([(
            "export-weekly-report".to_string(),
            PauseInfo {
                by: "ines".into(),
                at: at(20, 10, 0).to_rfc3339(),
                skipped: vec![at(23, 9, 0).to_rfc3339()],
            },
        )]);
        let attention = attention_from(&[paused], &[], &records, now(), TICK);
        assert_eq!(attention.items.len(), 1);
        let item = &attention.items[0];
        assert_eq!(item.kind, AttentionKind::Paused);
        assert_eq!(item.title, "export-weekly-report is paused");
        assert_eq!(
            item.detail,
            "Ines paused it on Nov 20. It skipped yesterday's 09:00 run."
        );
        assert!(item.run_id.is_none());
    }

    #[test]
    fn attention_missed_run_past_one_tick_happy() {
        let mut late = job("nightly-thumbnails", "0 2 * * *");
        late.next_run_at = Some(at(24, 2, 0));
        let attention = attention_from(&[late], &[], &HashMap::new(), now(), TICK);
        assert_eq!(attention.items.len(), 1);
        assert_eq!(attention.items[0].kind, AttentionKind::Missed);
        assert_eq!(
            attention.items[0].title,
            "nightly-thumbnails missed today's 02:00 run"
        );
    }

    #[test]
    fn attention_missed_ignores_runs_made_and_due_within_a_tick_sad() {
        let mut ran = job("nightly-thumbnails", "0 2 * * *");
        ran.next_run_at = Some(at(24, 2, 0));
        let runs = [attempt("nightly-thumbnails", "t-1", at(24, 2, 0), 1, true)];
        let mut due_now = job("bot-score-bumper", "*/5 * * * *");
        due_now.next_run_at = Some(now() - Duration::milliseconds(500));
        let attention = attention_from(&[ran, due_now], &runs, &HashMap::new(), now(), TICK);
        assert!(attention.items.is_empty(), "{:?}", attention.items);
    }

    #[test]
    fn attention_empty_when_all_worked_happy() {
        let jobs = [job("purge-expired-sessions", "0 3 * * *")];
        let runs = [
            attempt("purge-expired-sessions", "p-1", at(24, 3, 0), 1, true),
            attempt("purge-expired-sessions", "p-0", at(23, 3, 0), 1, true),
        ];
        let attention = attention_from(&jobs, &runs, &HashMap::new(), now(), TICK);
        assert_eq!(attention.items, []);
        assert_eq!(attention.runs_today, 1);
    }

    #[test]
    fn attention_orders_failed_then_paused() {
        let mut paused = job("a-paused", "0 9 * * 1");
        paused.enabled = false;
        let jobs = [paused, job("purge-expired-sessions", "0 3 * * *")];
        let attention = attention_from(&jobs, &failed_purge(), &HashMap::new(), now(), TICK);
        let kinds: Vec<_> = attention.items.iter().map(|i| i.kind).collect();
        assert_eq!(kinds, [AttentionKind::Failed, AttentionKind::Paused]);
        assert_eq!(
            attention.items[1].detail,
            "It won't run until someone resumes it."
        );
    }

    #[test]
    fn coming_up_merges_jobs_soonest_first_happy() {
        let mut paused = job("paused", "* * * * *");
        paused.enabled = false;
        let jobs = [
            job("purge-expired-sessions", "0 3 * * *"),
            job("bot-score-bumper", "*/5 * * * *"),
            paused,
        ];
        let runs = [attempt("bot-score-bumper", "b-1", at(24, 9, 35), 1, true)];
        let next = coming_up_from(&jobs, &runs, now(), 5);
        assert_eq!(next.len(), 5);
        assert_eq!(next[0].job_name, "bot-score-bumper");
        assert_eq!(next[0].when, "Today, 09:40");
        assert_eq!(next[0].in_words, "in 4 min");
        assert_eq!(next[0].schedule_text, "Every 5 minutes");
        assert!(!next[0].first_run);
        assert!(next.iter().all(|r| r.job_name != "paused"));
        let at_times: Vec<_> = next.iter().map(|r| r.at.clone()).collect();
        let mut sorted = at_times.clone();
        sorted.sort();
        assert_eq!(at_times, sorted);
    }

    #[test]
    fn coming_up_first_run_and_truncation_sad() {
        let jobs = [job("purge-expired-sessions", "0 3 * * *")];
        let next = coming_up_from(&jobs, &[], now(), 2);
        assert_eq!(next.len(), 2);
        assert_eq!(next[0].when, "Tomorrow, 03:00");
        assert_eq!(next[0].in_words, "in 17 h");
        assert!(next[0].first_run);
        assert_eq!(coming_up_from(&jobs, &[], now(), 0), []);
    }

    #[test]
    fn stats_count_failing_jobs_and_worked_share() {
        let jobs = [
            job("purge-expired-sessions", "0 3 * * *"),
            job("bot-score-bumper", "*/5 * * * *"),
        ];
        let mut runs = failed_purge();
        runs.push(attempt("bot-score-bumper", "b-1", at(24, 9, 30), 1, true));
        runs.push(attempt("bot-score-bumper", "b-2", at(24, 9, 25), 1, true));
        let stats = dashboard_stats_from_jobs_and_runs(&jobs, &runs, now());
        assert_eq!(stats.failing_jobs, 1);
        // Three runs in 24 h (the purge's retries count once); two worked.
        assert_eq!(stats.worked_24h_permille, Some(667));
        assert_eq!(stats.total_runs_today, 3);
    }

    #[test]
    fn stats_worked_share_none_without_finished_runs_sad() {
        let stats = dashboard_stats_from_jobs_and_runs(&[], &[], now());
        assert_eq!(stats.worked_24h_permille, None);
        assert_eq!(stats.failing_jobs, 0);
    }
}
