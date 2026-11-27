use std::collections::HashMap;

use orbital_data::{DataRecord, DataValue};

use crate::server::Job;

const fn job_status_key(status: crate::JobStatus) -> &'static str {
    use crate::JobStatus::{Active, Disabled, Paused};
    match status {
        Active => "active",
        Paused => "paused",
        Disabled => "disabled",
    }
}

pub fn job_to_record(job: Job) -> DataRecord {
    let id = job.id.clone();
    let text = |s: Option<String>| DataValue::Text(s.unwrap_or_default());
    DataRecord::new(
        id,
        HashMap::from([
            ("name".into(), DataValue::Text(job.name)),
            ("script_name".into(), DataValue::Text(job.script_name)),
            // Owner stays empty until per-job owners reach the list.
            ("owner".into(), DataValue::Text(String::new())),
            ("schedule".into(), DataValue::Text(job.schedule_text)),
            ("cron".into(), DataValue::Text(job.cron)),
            (
                "status".into(),
                DataValue::Text(job_status_key(job.status).into()),
            ),
            ("last_run".into(), text(job.last_run_at)),
            (
                "last_run_status".into(),
                text(
                    job.last_run_status
                        .map(|s| chronon_backend::run_status_label(s).to_string()),
                ),
            ),
            (
                "last_run_attempts".into(),
                text(job.last_run_attempts.map(|n| n.to_string())),
            ),
            ("next_run".into(), text(job.next_run_at)),
            ("timezone".into(), text(job.timezone)),
            ("your_access".into(), DataValue::Text(job.your_access)),
        ]),
    )
}
