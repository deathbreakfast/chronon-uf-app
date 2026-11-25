use leptos::prelude::*;
use orbital::components::{
    Caption1, Card, CardContent, CardHeader, SpacingSize, Subtitle1, Subtitle2,
};
use orbital::primitives::{Button, ButtonAppearance, Code, Flex, FlexAlign, FlexJustify};
use turf::inline_style_sheet_values;

use crate::components::NextRunsTable;
use crate::server::Job;

use super::super::display::display_timezone;
use super::super::loaded::JobPageCtx;

/// The next `n` runs of a cron job (RFC 3339), computed with the scheduler's own rules.
pub fn upcoming_runs(job: &Job, n: usize) -> Vec<String> {
    if job.cron.trim().is_empty() {
        return job.next_run_at.clone().into_iter().collect();
    }
    let tz = display_timezone(job.timezone.as_deref().unwrap_or_default());
    chronon_backend::next_runs(&job.cron, &tz, chrono::Utc::now(), n)
        .into_iter()
        .map(|t| t.to_rfc3339())
        .collect()
}

/// Schedule card: the sentence, the cron and timezone, and the next five runs.
#[component]
pub fn ScheduleCard(
    /// Shared page state.
    ctx: JobPageCtx,
) -> impl IntoView {
    let job = ctx.job;
    let access = ctx.access;
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .CronChip {
            padding: 0 var(--orb-space-inline-md);
            line-height: var(--orb-type-line-md);
        }
    };
    let is_cron = Signal::derive(move || job.with(|j| !j.cron.trim().is_empty()));
    let sentence = move || {
        job.with(|j| {
            if j.schedule_text.is_empty() {
                "Hidden".to_string()
            } else {
                j.schedule_text.clone()
            }
        })
    };
    let runs = Signal::derive(move || job.with(|j| upcoming_runs(j, 5)));

    view! {
        <style>{style_sheet}</style>
        <div id="chronon-job-detail-schedule" data-testid="schedule-card">
            <Card>
                <CardHeader>
                    <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                        <Subtitle2>"Schedule"</Subtitle2>
                        <Show when=move || access.get().can_edit && is_cron.get()>
                            <div data-testid="edit-schedule-button">
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    icon=icondata::AiEditOutlined
                                    on_click=Callback::new(move |_| ctx.schedule_dialog_open.set(true))
                                >
                                    "Edit schedule"
                                </Button>
                            </div>
                        </Show>
                    </Flex>
                </CardHeader>
                <CardContent>
                    <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                        <Subtitle1 attr:data-testid="schedule-card-sentence">{sentence}</Subtitle1>
                        <Show when=move || is_cron.get()>
                            <Caption1 attr:data-testid="schedule-card-cron">
                                "Cron "
                                {move || view! { <Code class=class_names.cron_chip text=job.with(|j| j.cron.clone()) /> }}
                                {move || format!(
                                    " · Timezone {}",
                                    display_timezone(job.with(|j| j.timezone.clone()).as_deref().unwrap_or_default())
                                )}
                            </Caption1>
                        </Show>
                        <Show
                            when=move || !runs.get().is_empty()
                            fallback=|| view! { <Caption1>"No runs coming up."</Caption1> }
                        >
                            <Subtitle2>"Next runs"</Subtitle2>
                            <NextRunsTable runs=runs testid="schedule-card-next-runs" />
                        </Show>
                    </Flex>
                </CardContent>
            </Card>
        </div>
    }
}
