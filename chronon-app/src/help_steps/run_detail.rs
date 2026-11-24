//! Spotlight steps for Run detail (`/chronon/runs/:run_id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Run title and status badge.
#[help_spotlight_step(
    route = "/chronon/runs/:run_id",
    feature_highlight = "chronon-run-header",
    title = "This attempt",
    spotlight = "chronon-run-detail-header",
    position = "bottom",
    order = 10
)]
#[component]
pub fn ChrononRunHeaderHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-run-header",
        "The title says when this attempt ran. The badge is how it ended. Run again starts the Job with these values.",
        None,
        &[],
    )
}

/// Breadcrumb back to the Job.
#[help_spotlight_step(
    route = "/chronon/runs/:run_id",
    feature_highlight = "chronon-run-job-link",
    title = "Open the Job",
    spotlight = "chronon-run-detail-breadcrumb",
    position = "bottom",
    order = 20
)]
#[component]
pub fn ChrononRunJobLinkHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-run-job-link",
        "Go back to the Job that produced this Run, or to every Job.",
        None,
        &[],
    )
}

/// Timing fields.
#[help_spotlight_step(
    route = "/chronon/runs/:run_id",
    feature_highlight = "chronon-run-timing",
    title = "Facts",
    spotlight = "chronon-run-detail-timing",
    position = "top",
    order = 30
)]
#[component]
pub fn ChrononRunTimingHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-run-timing",
        "What ran, who started it, and with which values.",
        None,
        &[
            "Started by: the schedule, a retry, or the person who used Run now",
            "Parameters: the values used, with the stored value where it differed",
            "Run id: quote it when you report a problem",
        ],
    )
}

/// Output region: error, stdout, stderr.
#[help_spotlight_step(
    route = "/chronon/runs/:run_id",
    feature_highlight = "chronon-run-output",
    title = "Output",
    spotlight = "chronon-run-detail-output",
    position = "top",
    order = 40
)]
#[component]
pub fn ChrononRunOutputHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-run-output",
        "When something fails, start here. Empty means nothing was captured for this attempt.",
        None,
        &[
            "Error: why the Script stopped, when it did",
            "Standard output: text the Script printed",
            "Standard error: text printed as errors",
        ],
    )
}
