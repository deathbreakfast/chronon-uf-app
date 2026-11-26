use std::fmt::Write as _;
use std::sync::Arc;

use leptos::prelude::*;
use orbital::components::{Body1Strong, Caption1, SpacingSize};
use orbital::primitives::{Code, ColumnType, DataTableColumnDef, Flex, FlexAlign};
use orbital_data::DataRecord;
use turf::inline_style_sheet_values;

use crate::components::{
    format_run_time, relative_to_now, ChrononHelpColumnHeader, JobStatusBadge, RunStatusBadge,
};
use crate::server::RunStatus;
use crate::JobStatus;

fn field(record: &DataRecord, key: &str) -> String {
    record
        .get(key)
        .map(orbital_data::DataValue::display_string)
        .unwrap_or_default()
}

fn run_status_from_label(label: &str) -> Option<RunStatus> {
    match label {
        "pending" => Some(RunStatus::Pending),
        "running" => Some(RunStatus::Running),
        "completed" => Some(RunStatus::Completed),
        "failed" => Some(RunStatus::Failed),
        "cancelled" => Some(RunStatus::Cancelled),
        "refused" => Some(RunStatus::Refused),
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
pub fn jobs_table_columns() -> Vec<DataTableColumnDef> {
    let status_view = Arc::new(|record: DataRecord| {
        let status = match field(&record, "status").as_str() {
            "paused" => JobStatus::Paused,
            "disabled" => JobStatus::Disabled,
            _ => JobStatus::Active,
        };
        view! { <JobStatusBadge status=status /> }.into_any()
    });

    let name_view = Arc::new(|record: DataRecord| {
        let name = field(&record, "name");
        let script = field(&record, "script_name");
        view! {
            <Flex vertical=true>
                <Body1Strong>{name}</Body1Strong>
                <Caption1>{script}</Caption1>
            </Flex>
        }
        .into_any()
    });

    let schedule_view = Arc::new(|record: DataRecord| {
        let (style, classes) = inline_style_sheet_values! {
            .CronChip {
                padding: 0 var(--orb-space-inline-md);
                line-height: var(--orb-type-line-md);
            }
        };
        let sentence = field(&record, "schedule");
        let sentence = if sentence.is_empty() {
            "Hidden".to_string()
        } else {
            sentence
        };
        let cron = field(&record, "cron");
        view! {
            <style>{style}</style>
            <Flex vertical=true gap=SpacingSize::Size20.flex_gap() attr:data-testid="jobs-schedule-cell">
                <span>{sentence}</span>
                {(!cron.is_empty()).then(|| view! {
                    <Caption1><Code class=classes.cron_chip text=cron /></Caption1>
                })}
            </Flex>
        }
        .into_any()
    });

    let last_run_view = Arc::new(|record: DataRecord| {
        let at = field(&record, "last_run");
        let status = run_status_from_label(&field(&record, "last_run_status"));
        let attempts = field(&record, "last_run_attempts")
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 1);
        let view = match (status, at.is_empty()) {
            (Some(status), _) => {
                let mut text = if at.is_empty() {
                    String::new()
                } else {
                    format_run_time(&at)
                };
                if let Some(n) = attempts {
                    let _ = write!(text, " · {n} attempts");
                }
                view! {
                    <Flex align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                        <RunStatusBadge status=status />
                        <Caption1>{text}</Caption1>
                    </Flex>
                }
                .into_any()
            }
            (None, false) => view! { <span>{format_run_time(&at)}</span> }.into_any(),
            // Last run status arrives with the runs work; until then show a dash.
            (None, true) => view! { <span>"\u{2014}"</span> }.into_any(),
        };
        view! { <div data-testid="jobs-last-run-cell">{view}</div> }.into_any()
    });

    let next_run_view = Arc::new(|record: DataRecord| {
        let cron = field(&record, "cron");
        let mut next = field(&record, "next_run");
        if next.is_empty() && !cron.is_empty() {
            // Nothing stored yet: work it out from the cron, as the scheduler will.
            let tz = field(&record, "timezone");
            let tz = if tz.is_empty() { "UTC".to_string() } else { tz };
            next = chronon_backend::next_runs(&cron, &tz, chrono::Utc::now(), 1)
                .first()
                .map(chrono::DateTime::to_rfc3339)
                .unwrap_or_default();
        }
        let paused = field(&record, "status") != "active";
        let view = if paused {
            view! { <span>"Paused"</span> }.into_any()
        } else if next.is_empty() {
            view! { <span>"Manual"</span> }.into_any()
        } else {
            view! {
                <Flex vertical=true>
                    <span>{format_run_time(&next)}</span>
                    <Caption1>{relative_to_now(&next)}</Caption1>
                </Flex>
            }
            .into_any()
        };
        view! { <div data-testid="jobs-next-run-cell">{view}</div> }.into_any()
    });

    // The reader's role on each job; "No access" rows stay listed so people can ask.
    let access_view = Arc::new(|record: DataRecord| {
        let access = field(&record, "your_access");
        view! { <span data-testid="jobs-your-access">{access}</span> }.into_any()
    });

    let schedule_header = Arc::new(|| {
        view! {
            <ChrononHelpColumnHeader
                label="Schedule"
                info=view! {
                    <Caption1>
                        "The schedule in words, in the job's timezone, with its cron underneath. Run-once and manual jobs show their own labels."
                    </Caption1>
                }.into_any()
            />
        }
        .into_any()
    });

    vec![
        DataTableColumnDef::new("name", "Job")
            .with_sortable(false)
            .with_cell_view(name_view),
        DataTableColumnDef::new("owner", "Owner").with_sortable(false),
        DataTableColumnDef::new("your_access", "Your access")
            .with_sortable(false)
            .with_cell_view(access_view),
        DataTableColumnDef::new("schedule", "Schedule")
            .with_sortable(false)
            .with_header_view(schedule_header)
            .with_cell_view(schedule_view),
        DataTableColumnDef::new("status", "Status")
            .with_col_type(ColumnType::SingleSelect)
            .with_sortable(false)
            .with_cell_view(status_view),
        DataTableColumnDef::new("last_run", "Last run")
            .with_sortable(false)
            .with_cell_view(last_run_view),
        DataTableColumnDef::new("next_run", "Next run")
            .with_sortable(false)
            .with_cell_view(next_run_view),
    ]
}
