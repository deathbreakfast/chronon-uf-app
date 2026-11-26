use std::collections::HashMap;

use orbital_data::{DataRecord, DataValue};

use crate::server::RunSummary;

const fn run_status_key(status: crate::RunStatus) -> &'static str {
    use crate::RunStatus::{Cancelled, Completed, Failed, Pending, Refused, Running};
    match status {
        Pending => "pending",
        Running => "running",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
        Refused => "refused",
    }
}

/// `4 of 4` for a retried run, `1` for a single try.
fn attempts_label(attempts: u32) -> String {
    if attempts > 1 {
        format!("{attempts} of {attempts}")
    } else {
        attempts.to_string()
    }
}

/// `1.2 s`, or `1.2 s each` for a retried run (the average per attempt).
fn duration_label(row: &RunSummary) -> String {
    row.duration_ms.map_or_else(
        || "\u{2014}".to_string(),
        |ms| {
            let words = chronon_backend::duration_words(ms);
            if row.attempts > 1 {
                format!("{words} each")
            } else {
                words
            }
        },
    )
}

/// Who started the run; a Refused run adds why, such as
/// `ravi (Run now) · no longer holds Run on billing-rollup`.
fn started_by_text(row: &RunSummary) -> String {
    let who = row.started_by.short_label();
    match row.refused_action.as_deref() {
        Some(action) => format!(
            "{who} · {}",
            chronon_backend::refused_note(action, &row.job_name)
        ),
        None => who,
    }
}

/// Maps a run summary onto a runs-table record keyed by its first attempt's id.
pub fn run_to_record(row: RunSummary) -> DataRecord {
    let id = row.root_run_id.clone();
    let mut fields: HashMap<String, DataValue> = HashMap::from([
        (
            "result".into(),
            DataValue::Text(run_status_key(row.result).into()),
        ),
        (
            "attempts".into(),
            DataValue::Text(attempts_label(row.attempts)),
        ),
        (
            "started_at".into(),
            DataValue::Text(crate::utils::format_started_at(&row.started_at)),
        ),
        ("duration".into(), DataValue::Text(duration_label(&row))),
        ("started_by".into(), DataValue::Text(started_by_text(&row))),
    ]);
    if !row.job_name.is_empty() {
        fields.insert("job_name".into(), DataValue::Text(row.job_name));
    }
    DataRecord::new(id, fields)
}
