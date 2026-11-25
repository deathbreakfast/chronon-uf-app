use leptos::prelude::*;
use orbital::components::{Body1, SpacingSize};
use orbital::primitives::{
    Button, ButtonAppearance, Checkbox, Dialog, DialogActions, DialogBody, DialogContent,
    DialogSurface, DialogTitle, Flex, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarTitle,
};
use std::fmt::Write as _;

use crate::components::format_run_time;
use crate::server::{run_job_now, set_job_paused, JobStatus, UpdateJobOutcome};

use super::super::loaded::JobPageCtx;
use super::schedule_card::upcoming_runs;

/// Plain sentence for a save that lost to a newer revision.
pub fn format_conflict(outcome: &UpdateJobOutcome) -> String {
    match outcome {
        UpdateJobOutcome::Conflict { revision, by, at } => format!(
            "Someone saved this job while you were editing: {by} saved revision {revision} on {}. \
             Close this and open it again to see their change.",
            format_run_time(at)
        ),
        UpdateJobOutcome::Saved { .. } => String::new(),
    }
}

/// Pause or Resume confirmation. Saving makes a revision recorded against you.
#[component]
pub fn PauseDialog(
    /// Shared page state.
    ctx: JobPageCtx,
) -> impl IntoView {
    let job = ctx.job;
    let open = ctx.pause_dialog_open;
    let pausing = Signal::derive(move || job.with(|j| j.status == JobStatus::Active));
    let run_once = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let next_run = move || {
        job.with(|j| upcoming_runs(j, 1))
            .first()
            .map(|t| format_run_time(t))
    };

    let on_confirm = Callback::new(move |_| {
        if saving.get() {
            return;
        }
        let pause = pausing.get_untracked();
        let also_run = !pause && run_once.get_untracked();
        let id = job.with_untracked(|j| j.id.clone());
        saving.set(true);
        error.set(None);
        leptos::task::spawn_local_scoped(async move {
            match set_job_paused(id.clone(), pause).await {
                Ok(UpdateJobOutcome::Saved { .. }) => {
                    if also_run {
                        if let Err(e) = run_job_now(id, None).await {
                            error.set(Some(format!("Resumed, but Run now failed: {e}")));
                        }
                    }
                    ctx.refresh.run(());
                    if error.get_untracked().is_none() {
                        open.set(false);
                    }
                }
                Ok(conflict) => error.set(Some(format_conflict(&conflict))),
                Err(e) => error.set(Some(e.to_string())),
            }
            saving.set(false);
            run_once.set(false);
        });
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>
                        {move || if pausing.get() { "Pause this job?" } else { "Resume this job?" }}
                    </DialogTitle>
                    <DialogContent>
                        <div data-testid="pause-dialog">
                            <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                                <Show
                                    when=move || pausing.get()
                                    fallback=move || view! {
                                        <Body1>
                                            {move || next_run().map_or_else(
                                                || "It runs again on its schedule.".to_string(),
                                                |t| format!("The next run is {t}."),
                                            )}
                                        </Body1>
                                        <Body1>"Runs it skipped while paused aren't made up."</Body1>
                                        <div data-testid="resume-run-once">
                                            <Checkbox checked=run_once label="Also run it once now" />
                                        </div>
                                    }
                                >
                                    <Body1>
                                        {move || next_run().map_or_else(
                                            || "No runs until someone resumes it.".to_string(),
                                            |t| format!("No runs until someone resumes it. {t} would be the next."),
                                        )}
                                    </Body1>
                                    <Body1>
                                        "Runs it skips aren't made up later. A run that's already going finishes."
                                    </Body1>
                                </Show>
                                <Show when=move || error.get().is_some()>
                                    <MessageBar intent=MessageBarIntent::Error>
                                        {move || error.get().unwrap_or_default()}
                                    </MessageBar>
                                </Show>
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| open.set(false))>
                            "Cancel"
                        </Button>
                        <div data-testid="pause-dialog-confirm">
                            <Button appearance=ButtonAppearance::Primary disabled=saving on_click=on_confirm>
                                {move || if pausing.get() { "Pause" } else { "Resume" }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// Warning shown while the job is paused: who paused it, when, the runs it skipped.
#[component]
pub fn PausedBanner(
    /// Shared page state.
    ctx: JobPageCtx,
) -> impl IntoView {
    let job = ctx.job;
    let paused = Signal::derive(move || job.with(|j| j.status != JobStatus::Active));
    let detail = move || match ctx.pause_info.get() {
        Some(info) => {
            let mut text = format!("Paused by {} on {}.", info.by, format_run_time(&info.at));
            if info.skipped.is_empty() {
                text.push_str(" No runs skipped yet.");
            } else {
                let times: Vec<String> = info.skipped.iter().map(|t| format_run_time(t)).collect();
                let _ = write!(text, " Skipped: {}.", times.join(", "));
            }
            text
        }
        None => "Scheduled runs are stopped until someone resumes it.".to_string(),
    };

    view! {
        <Show when=move || paused.get()>
            <div data-testid="paused-banner">
                <MessageBar intent=MessageBarIntent::Warning>
                    <MessageBarTitle>"Paused"</MessageBarTitle>
                    <MessageBarBody>
                        <span data-testid="paused-banner-detail">{detail}</span>
                    </MessageBarBody>
                    <MessageBarActions>
                        <Show when=move || ctx.access.get().can_edit>
                            <div data-testid="paused-banner-resume">
                                <Button on_click=Callback::new(move |_| ctx.pause_dialog_open.set(true))>
                                    "Resume"
                                </Button>
                            </div>
                        </Show>
                    </MessageBarActions>
                </MessageBar>
            </div>
        </Show>
    }
}
