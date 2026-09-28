use chrono::{TimeZone, Timelike, Utc};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};
use orbital_data::DataValue;
use orbital_paging::{FilterLogicWire, FilterQuery, FilterRuleParam, PageRequest};

use super::*;

fn sample_job(name: &str, script: &str, status: JobStatus) -> Job {
    Job {
        id: format!("id-{name}"),
        name: name.into(),
        script_name: script.into(),
        cron: "0 * * * *".into(),
        status,
        revision: 1,
        last_run_at: None,
        next_run_at: None,
        timezone: None,
        params: serde_json::json!({}),
    }
}

fn sample_run(id: &str, job_name: &str, status: RunStatus) -> Run {
    Run {
        id: id.into(),
        job_id: "j1".into(),
        job_name: job_name.into(),
        status,
        started_at: "2026-01-01T00:00:00Z".into(),
        finished_at: None,
        duration_ms: None,
        logs: None,
        stderr: None,
        error_message: None,
        parent_run_id: None,
    }
}

fn sample_core_run(
    id: &str,
    status: CoreRunStatus,
    scheduled_for: chrono::DateTime<Utc>,
) -> CoreRun {
    let mut run = CoreRun::for_job("job-1", "cleanup", scheduled_for);
    run.run_id = id.into();
    run.status = status;
    run
}

#[test]
fn validate_job_id_accepts_id_happy_path() {
    validate_job_id("job-1").expect("id");
    validate_job_id("  nightly.cleanup  ").expect("trimmed dotted name as id");
}

#[test]
fn validate_job_id_rejects_blank_sad() {
    assert_eq!(
        validate_job_id("").expect_err("blank"),
        ChrononIdError::EmptyJobId
    );
    assert!(ChrononIdError::EmptyJobId.to_string().contains("required"));
}

#[test]
fn validate_job_id_rejects_slash_control_dotdot_sad() {
    assert_eq!(
        validate_job_id("a/b").expect_err("slash"),
        ChrononIdError::UnsafeJobId
    );
    assert_eq!(
        validate_job_id("a\\b").expect_err("backslash"),
        ChrononIdError::UnsafeJobId
    );
    assert_eq!(
        validate_job_id("a\tb").expect_err("control"),
        ChrononIdError::UnsafeJobId
    );
    assert_eq!(
        validate_job_id("..").expect_err("dotdot"),
        ChrononIdError::UnsafeJobId
    );
    assert_eq!(
        validate_job_id(".").expect_err("dot"),
        ChrononIdError::UnsafeJobId
    );
}

#[test]
fn validate_job_id_rejects_oversized_sad() {
    let oversized = "a".repeat(MAX_CHRONON_ID_CHARS + 1);
    assert_eq!(
        validate_job_id(&oversized).expect_err("too long"),
        ChrononIdError::JobIdTooLong
    );
}

#[test]
fn validate_run_id_accepts_id_happy_path() {
    validate_run_id("run-1").expect("id");
}

#[test]
fn validate_run_id_rejects_blank_sad() {
    assert_eq!(
        validate_run_id("  ").expect_err("whitespace"),
        ChrononIdError::EmptyRunId
    );
}

#[test]
fn validate_run_id_rejects_control_sad() {
    assert_eq!(
        validate_run_id("run\nid").expect_err("control"),
        ChrononIdError::UnsafeRunId
    );
}

#[test]
fn validate_job_name_accepts_trimmed_happy_path() {
    validate_job_name("  nightly  ").expect("trimmed");
}

#[test]
fn validate_job_name_rejects_blank_sad() {
    assert_eq!(
        validate_job_name("   ").expect_err("whitespace"),
        ChrononIdError::EmptyJobName
    );
}

#[test]
fn validate_job_name_rejects_slash_sad() {
    assert_eq!(
        validate_job_name("a/b").expect_err("slash"),
        ChrononIdError::UnsafeJobName
    );
}

#[test]
fn validate_script_name_rejects_blank_sad() {
    assert_eq!(
        validate_script_name("").expect_err("blank"),
        ChrononIdError::EmptyScriptName
    );
}

#[test]
fn validate_script_name_rejects_oversized_sad() {
    let oversized = "a".repeat(MAX_CHRONON_ID_CHARS + 1);
    assert_eq!(
        validate_script_name(&oversized).expect_err("too long"),
        ChrononIdError::ScriptNameTooLong
    );
}

#[test]
fn validate_job_id_or_name_rejects_blank_sad() {
    assert_eq!(
        validate_job_id_or_name(" ").expect_err("whitespace"),
        ChrononIdError::EmptyJobIdOrName
    );
}

#[test]
fn validate_job_id_or_name_rejects_slash_sad() {
    assert_eq!(
        validate_job_id_or_name("job/1").expect_err("slash"),
        ChrononIdError::UnsafeJobIdOrName
    );
}

#[test]
fn encode_ops_path_segment_encodes_slash_and_space_happy_path() {
    assert_eq!(encode_ops_path_segment("orders"), "orders");
    assert_eq!(encode_ops_path_segment("a/b"), "a%2Fb");
    assert_eq!(encode_ops_path_segment("a b"), "a%20b");
    assert_eq!(encode_ops_path_segment("a\\b"), "a%5Cb");
}

#[test]
fn chronon_ops_paths_encode_segments_happy_path() {
    assert_eq!(chronon_job_path("a/b"), "/chronon/jobs/a%2Fb");
    assert_eq!(chronon_run_path("r/1"), "/chronon/runs/r%2F1");
}

#[test]
fn find_job_by_id_resolves_exact_happy_path() {
    let jobs = vec![
        sample_job("alpha", "cleanup", JobStatus::Active),
        sample_job("beta", "report", JobStatus::Paused),
    ];
    let found = find_job_by_id(&jobs, "id-beta").expect("listed");
    assert_eq!(found.name, "beta");
}

#[test]
fn find_job_by_id_unknown_is_none_sad() {
    let jobs = vec![sample_job("alpha", "cleanup", JobStatus::Active)];
    assert!(find_job_by_id(&jobs, "__chronon_missing_job__").is_none());
}

#[test]
fn find_run_by_id_resolves_exact_happy_path() {
    let runs = vec![
        sample_run("r1", "alpha", RunStatus::Completed),
        sample_run("r2", "beta", RunStatus::Failed),
    ];
    let found = find_run_by_id(&runs, "r2").expect("listed");
    assert_eq!(found.job_name, "beta");
}

#[test]
fn find_run_by_id_unknown_is_none_sad() {
    let runs = vec![sample_run("r1", "alpha", RunStatus::Completed)];
    assert!(find_run_by_id(&runs, "__chronon_missing_run__").is_none());
}

#[test]
fn resolve_job_id_from_list_prefers_id_then_name_happy_path() {
    let jobs = vec![sample_job("alpha", "cleanup", JobStatus::Active)];
    assert_eq!(
        resolve_job_id_from_list(&jobs, "id-alpha").as_deref(),
        Some("id-alpha")
    );
    assert_eq!(
        resolve_job_id_from_list(&jobs, "alpha").as_deref(),
        Some("id-alpha")
    );
}

#[test]
fn sort_jobs_by_name_orders_lexicographically_happy_path() {
    let mut jobs = vec![
        sample_job("zeta", "a", JobStatus::Active),
        sample_job("alpha", "b", JobStatus::Active),
    ];
    sort_jobs_by_name(&mut jobs);
    assert_eq!(jobs[0].name, "alpha");
    assert_eq!(jobs[1].name, "zeta");
}

#[test]
fn normalized_params_replaces_null_happy_path() {
    assert_eq!(
        normalized_params(serde_json::Value::Null),
        serde_json::json!({})
    );
}

#[test]
fn parse_script_params_parses_signature_happy_path() {
    let parsed = parse_script_params(r#"{"resource_id":"String","force":"bool"}"#);
    assert_eq!(parsed.len(), 2);
    assert!(parsed
        .iter()
        .any(|p| p.name == "resource_id" && p.param_type == "String"));
}

#[test]
fn parse_script_params_invalid_json_empty_sad() {
    assert_eq!(parse_script_params("not-json").len(), 0);
}

#[test]
fn parse_run_once_datetime_accepts_rfc3339_happy_path() {
    parse_run_once_datetime("2026-01-25T03:00:00Z").expect("rfc3339");
}

#[test]
fn parse_run_once_datetime_rejects_invalid_sad() {
    let err = parse_run_once_datetime("not-a-date").expect_err("bad");
    assert!(matches!(err, ChrononScheduleError::InvalidRunOnce));
    assert!(err.to_string().contains("Invalid run-once"), "{err}");
}

#[test]
fn build_create_job_model_manual_defaults_happy_path() {
    let payload = CreateJobRequest {
        job_name: "nightly-cleanup".into(),
        script_name: "cleanup_script".into(),
        schedule_type: CreateJobScheduleType::Manual,
        cron_expr: None,
        timezone: None,
        run_once_at: None,
        params: serde_json::json!({"retain_days": 7}),
        concurrency: 99,
        timeout_seconds: 120,
        max_retries: 55,
    };
    let job = build_create_job_model(&payload, "sig-hash".into()).expect("build");
    assert_eq!(job.job_name, "nightly-cleanup");
    assert_eq!(job.concurrency, 10);
    assert_eq!(job.timeout_ms, Some(120_000));
    assert!(job.next_run_at.is_none());
}

#[test]
fn build_create_job_model_rejects_empty_job_name_sad() {
    let payload = CreateJobRequest {
        job_name: "   ".into(),
        script_name: "cleanup_script".into(),
        schedule_type: CreateJobScheduleType::Manual,
        cron_expr: None,
        timezone: None,
        run_once_at: None,
        params: serde_json::json!({}),
        concurrency: 1,
        timeout_seconds: 60,
        max_retries: 0,
    };
    let err = build_create_job_model(&payload, "sig".into()).expect_err("blank name");
    assert!(matches!(err, ChrononScheduleError::Id(_)));
    assert!(err.to_string().contains("required"), "{err}");
}

#[test]
fn build_create_job_model_rejects_empty_cron_sad() {
    let payload = CreateJobRequest {
        job_name: "cron-job".into(),
        script_name: "cleanup_script".into(),
        schedule_type: CreateJobScheduleType::Cron,
        cron_expr: Some("  ".into()),
        timezone: None,
        run_once_at: None,
        params: serde_json::json!({}),
        concurrency: 1,
        timeout_seconds: 60,
        max_retries: 0,
    };
    let err = build_create_job_model(&payload, "sig".into()).expect_err("empty cron");
    assert!(matches!(err, ChrononScheduleError::MissingCron));
    assert!(err.to_string().contains("Cron expression"), "{err}");
}

#[test]
fn backend_job_to_job_maps_enabled_manual_happy_path() {
    let mut backend = CoreJob::new("nightly", "cleanup");
    backend.enabled = true;
    backend.cron_expr = None;
    backend.current_revision = 3;
    let job = backend_job_to_job(backend);
    assert_eq!(job.name, "nightly");
    assert_eq!(job.cron, "manual");
    assert_eq!(job.status, JobStatus::Active);
    assert_eq!(job.revision, 3);
}

#[test]
fn chronon_run_status_to_ui_maps_variants_happy_path() {
    assert_eq!(
        chronon_run_status_to_ui(CoreRunStatus::Queued),
        RunStatus::Pending
    );
    assert_eq!(
        chronon_run_status_to_ui(CoreRunStatus::Success),
        RunStatus::Completed
    );
    assert_eq!(
        chronon_run_status_to_ui(CoreRunStatus::Canceled),
        RunStatus::Cancelled
    );
}

#[test]
fn apply_jobs_page_query_filters_quick_search_happy_path() {
    let mut jobs = vec![
        sample_job("alpha", "cleanup", JobStatus::Active),
        sample_job("beta", "report", JobStatus::Paused),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("report".into()),
        filter: None,
        sort: None,
    };
    apply_jobs_page_query(&mut jobs, &request);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "beta");
}

#[test]
fn apply_jobs_page_query_status_unknown_empty_sad() {
    let mut jobs = vec![
        sample_job("alpha", "cleanup", JobStatus::Active),
        sample_job("beta", "report", JobStatus::Paused),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: None,
        filter: Some(FilterQuery {
            logic: FilterLogicWire::And,
            items: vec![FilterRuleParam {
                field: "status".into(),
                operator: "equals".into(),
                value: DataValue::Text("disabled".into()),
            }],
        }),
        sort: None,
    };
    apply_jobs_page_query(&mut jobs, &request);
    assert_eq!(jobs.len(), 0);
}

#[test]
fn apply_runs_page_query_not_equals_status_happy_path() {
    let mut runs = vec![
        sample_run("r1", "alpha", RunStatus::Completed),
        sample_run("r2", "beta", RunStatus::Failed),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: None,
        filter: Some(FilterQuery {
            logic: FilterLogicWire::And,
            items: vec![FilterRuleParam {
                field: "status".into(),
                operator: "not_equals".into(),
                value: DataValue::Text("completed".into()),
            }],
        }),
        sort: None,
    };
    apply_runs_page_query(&mut runs, &request);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, "r2");
}

#[test]
fn apply_scripts_page_query_filters_by_description_happy_path() {
    let mut scripts = vec![
        Script {
            name: "cleanup".into(),
            signature: "cleanup()".into(),
            params: vec![],
            description: Some("nightly".into()),
        },
        Script {
            name: "report".into(),
            signature: "report()".into(),
            params: vec![],
            description: None,
        },
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("nightly".into()),
        filter: None,
        sort: None,
    };
    apply_scripts_page_query(&mut scripts, &request);
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].name, "cleanup");
}

#[test]
fn runs_page_needs_memory_scan_blank_quick_search_sad() {
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("  ".into()),
        filter: None,
        sort: None,
    };
    assert!(!runs_page_needs_memory_scan(&request));
}

#[test]
fn utc_start_of_day_is_midnight_happy_path() {
    let now = Utc.with_ymd_and_hms(2026, 3, 15, 14, 30, 45).unwrap();
    let start = utc_start_of_day(now);
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap());
}

#[test]
fn dashboard_stats_aggregates_counts_happy_path() {
    let mut active = CoreJob::new("a", "s");
    active.enabled = true;
    let mut paused = CoreJob::new("b", "s");
    paused.enabled = false;
    let today = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();
    let runs = vec![
        sample_core_run(
            "r1",
            CoreRunStatus::Success,
            Utc.with_ymd_and_hms(2026, 1, 2, 1, 0, 0).unwrap(),
        ),
        sample_core_run(
            "r2",
            CoreRunStatus::Failed,
            Utc.with_ymd_and_hms(2026, 1, 2, 2, 0, 0).unwrap(),
        ),
        sample_core_run(
            "r3",
            CoreRunStatus::Running,
            Utc.with_ymd_and_hms(2026, 1, 2, 3, 0, 0).unwrap(),
        ),
        sample_core_run(
            "r-old",
            CoreRunStatus::Success,
            Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        ),
    ];
    let stats = dashboard_stats_from_jobs_and_runs(&[active, paused], &runs, today);
    assert_eq!(stats.total_jobs, 2);
    assert_eq!(stats.active_jobs, 1);
    assert_eq!(stats.paused_jobs, 1);
    assert_eq!(stats.total_runs_today, 3);
    assert_eq!(stats.successful_runs_today, 1);
    assert_eq!(stats.failed_runs_today, 1);
    assert_eq!(stats.running_now, 1);
}

#[test]
fn run_bucket_granularity_switches_at_one_day_happy_path() {
    assert_eq!(run_bucket_granularity(86_400), RunBucketGranularity::Hourly);
    assert_eq!(run_bucket_granularity(86_401), RunBucketGranularity::Daily);
}

#[test]
fn align_run_bucket_hourly_floors_happy_path() {
    let ts = Utc.with_ymd_and_hms(2026, 1, 1, 10, 45, 0).unwrap();
    let aligned = align_run_bucket(ts, RunBucketGranularity::Hourly);
    assert_eq!(aligned.hour(), 10);
    assert_eq!(aligned.minute(), 0);
}

#[test]
fn run_stats_series_includes_success_and_failed_happy_path() {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let runs = vec![
        sample_core_run(
            "r1",
            CoreRunStatus::Success,
            now - chrono::Duration::hours(1),
        ),
        sample_core_run(
            "r2",
            CoreRunStatus::Failed,
            now - chrono::Duration::hours(2),
        ),
    ];
    let series = run_stats_series_from_runs(&runs, now, 86_400);
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].id, "successful");
    assert!(series[0].points.iter().any(|p| p.value > 0.0));
    assert!(series[1].points.iter().any(|p| p.value > 0.0));
}

#[test]
fn run_stats_series_all_outside_window_zero_success_sad() {
    let now = Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap();
    let runs = vec![sample_core_run(
        "r-old",
        CoreRunStatus::Success,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    )];
    let series = run_stats_series_from_runs(&runs, now, 86_400);
    assert!(series[0].points.iter().all(|p| p.value == 0.0));
}

#[test]
fn apply_update_payload_to_job_normalizes_null_params_happy_path() {
    let existing = CoreJob::new("old-name", "cleanup_script");
    let payload = UpdateJobRequest {
        job_name: "new-name".into(),
        cron_expr: Some("0 * * * *".into()),
        timezone: None,
        params: serde_json::Value::Null,
        enabled: false,
    };
    let updated = apply_update_payload_to_job(existing, &payload).expect("update");
    assert_eq!(updated.job_name, "new-name");
    assert!(!updated.enabled);
    assert_eq!(updated.params_json, serde_json::json!({}));
}

#[test]
fn job_status_serde_roundtrip_happy_path() {
    let status = JobStatus::Active;
    let json = serde_json::to_string(&status).expect("serialize");
    assert_eq!(json, "\"active\"");
    let back: JobStatus = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, JobStatus::Active);
}

/// Same guard `run_job_now` uses: System-shaped `actor_json` must fail closed.
#[test]
fn validate_external_job_actor_json_rejects_system_for_run_now_sad() {
    let system = serde_json::json!({"System": {"operation": "platform_ttl"}});
    let err = chronon_coordinator::validate_external_job_actor_json(&system).unwrap_err();
    assert!(
        err.to_string().contains("System"),
        "expected System refuse message, got: {err}"
    );
}

#[test]
fn validate_external_job_actor_json_allows_user_actor_happy() {
    let user = serde_json::json!({"User": {"user_id": "user:alice"}});
    chronon_coordinator::validate_external_job_actor_json(&user).expect("user actor ok");
    let service = serde_json::json!({"Service": {"name": "chronon_api"}});
    chronon_coordinator::validate_external_job_actor_json(&service).expect("service actor ok");
}
