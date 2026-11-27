//! Spotlight steps for the Runs list (`/chronon/runs`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Runs page intro: one attempt per Run.
#[help_spotlight_step(
    route = "/chronon/runs",
    feature_highlight = "chronon-runs-intro",
    title = "Run history",
    spotlight = "chronon-runs-page",
    position = "top",
    order = 10
)]
#[component]
pub fn ChrononRunsIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-runs-intro",
        "A Run is one attempt to execute a Job's Script. Like one baking of the recipe: success, failure, or still in the oven.",
        None,
        &[],
    )
}

/// Search and filters on the runs list.
#[help_spotlight_step(
    route = "/chronon/runs",
    feature_highlight = "chronon-runs-search",
    title = "Find a Run",
    spotlight = "chronon-runs-search",
    position = "bottom",
    order = 20
)]
#[component]
pub fn ChrononRunsSearchHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-runs-search",
        "Search by Job or run id. Pick Failed and a time range when you are hunting failures.",
        None,
        &[],
    )
}

/// Runs data table columns.
#[help_spotlight_step(
    route = "/chronon/runs",
    feature_highlight = "chronon-runs-table",
    title = "Each Run",
    spotlight = "chronon-runs-data-table",
    position = "top",
    order = 30
)]
#[component]
pub fn ChrononRunsTableHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-runs-table",
        "One row per Run. Retries are folded into the Run they retry.",
        None,
        &[
            "Job: which schedule produced the Run",
            "Result: how the last attempt ended",
            "Attempts: 4 of 4 means three retries",
            "Started by: the schedule, or who used Run now",
        ],
    )
}

/// Open a Run via table row click.
#[help_spotlight_step(
    route = "/chronon/runs",
    feature_highlight = "chronon-runs-open",
    title = "Open a Run",
    spotlight = "chronon-runs-data-table",
    position = "bottom",
    order = 40
)]
#[component]
pub fn ChrononRunsOpenHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-runs-open",
        "Click a row for start/finish times and the text the Script printed.",
        None,
        &[],
    )
}

/// Run outcomes chart under the runs table.
#[help_spotlight_step(
    route = "/chronon/runs",
    feature_highlight = "chronon-runs-trend",
    title = "How runs are finishing",
    spotlight = "chronon-run-trend-card",
    position = "top",
    order = 50
)]
#[component]
pub fn ChrononRunsTrendHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-runs-trend",
        "Compare worked and failed runs over time, when you want the reliability story rather than one error.",
        Some("Switch between 24h and 7d to change the window."),
        &[],
    )
}
