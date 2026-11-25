use leptos::prelude::*;
use orbital::components::{Caption1, SpacingSize};
use orbital::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Flex, MessageBar, MessageBarIntent,
};

use crate::components::{format_run_time, NextRunsTable, ScheduleFields};
use crate::server::{update_job, JobStatus, SchedulePreview, UpdateJobOutcome, UpdateJobRequest};

use super::super::display::display_timezone;
use super::super::loaded::JobPageCtx;
use super::pause_dialog::format_conflict;
use super::schedule_card::upcoming_runs;

/// What saving does, for the caption under the preview.
fn save_summary(next_revision: u32, before: &[String], after: &[String]) -> String {
    let change = match (before.first(), after.first()) {
        (Some(b), Some(a)) if b != a => format!(
            " The next run moves from {} to {}.",
            format_run_time(b),
            format_run_time(a)
        ),
        (_, Some(a)) => format!(" The next run stays at {}.", format_run_time(a)),
        _ => String::new(),
    };
    format!(
        "Saving makes revision {next_revision}.{change} A run that's already going isn't stopped."
    )
}

/// Edit schedule: the shared fields with the next five runs before and after the change.
#[allow(clippy::too_many_lines)]
#[component]
pub fn ScheduleDialog(
    /// Shared page state.
    ctx: JobPageCtx,
) -> impl IntoView {
    let job = ctx.job;
    let open = ctx.schedule_dialog_open;
    let cron = RwSignal::new(String::new());
    let timezone = RwSignal::new(String::new());
    let preview = RwSignal::new(SchedulePreview::default());
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    // Revision the dialog opened on; a newer one at save time is a conflict.
    let opened_revision = RwSignal::new(0u32);

    // While closed, keep the fields on the saved job so they mount with its schedule
    // (Repeats and At are worked out from the cron when the fields mount).
    let sync_from_job = move |j: &crate::server::Job| {
        cron.set(j.cron.clone());
        timezone.set(display_timezone(j.timezone.as_deref().unwrap_or_default()));
        preview.set(SchedulePreview::default());
        error.set(None);
        opened_revision.set(j.revision);
    };
    sync_from_job(&job.get_untracked());
    Effect::new(move |_| {
        let j = job.get();
        if !open.get() {
            sync_from_job(&j);
        }
    });

    let before = Signal::derive(move || job.with(|j| upcoming_runs(j, 5)));
    let after = Signal::derive(move || preview.with(|p| p.next.clone()));
    let has_error = Signal::derive(move || {
        preview.with(|p| p.error.is_some() || p.next.is_empty())
            || cron.with(|c| c.trim().is_empty())
    });

    let on_save = Callback::new(move |_| {
        if saving.get() || has_error.get_untracked() {
            return;
        }
        let j = job.get_untracked();
        let payload = UpdateJobRequest {
            job_name: j.name.clone(),
            cron_expr: Some(cron.get_untracked().trim().to_string()),
            timezone: Some(timezone.get_untracked()),
            params: j.params.clone(),
            enabled: j.status == JobStatus::Active,
            pool: None,
            expected_revision: Some(opened_revision.get_untracked()),
        };
        saving.set(true);
        error.set(None);
        leptos::task::spawn_local_scoped(async move {
            match update_job(j.id.clone(), payload).await {
                Ok(UpdateJobOutcome::Saved {
                    revision,
                    next_run_at,
                }) => {
                    let next = next_run_at
                        .map_or_else(|| "none scheduled".to_string(), |t| format_run_time(&t));
                    ctx.flash.set(Some(format!(
                        "Schedule saved as revision {revision}. Next run: {next}."
                    )));
                    ctx.refresh.run(());
                    open.set(false);
                }
                Ok(conflict) => error.set(Some(format_conflict(&conflict))),
                Err(e) => error.set(Some(e.to_string())),
            }
            saving.set(false);
        });
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Edit schedule"</DialogTitle>
                    <DialogContent>
                        <div data-testid="schedule-dialog">
                            <Show when=move || open.get()>
                                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                                    <ScheduleFields cron=cron timezone=timezone preview=preview />
                                    <NextRunsTable
                                        runs=after
                                        before=before
                                        testid="schedule-dialog-preview"
                                    />
                                    <Caption1 attr:data-testid="schedule-dialog-summary">
                                        {move || {
                                            if has_error.get() {
                                                String::new()
                                            } else {
                                                save_summary(opened_revision.get() + 1, &before.get(), &after.get())
                                            }
                                        }}
                                    </Caption1>
                                    <Show when=move || error.get().is_some()>
                                        <div data-testid="schedule-dialog-error">
                                            <MessageBar intent=MessageBarIntent::Warning>
                                                {move || error.get().unwrap_or_default()}
                                            </MessageBar>
                                        </div>
                                    </Show>
                                </Flex>
                            </Show>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| open.set(false))>
                            "Cancel"
                        </Button>
                        <div data-testid="schedule-dialog-save">
                            <Button
                                appearance=ButtonAppearance::Primary
                                disabled=Signal::derive(move || saving.get() || has_error.get())
                                on_click=on_save
                            >
                                {move || if saving.get() { "Saving..." } else { "Save" }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::save_summary;

    #[test]
    fn summary_names_the_moved_run() {
        let before = vec!["2026-11-25T03:00:00+00:00".to_string()];
        let after = vec!["2026-11-25T04:00:00+00:00".to_string()];
        assert_eq!(
            save_summary(4, &before, &after),
            "Saving makes revision 4. The next run moves from Wed, Nov 25 at 03:00 UTC to \
             Wed, Nov 25 at 04:00 UTC. A run that's already going isn't stopped."
        );
    }
}
