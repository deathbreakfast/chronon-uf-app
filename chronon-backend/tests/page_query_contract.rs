//! Integration contracts for jobs/runs/scripts `DataTable` query adapters.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chronon_backend::{
    apply_jobs_page_query, apply_runs_page_query, apply_scripts_page_query,
    runs_page_needs_memory_scan, Job, JobStatus, Run, RunStatus, Script,
};
use orbital_data::DataValue;
use orbital_paging::{FilterLogicWire, FilterQuery, FilterRuleParam, PageRequest};

fn sample_job(name: &str, script: &str, status: JobStatus) -> Job {
    Job {
        id: name.into(),
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
        duration_ms: None,
        logs: None,
        stderr: None,
        error_message: None,
        parent_run_id: None,
    }
}

#[test]
fn jobs_datatable_quick_search_happy_path() {
    let mut jobs = vec![
        sample_job("alpha", "cleanup", JobStatus::Active),
        sample_job("beta", "report", JobStatus::Paused),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("alpha".into()),
        filter: None,
        sort: None,
    };
    apply_jobs_page_query(&mut jobs, &request);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "alpha");
}

#[test]
fn jobs_datatable_or_logic_keeps_either_match_happy_path() {
    let mut jobs = vec![
        sample_job("alpha", "cleanup", JobStatus::Active),
        sample_job("beta", "report", JobStatus::Paused),
        sample_job("gamma", "other", JobStatus::Disabled),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: None,
        filter: Some(FilterQuery {
            logic: FilterLogicWire::Or,
            items: vec![
                FilterRuleParam {
                    field: "name".into(),
                    operator: "equals".into(),
                    value: DataValue::Text("alpha".into()),
                },
                FilterRuleParam {
                    field: "status".into(),
                    operator: "equals".into(),
                    value: DataValue::Text("paused".into()),
                },
            ],
        }),
        sort: None,
    };
    apply_jobs_page_query(&mut jobs, &request);
    assert_eq!(jobs.len(), 2);
}

#[test]
fn jobs_datatable_status_equals_happy_path() {
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
                value: DataValue::Text("paused".into()),
            }],
        }),
        sort: None,
    };
    apply_jobs_page_query(&mut jobs, &request);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "beta");
}

#[test]
fn jobs_datatable_status_unknown_empty_sad() {
    let mut jobs = vec![sample_job("alpha", "cleanup", JobStatus::Active)];
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
fn runs_datatable_status_equals_happy_path() {
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
                operator: "equals".into(),
                value: DataValue::Text("failed".into()),
            }],
        }),
        sort: None,
    };
    apply_runs_page_query(&mut runs, &request);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, "r2");
}

#[test]
fn runs_datatable_not_equals_status_happy_path() {
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
fn runs_datatable_quick_search_happy_path() {
    let mut runs = vec![
        sample_run("r1", "alpha", RunStatus::Completed),
        sample_run("r2", "beta", RunStatus::Failed),
    ];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("beta".into()),
        filter: None,
        sort: None,
    };
    apply_runs_page_query(&mut runs, &request);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].job_name, "beta");
}

#[test]
fn scripts_datatable_quick_search_happy_path() {
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
fn scripts_datatable_name_filter_unknown_empty_sad() {
    let mut scripts = vec![Script {
        name: "cleanup".into(),
        signature: "cleanup()".into(),
        params: vec![],
        description: None,
    }];
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: None,
        filter: Some(FilterQuery {
            logic: FilterLogicWire::And,
            items: vec![FilterRuleParam {
                field: "name".into(),
                operator: "equals".into(),
                value: DataValue::Text("__missing__".into()),
            }],
        }),
        sort: None,
    };
    apply_scripts_page_query(&mut scripts, &request);
    assert_eq!(scripts.len(), 0);
}

#[test]
fn runs_page_needs_memory_scan_with_filter_happy_path() {
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: None,
        filter: Some(FilterQuery {
            logic: FilterLogicWire::And,
            items: vec![FilterRuleParam {
                field: "status".into(),
                operator: "equals".into(),
                value: DataValue::Text("failed".into()),
            }],
        }),
        sort: None,
    };
    assert!(runs_page_needs_memory_scan(&request));
}

#[test]
fn runs_page_needs_memory_scan_blank_quick_search_sad() {
    let request = PageRequest {
        offset: 0,
        limit: 20,
        quick_search: Some("   ".into()),
        filter: None,
        sort: None,
    };
    assert!(!runs_page_needs_memory_scan(&request));
}
