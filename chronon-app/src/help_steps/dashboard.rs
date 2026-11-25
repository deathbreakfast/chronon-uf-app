//! Spotlight steps for the Chronon dashboard (`/chronon`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Centered intro: control room metaphor and Job / Script / Run vocabulary.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-intro",
    title = "Welcome to Chronon",
    order = 10
)]
#[component]
pub fn ChrononIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-intro",
        "Chronon is the control room for work that should run on a schedule. Think of it like a shared calendar for automated tasks: pick what to run, choose when, and keep a history of every attempt.",
        Some("Anyone signed in can browse these pages. Changing a schedule or pressing Run Now needs Chronon admin. We will walk the screens one piece at a time."),
        &[
            "Job: the calendar entry (what + when)",
            "Script: the recipe the job runs",
            "Run: one time that recipe actually ran",
        ],
    )
}

/// Needs attention: failed, paused and missed jobs.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-dashboard-attention",
    title = "What needs you",
    spotlight = "chronon-dashboard-attention",
    position = "bottom",
    order = 20
)]
#[component]
pub fn ChrononDashboardAttentionHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-dashboard-attention",
        "Start here. A job shows up when its last run failed, when it's paused, or when it missed a run it should have made.",
        Some("A run that failed and then worked on a retry doesn't count as failed."),
        &[],
    )
}

/// Stat cards: failing jobs, runs today, worked share, running now, paused.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-dashboard-stats",
    title = "At a glance",
    spotlight = "chronon-dashboard-stats",
    position = "bottom",
    order = 30
)]
#[component]
pub fn ChrononDashboardStatsHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-dashboard-stats",
        "A run and its retries count once, by how the last attempt ended.",
        Some("Worked (24h) is the share of runs that finished cleanly in the last 24 hours."),
        &[],
    )
}

/// Coming up: the next runs across every job.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-dashboard-coming-up",
    title = "What runs next",
    spotlight = "chronon-dashboard-coming-up",
    position = "top",
    order = 40
)]
#[component]
pub fn ChrononDashboardComingUpHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-dashboard-coming-up",
        "The next five runs across every active job. Paused jobs don't appear until someone resumes them.",
        Some("First run marks a job that has never run, so check its first result."),
        &[],
    )
}

/// Recent runs on the dashboard.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-dashboard-recent",
    title = "Latest runs",
    spotlight = "chronon-dashboard-recent-runs",
    position = "top",
    order = 50
)]
#[component]
pub fn ChrononDashboardRecentHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-dashboard-recent",
        "Click a row to read that run's attempts and output. Retries are folded into the run they retry.",
        Some("The run outcomes chart is on the Runs page."),
        &[],
    )
}

/// Left navigation destinations.
#[help_spotlight_step(
    route = "/chronon",
    feature_highlight = "chronon-nav",
    title = "Finding your way",
    spotlight = "chronon-nav",
    position = "right",
    order = 60
)]
#[component]
pub fn ChrononNavHelp() -> impl IntoView {
    help_stack(
        "help-step-chronon-nav",
        "Use the left menu to open Dashboard for a health overview, Jobs for schedules and settings, Runs for history of attempts, and Scripts for recipes jobs can call.",
        Some("Help → Replay restarts this page's tour."),
        &[],
    )
}
