use chrono::{DateTime, Duration, TimeZone, Utc};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus, ScheduleKind};
use serde_json::json;

use super::*;
use crate::types::{DayResult, Job, JobStatus, RunStatus, StartedBy};

fn no_names() -> HashMap<String, String> {
    HashMap::new()
}

fn at(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 24, h, m, 0).unwrap()
}

fn purge_job() -> CoreJob {
    let mut job = CoreJob::new("session-purge", "purge_sessions");
    job.job_id = "job-purge".into();
    job.schedule_kind = ScheduleKind::Cron;
    job.cron_expr = Some("0 3 * * *".into());
    job.actor_json = json!({"User": {"user_id": "ines"}});
    job.retry_policy_json = json!({
        "max_attempts": 3, "base_delay_ms": 30000_u64,
        "backoff_multiplier": 1.0_f64, "max_delay_ms": 60000_u64
    });
    job
}

fn attempt(id: &str, when: DateTime<Utc>, n: i32, status: CoreRunStatus) -> CoreRun {
    let mut run = CoreRun::new("purge_sessions", when);
    run.run_id = id.into();
    run.job_id = Some("job-purge".into());
    run.attempt = n;
    run.status = status;
    run.started_at = Some(when);
    run.duration_ms = Some(1200);
    run.actor_json = json!({"User": {"user_id": "ines"}});
    run.params_json = json!({"older_than_days": 30});
    run
}

/// A failed 03:00 run and its three failed retries, 30 s apart.
fn failed_purge() -> Vec<CoreRun> {
    (1..=4)
        .map(|n| {
            attempt(
                &format!("purge-{n}"),
                at(3, 0) + Duration::seconds(30 * i64::from(n - 1)),
                n,
                CoreRunStatus::Failed,
            )
        })
        .collect()
}

#[test]
fn started_by_schedule_for_system_actor() {
    let mut run = attempt(
        "r1",
        at(9, 17) + Duration::milliseconds(5),
        1,
        CoreRunStatus::Success,
    );
    run.actor_json = json!({"System": {"operation": "ttl"}});
    assert_eq!(
        started_by(&run, "r1", None, &no_names()),
        StartedBy::Schedule
    );
}

#[test]
fn started_by_schedule_for_person_owned_job_on_cron_tick() {
    let job = purge_job();
    let run = attempt("r1", at(3, 0), 1, CoreRunStatus::Success);
    assert_eq!(
        started_by(&run, "r1", Some(&job), &no_names()),
        StartedBy::Schedule
    );
}

#[test]
fn started_by_retry_for_later_attempt() {
    let job = purge_job();
    let run = attempt(
        "r2",
        at(3, 0) + Duration::seconds(30),
        2,
        CoreRunStatus::Failed,
    );
    assert_eq!(
        started_by(&run, "r1", Some(&job), &no_names()),
        StartedBy::Retry { of: "r1".into() }
    );
    let mut linked = attempt("r3", at(4, 0), 1, CoreRunStatus::Failed);
    linked.parent_run_id = Some("r1".into());
    assert!(matches!(
        started_by(&linked, "r1", Some(&job), &no_names()),
        StartedBy::Retry { .. }
    ));
}

#[test]
fn started_by_person_for_run_now() {
    let job = purge_job();
    let mut run = attempt(
        "r1",
        at(10, 15) + Duration::milliseconds(321),
        1,
        CoreRunStatus::Success,
    );
    run.actor_json = json!({"User": {"user_id": "kenji"}});
    assert_eq!(
        started_by(&run, "r1", Some(&job), &no_names()),
        StartedBy::Person {
            label: "kenji (Run now)".into()
        }
    );
}

#[test]
fn started_by_person_uses_display_name() {
    let job = purge_job();
    let mut run = attempt(
        "r1",
        at(10, 15) + Duration::milliseconds(321),
        1,
        CoreRunStatus::Success,
    );
    run.actor_json = json!({"User": {"user_id": "kenji"}});
    let names = HashMap::from([("kenji".to_string(), "Kenji Sato".to_string())]);
    assert_eq!(
        started_by(&run, "r1", Some(&job), &names),
        StartedBy::Person {
            label: "Kenji Sato (Run now)".into()
        }
    );
}

#[test]
fn started_by_service_user_keeps_its_name() {
    let mut run = attempt("r1", at(10, 15), 1, CoreRunStatus::Success);
    run.actor_json = json!({"ServiceUser": {"service_name": "agent"}});
    let names = HashMap::from([("agent".to_string(), "Not this".to_string())]);
    assert_eq!(
        started_by(&run, "r1", None, &names).short_label(),
        "agent (Run now)"
    );
}

#[test]
fn run_now_user_ids_dedupes_and_skips_non_users() {
    let actors = [
        json!({"User": {"user_id": "kenji"}}),
        json!({"User": {"user_id": "kenji"}}),
        json!({"System": {"operation": "ttl"}}),
        json!({"ServiceUser": {"service_name": "agent"}}),
        json!({"User": {"user_id": "ines"}}),
    ];
    let runs: Vec<CoreRun> = actors
        .into_iter()
        .zip(1..)
        .map(|(actor, hour)| {
            let mut run = attempt(&format!("r{hour}"), at(hour, 0), 1, CoreRunStatus::Success);
            run.actor_json = actor;
            run
        })
        .collect();
    assert_eq!(
        run_now_user_ids(&runs),
        vec!["ines".to_string(), "kenji".to_string()]
    );
}

#[test]
fn fold_runs_folds_three_retries_into_one_failed_row() {
    let job = purge_job();
    let rows = fold_runs(&failed_purge(), &[job], &no_names());
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.root_run_id, "purge-1");
    assert_eq!(row.result, RunStatus::Failed);
    assert_eq!(row.attempts, 4);
    assert_eq!(row.job_name, "session-purge");
    assert_eq!(row.started_by, StartedBy::Schedule);
    assert_eq!(row.duration_ms, Some(1200));
}

#[test]
fn fold_runs_uses_root_run_id_links() {
    let mut runs = failed_purge();
    for r in runs.iter_mut().skip(1) {
        r.attempt = 1;
        r.root_run_id = Some("purge-1".into());
    }
    let rows = fold_runs(&runs, &[], &no_names());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].attempts, 4);
}

#[test]
fn retry_that_worked_makes_the_run_worked() {
    let mut runs = failed_purge();
    runs.truncate(2);
    runs[1].status = CoreRunStatus::Success;
    let rows = fold_runs(&runs, &[purge_job()], &no_names());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].result, RunStatus::Completed);
}

#[test]
fn run_attempts_labels_first_try_and_retries() {
    let runs = failed_purge();
    let group = find_run_group(&runs, "purge-3").unwrap();
    let labels: Vec<String> = run_attempts(&group).into_iter().map(|a| a.label).collect();
    assert_eq!(labels, ["First try", "Retry 1", "Retry 2", "Retry 3"]);
}

#[test]
fn dashboard_counts_a_run_and_its_retries_once() {
    let stats =
        crate::dashboard_stats_from_jobs_and_runs(&[purge_job()], &failed_purge(), at(0, 0));
    assert_eq!(stats.total_runs_today, 1);
    assert_eq!(stats.failed_runs_today, 1);
    let series = crate::run_stats_series_from_runs(&failed_purge(), at(12, 0), 86_400);
    let failed: f64 = series[1].points.iter().map(|p| p.value).sum();
    assert!((failed - 1.0).abs() < f64::EPSILON);
}

#[test]
fn fill_last_run_sets_time_and_status() {
    let mut job = Job {
        id: "job-purge".into(),
        name: "session-purge".into(),
        script_name: "purge_sessions".into(),
        cron: "0 3 * * *".into(),
        status: JobStatus::Active,
        revision: 1,
        last_run_at: None,
        last_run_status: None,
        next_run_at: None,
        timezone: None,
        params: json!({}),
        pool: None,
        schedule_text: String::new(),
        last_run_attempts: None,
        owner_label: String::new(),
        system_owned: false,
        concurrency: 1,
        timeout_ms: None,
        retry_max_attempts: 0,
        retry_base_delay_ms: 0,
    };
    crate::fill_last_run(&mut job, &failed_purge());
    assert_eq!(
        job.last_run_at.as_deref(),
        Some(at(3, 0).to_rfc3339().as_str())
    );
    assert_eq!(job.last_run_status, Some(RunStatus::Failed));
}

#[test]
fn job_week_counts_failed_then_worked_day_as_worked() {
    let mut runs = Vec::new();
    for back in 1..=3 {
        runs.push(attempt(
            &format!("ok-{back}"),
            at(3, 0) - Duration::days(back),
            1,
            CoreRunStatus::Success,
        ));
    }
    // Yesterday-but-two failed first, then worked on retry.
    let day = at(3, 0) - Duration::days(4);
    runs.push(attempt("d4-1", day, 1, CoreRunStatus::Failed));
    runs.push(attempt(
        "d4-2",
        day + Duration::seconds(30),
        2,
        CoreRunStatus::Success,
    ));
    runs.extend(failed_purge());

    let week = job_week(&runs, at(12, 0));
    let results: Vec<DayResult> = week.days.iter().map(|d| d.result).collect();
    assert_eq!(
        results,
        [
            DayResult::None,
            DayResult::None,
            DayResult::Worked,
            DayResult::Worked,
            DayResult::Worked,
            DayResult::Worked,
            DayResult::Failed,
        ]
    );
    assert_eq!(
        week.summary,
        "Last run: failed today at 03:00 after 4 attempts. Before that it worked 4 days in a row."
    );
    let failing = week.failing.unwrap();
    assert_eq!(failing.run_id, "purge-1");
    assert_eq!(failing.title, "The 03:00 run failed");
}

#[test]
fn filters_and_caption() {
    let mut rows = fold_runs(&failed_purge(), &[purge_job()], &no_names());
    let mut request = PageRequest::new(0, 20);
    request.filter = Some(orbital_paging::FilterQuery {
        items: vec![orbital_paging::FilterRuleParam {
            field: "result".into(),
            operator: "is".into(),
            value: orbital_data::DataValue::Text("worked".into()),
        }],
        logic: orbital_paging::FilterLogicWire::And,
    });
    let mut worked = rows.clone();
    apply_run_summary_query(&mut worked, &request, at(12, 0));
    assert_eq!(worked.len(), 0);
    apply_run_summary_query(&mut rows, &PageRequest::new(0, 20), at(12, 0));
    assert_eq!(
        runs_caption(&rows, RunTimeFilter::Last24Hours),
        "1 run in the last 24 hours. The 03:00 session-purge run was retried 3 times."
    );
}

#[test]
fn words() {
    assert_eq!(describe_cron("0 3 * * *", None), "every day at 03:00 UTC");
    assert_eq!(
        describe_cron("0 0 3 * * *", Some("UTC")),
        "every day at 03:00 UTC"
    );
    assert_eq!(describe_cron("0 * * * *", None), "every hour");
    assert_eq!(describe_cron("*/5 * * * *", None), "every 5 minutes");
    assert_eq!(
        describe_cron("0 3 * * 1", None),
        "on the schedule 0 3 * * 1"
    );
    assert_eq!(run_title(at(3, 0), at(12, 0)), "Run at 03:00 today");
    assert_eq!(
        when_words(at(3, 0) + Duration::days(1), at(12, 0)),
        "tomorrow at 03:00"
    );
    assert_eq!(duration_words(1200), "1.2 s");
    assert_eq!(duration_words(15), "15 ms");
    assert_eq!(
        retry_caption(Some(&purge_job())),
        "Retries run 30 s apart. The job allows 3."
    );
    assert_eq!(if_it_fails_words(&purge_job()), "Retries up to 3 times");
}

#[test]
fn coerce_script_param_refuses_words_for_integers() {
    assert_eq!(
        coerce_script_param("i64", "fourteen", true),
        Err("Enter a whole number, such as 30.".to_string())
    );
    assert_eq!(
        coerce_script_param("i64", " 14 ", true),
        Ok(Some(json!(14)))
    );
    assert_eq!(coerce_script_param("String", "", false), Ok(None));
    assert!(coerce_script_param("String", "", true).is_err());
    assert_eq!(param_field_text(Some(&json!(30))), "30");
}
