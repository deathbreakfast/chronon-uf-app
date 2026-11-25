//! Spotlight steps for Job detail (`/chronon/jobs/:job_id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Job title and status badge.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-header",
    title = "This Job",
    spotlight = "chronon-job-detail-header",
    position = "bottom",
    order = 10
)]
#[component]
pub fn ChrononJobHeaderHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-header",
        "The title is the Job name. The status badge shows whether the schedule is active or paused.",
        None,
        &[],
    )
}

/// Schedule card.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-schedule",
    title = "Schedule",
    spotlight = "chronon-job-detail-schedule",
    position = "bottom",
    order = 20
)]
#[component]
pub fn ChrononJobScheduleHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-schedule",
        "The schedule in words, in the Job's timezone, with its next runs. Edit schedule shows the runs before and after a change.",
        None,
        &[],
    )
}

/// Pause or Resume.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-pause",
    title = "Pause or resume",
    spotlight = "chronon-job-detail-pause",
    position = "bottom",
    order = 30
)]
#[component]
pub fn ChrononJobPauseHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-pause",
        "Pause stops scheduled runs until someone resumes the Job. Runs skipped while paused aren't made up. Each change saves a revision.",
        None,
        &[],
    )
}

/// Run Now button.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-run-now",
    title = "Run Now",
    spotlight = "chronon-job-detail-run-now",
    position = "bottom",
    order = 40
)]
#[component]
pub fn ChrononJobRunNowHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-run-now",
        "Starts one attempt now without waiting for the schedule. A dialog may let you override parameters for this attempt only.",
        None,
        &[],
    )
}

/// Edit button.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-edit",
    title = "Edit the Job",
    spotlight = "chronon-job-detail-edit",
    position = "bottom",
    order = 50
)]
#[component]
pub fn ChrononJobEditHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-edit",
        "Edit opens a form for the name, worker pool and parameters. Save and Cancel appear after Edit.",
        None,
        &[],
    )
}

/// Save button (edit mode).
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-save",
    title = "Save changes",
    spotlight = "chronon-job-detail-save",
    position = "top",
    order = 60
)]
#[component]
pub fn ChrononJobSaveHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-save",
        "Save writes a new revision of this Job. Visible while editing. Needs Chronon admin.",
        None,
        &[],
    )
}

/// Cancel edit button.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-cancel-edit",
    title = "Cancel edit",
    spotlight = "chronon-job-detail-cancel",
    position = "top",
    order = 70
)]
#[component]
pub fn ChrononJobCancelEditHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-cancel-edit",
        "Cancel leaves edit mode and discards unsaved form changes. The last saved revision stays as it was.",
        None,
        &[],
    )
}

/// Configuration snapshot card.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-config",
    title = "Configuration",
    spotlight = "chronon-job-detail-config",
    position = "top",
    order = 80
)]
#[component]
pub fn ChrononJobConfigHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-config",
        "How this Job runs.",
        None,
        &[
            "Script: recipe this Job runs",
            "Worker pool, runs at once, timeout and retries",
            "Parameters: values passed into the Script",
        ],
    )
}

/// Recent runs for this Job.
#[help_spotlight_step(
    route = "/chronon/jobs/:job_id",
    feature_highlight = "chronon-job-recent",
    title = "Runs for this Job",
    spotlight = "chronon-job-detail-recent-runs",
    position = "top",
    order = 90
)]
#[component]
pub fn ChrononJobRecentHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-job-recent",
        "History for this Job only. Click a row to open that attempt's timing and output.",
        None,
        &[],
    )
}
