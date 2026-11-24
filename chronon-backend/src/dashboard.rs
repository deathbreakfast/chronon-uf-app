//! Dashboard KPI and run-trend helpers for Chronon server functions.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveTime, Timelike, Utc};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};

use crate::runs::group_runs;
use crate::types::{DashboardChartPoint, DashboardChartSeries, DashboardStats};

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
/// A run and its retries count once, by the final attempt.
#[must_use]
pub fn dashboard_stats_from_jobs_and_runs(
    jobs: &[CoreJob],
    runs: &[CoreRun],
    today_start: DateTime<Utc>,
) -> DashboardStats {
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
            .filter(|r| matches!(r.status, CoreRunStatus::Failed | CoreRunStatus::Timeout))
            .count(),
    );
    let running_now = count_u32(
        runs_today
            .iter()
            .filter(|r| matches!(r.status, CoreRunStatus::Running))
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
    }
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
            CoreRunStatus::Failed | CoreRunStatus::Timeout | CoreRunStatus::Canceled => {
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
