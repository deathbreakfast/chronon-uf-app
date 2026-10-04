//! Integration contracts for job/run/script helpers backing
//! `get_jobs` / `get_job` / `get_run` / `get_scripts` / create+schedule.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use chronon_backend::{
    backend_job_to_job, build_create_job_model, find_job_by_id, find_job_by_name, find_run_by_id,
    find_script_by_name, model_run_to_app_run, resolve_job_id_from_list, sort_jobs_by_name,
    sort_scripts_by_name, validate_job_id, validate_job_id_or_name, validate_job_name,
    validate_run_id, validate_script_name, ChrononIdError, CreateJobRequest, CreateJobScheduleType,
    Job, JobStatus, Run, RunStatus, Script,
};
use chronon_core::{Job as CoreJob, Run as CoreRun, RunStatus as CoreRunStatus};

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
        pool: None,
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
        duration_ms: Some(42),
        logs: None,
        stderr: None,
        error_message: None,
        parent_run_id: None,
    }
}

#[test]
fn get_jobs_list_sorted_and_named_happy_path() {
    let mut jobs = vec![
        sample_job("zeta.job", "cleanup", JobStatus::Active),
        sample_job("alpha.job", "report", JobStatus::Paused),
    ];
    sort_jobs_by_name(&mut jobs);
    assert_eq!(jobs[0].name, "alpha.job");
    assert_eq!(jobs[1].name, "zeta.job");
    for j in &jobs {
        assert_ne!(j.name.trim(), "");
        assert_ne!(j.script_name.trim(), "");
    }
}

#[test]
fn get_job_detail_matches_list_entry_happy_path() {
    let jobs = vec![
        sample_job("orders", "cleanup", JobStatus::Active),
        sample_job("payments", "report", JobStatus::Paused),
    ];
    let detail = find_job_by_name(&jobs, "orders").expect("listed job must resolve");
    assert_eq!(detail.id, "id-orders");
    assert_eq!(detail.status, JobStatus::Active);
}

#[test]
fn get_job_unknown_name_is_none_sad() {
    let jobs = vec![sample_job("orders", "cleanup", JobStatus::Active)];
    assert!(find_job_by_name(&jobs, "__chronon_uf_app_no_such_job__").is_none());
}

#[test]
fn get_run_detail_matches_list_entry_happy_path() {
    let runs = vec![
        sample_run("r1", "orders", RunStatus::Completed),
        sample_run("r2", "payments", RunStatus::Failed),
    ];
    let detail = find_run_by_id(&runs, "r2").expect("listed run must resolve");
    assert_eq!(detail.id, "r2");
    assert_eq!(detail.job_name, "payments");
    assert_eq!(detail.status, RunStatus::Failed);
}

#[test]
fn get_run_unknown_id_is_none_sad() {
    let runs = vec![sample_run("r1", "orders", RunStatus::Completed)];
    assert!(find_run_by_id(&runs, "__chronon_uf_app_no_such_run__").is_none());
}

#[test]
fn resolve_job_id_or_name_list_entry_happy_path() {
    let jobs = vec![sample_job("nightly", "cleanup", JobStatus::Active)];
    assert_eq!(
        resolve_job_id_from_list(&jobs, "nightly").as_deref(),
        Some("id-nightly")
    );
    assert_eq!(
        find_job_by_id(&jobs, "id-nightly").map(|j| j.name.as_str()),
        Some("nightly")
    );
}

#[test]
fn resolve_job_unknown_id_or_name_is_none_sad() {
    let jobs = vec![sample_job("nightly", "cleanup", JobStatus::Active)];
    assert!(resolve_job_id_from_list(&jobs, "__missing__").is_none());
}

#[test]
fn get_scripts_list_sorted_and_named_happy_path() {
    let mut scripts = vec![
        Script {
            name: "zeta".into(),
            signature: "zeta()".into(),
            params: vec![],
            description: None,
        },
        Script {
            name: "alpha".into(),
            signature: "alpha()".into(),
            params: vec![],
            description: Some("first".into()),
        },
    ];
    sort_scripts_by_name(&mut scripts);
    assert_eq!(scripts[0].name, "alpha");
    assert_eq!(scripts[1].name, "zeta");
}

#[test]
fn get_script_detail_matches_list_entry_happy_path() {
    let scripts = vec![Script {
        name: "cleanup".into(),
        signature: "cleanup()".into(),
        params: vec![],
        description: None,
    }];
    let detail = find_script_by_name(&scripts, "cleanup").expect("listed");
    assert_eq!(detail.signature, "cleanup()");
}

#[test]
fn get_script_unknown_name_is_none_sad() {
    let scripts = vec![Script {
        name: "cleanup".into(),
        signature: "cleanup()".into(),
        params: vec![],
        description: None,
    }];
    assert!(find_script_by_name(&scripts, "__no_script__").is_none());
}

#[test]
fn create_job_manual_schedule_happy_path() {
    let payload = CreateJobRequest {
        job_name: "manual-job".into(),
        script_name: "cleanup".into(),
        schedule_type: CreateJobScheduleType::Manual,
        cron_expr: None,
        timezone: None,
        run_once_at: None,
        params: serde_json::json!({}),
        concurrency: 2,
        timeout_seconds: 30,
        max_retries: 1,
        pool: None,
    };
    let job = build_create_job_model(&payload, "sig".into()).expect("create");
    let ui = backend_job_to_job(job);
    assert_eq!(ui.name, "manual-job");
    assert_eq!(ui.cron, "manual");
    assert_eq!(ui.status, JobStatus::Active);
}

#[test]
fn create_job_run_once_schedule_happy_path() {
    let payload = CreateJobRequest {
        job_name: "once-job".into(),
        script_name: "cleanup".into(),
        schedule_type: CreateJobScheduleType::RunOnce,
        cron_expr: None,
        timezone: None,
        run_once_at: Some("2026-01-25T03:00:00Z".into()),
        params: serde_json::json!({}),
        concurrency: 1,
        timeout_seconds: 60,
        max_retries: 0,
        pool: None,
    };
    let job = build_create_job_model(&payload, "sig".into()).expect("create");
    assert!(job.run_once_at.is_some());
    assert_eq!(job.next_run_at, job.run_once_at);
}

#[test]
fn create_job_empty_run_once_sad() {
    let payload = CreateJobRequest {
        job_name: "once-job".into(),
        script_name: "cleanup".into(),
        schedule_type: CreateJobScheduleType::RunOnce,
        cron_expr: None,
        timezone: None,
        run_once_at: Some(String::new()),
        params: serde_json::json!({}),
        concurrency: 1,
        timeout_seconds: 60,
        max_retries: 0,
        pool: None,
    };
    let err = build_create_job_model(&payload, "sig".into()).expect_err("empty run-once");
    assert!(matches!(
        err,
        chronon_backend::ChrononScheduleError::MissingRunOnce
    ));
    assert!(err.to_string().contains("Run-once"), "{err}");
}

#[test]
fn model_run_to_app_run_preserves_identity_happy_path() {
    let mut core = CoreRun::for_job("job-1", "cleanup", Utc::now());
    core.run_id = "run-99".into();
    core.status = CoreRunStatus::Failed;
    core.error_json = Some(serde_json::json!({"message": "boom"}));
    core.duration_ms = Some(12);
    let ui = model_run_to_app_run(core, "nightly".into());
    assert_eq!(ui.id, "run-99");
    assert_eq!(ui.job_name, "nightly");
    assert_eq!(ui.status, RunStatus::Failed);
    assert_eq!(ui.error_message.as_deref(), Some("boom"));
    assert_eq!(ui.duration_ms, Some(12));
}

#[test]
fn backend_job_paused_when_disabled_happy_path() {
    let mut core = CoreJob::new("paused-job", "cleanup");
    core.enabled = false;
    core.cron_expr = Some("0 0 * * *".into());
    let ui = backend_job_to_job(core);
    assert_eq!(ui.status, JobStatus::Paused);
    assert_eq!(ui.cron, "0 0 * * *");
}

#[test]
fn validate_job_name_accepts_table_happy_path() {
    validate_job_name("nightly").expect("name");
    validate_job_id("job-1").expect("id");
    validate_run_id("run-1").expect("run");
    validate_script_name("cleanup").expect("script");
    validate_job_id_or_name("nightly").expect("key");
}

#[test]
fn validate_job_name_rejects_blank_sad() {
    assert_eq!(
        validate_job_name("").expect_err("blank"),
        ChrononIdError::EmptyJobName
    );
}

#[test]
fn validate_job_id_rejects_blank_sad() {
    assert_eq!(
        validate_job_id(" ").expect_err("blank"),
        ChrononIdError::EmptyJobId
    );
}

#[test]
fn validate_run_id_rejects_blank_sad() {
    assert_eq!(
        validate_run_id("").expect_err("blank"),
        ChrononIdError::EmptyRunId
    );
}

#[test]
fn validate_script_name_rejects_blank_sad() {
    assert_eq!(
        validate_script_name("  ").expect_err("blank"),
        ChrononIdError::EmptyScriptName
    );
}

#[test]
fn validate_job_name_rejects_slash_and_oversized_sad() {
    assert_eq!(
        validate_job_name("a/b").expect_err("slash"),
        ChrononIdError::UnsafeJobName
    );
    let oversized = "a".repeat(257);
    assert_eq!(
        validate_job_id(&oversized).expect_err("too long"),
        ChrononIdError::JobIdTooLong
    );
    assert_eq!(
        validate_run_id("r\\1").expect_err("backslash"),
        ChrononIdError::UnsafeRunId
    );
}
