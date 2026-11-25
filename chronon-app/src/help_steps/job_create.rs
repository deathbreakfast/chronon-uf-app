//! Spotlight steps for Create Job (`/chronon/jobs/new`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Create form intro: walk top to bottom.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-intro",
    title = "Building a schedule",
    spotlight = "chronon-job-create-page",
    position = "top",
    order = 10
)]
#[component]
pub fn ChrononCreateIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-intro",
        "Walk top to bottom: name the Job, choose the Script, set parameters, choose when it runs, then create. Advanced options are optional.",
        None,
        &[],
    )
}

/// Breadcrumb back to Jobs.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-back",
    title = "Back to Jobs",
    spotlight = "chronon-job-create-back",
    position = "bottom",
    order = 20
)]
#[component]
pub fn ChrononCreateBackHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-back",
        "Jobs in the breadcrumb returns to the list without creating anything.",
        None,
        &[],
    )
}

/// Name and Script fields.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-basic",
    title = "Name and Script",
    spotlight = "chronon-job-create-basic",
    position = "bottom",
    order = 30
)]
#[component]
pub fn ChrononCreateBasicHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-basic",
        "After you pick a Script, parameter fields appear.",
        None,
        &[
            "Name: label people will search for",
            "Script: registered recipe from Scripts",
        ],
    )
}

/// Parameters section.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-params",
    title = "Parameters",
    spotlight = "chronon-job-create-params",
    position = "top",
    order = 40
)]
#[component]
pub fn ChrononCreateParamsHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-params",
        "Parameters are the knobs this Script accepts. Fill them when the recipe needs inputs (paths, limits, flags). If none appear, the Script needs no extras.",
        None,
        &[],
    )
}

/// Schedule type and fields.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-schedule",
    title = "When it should run",
    spotlight = "chronon-job-create-schedule",
    position = "top",
    order = 50
)]
#[component]
pub fn ChrononCreateScheduleHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-schedule",
        "Pick when the Job runs. Each choice shows the fields it needs.",
        None,
        &[
            "On a schedule: repeats, shown in words with its first runs",
            "Once: one date and time, then done",
            "Only from Run now: runs when someone starts it",
        ],
    )
}

/// Advanced options section.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-advanced",
    title = "Advanced options",
    spotlight = "chronon-job-create-advanced",
    position = "top",
    order = 60
)]
#[component]
pub fn ChrononCreateAdvancedHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-advanced",
        "Expand this section when you need finer control. Defaults are fine for most schedules.",
        None,
        &[
            "Concurrency: how many of this Job at once",
            "Timeout: how long one attempt may run",
            "Max retries: how many times to try again",
        ],
    )
}

/// Cancel button.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-cancel",
    title = "Discard and leave",
    spotlight = "chronon-job-create-cancel",
    position = "top",
    order = 70
)]
#[component]
pub fn ChrononCreateCancelHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-cancel",
        "Cancel abandons the form and returns to Jobs. Nothing is saved.",
        None,
        &[],
    )
}

/// Create Job submit button.
#[help_spotlight_step(
    route = "/chronon/jobs/new",
    feature_highlight = "chronon-create-submit",
    title = "Save the Job",
    spotlight = "chronon-job-create-submit",
    position = "top",
    order = 80
)]
#[component]
pub fn ChrononCreateSubmitHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-create-submit",
        "Create writes the new schedule. You need a verified email and Chronon admin. On success you return to the Jobs list.",
        None,
        &[],
    )
}
