use leptos::prelude::*;
use orbital::components::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, SpacingSize, Title3,
};
use orbital::primitives::{Button, ButtonAppearance, Flex, FlexAlign, FlexJustify, FlexWrap, Link};

use crate::components::RunStatusBadge;
use crate::pages::job_detail::{RunNowDialog, RunNowDialogInput, RunStarted, RunStartedNotice};
use crate::server::{RunDetail, RunStatus};

use super::{RunAttemptsCard, RunInfoGrid, RunLogsDisplay};

/// Loaded run detail body — values derived from `run_res` so refetch updates in place.
#[allow(clippy::too_many_lines)]
#[component]
pub fn RunDetailContent(
    /// Resource that loads the run detail.
    run_res: Resource<Result<Option<RunDetail>, ServerFnError>>,
) -> impl IntoView {
    let detail = Memo::new(move |_| run_res.get().and_then(|r| r.ok().flatten()));
    // First value for the parts that never change for this page (ids, names).
    let first = detail.get_untracked();
    let job_id = first
        .as_ref()
        .map(|d| d.run.job_id.clone())
        .unwrap_or_default();
    let job_name = first
        .as_ref()
        .map(|d| d.run.job_name.clone())
        .unwrap_or_default();

    let title = Memo::new(move |_| detail.get().map(|d| d.title).unwrap_or_default());
    let status = Memo::new(move |_| detail.get().map(|d| d.run.status));
    let can_run = Memo::new(move |_| detail.get().is_some_and(|d| d.can_run));
    let root_run_id =
        Signal::derive(move || detail.get().map(|d| d.root_run_id).unwrap_or_default());
    let current_run_id = Signal::derive(move || detail.get().map(|d| d.run.id).unwrap_or_default());
    let retry_caption =
        Signal::derive(move || detail.get().map(|d| d.retry_caption).unwrap_or_default());
    // Refresh the attempts list when this attempt's status changes (a retry may have started).
    let attempts_refresh = Signal::derive(move || {
        status.get().map_or(0, |s| match s {
            RunStatus::Pending => 1,
            RunStatus::Running => 2,
            RunStatus::Completed => 3,
            RunStatus::Failed => 4,
            RunStatus::Cancelled => 5,
            RunStatus::Refused => 6,
        })
    });

    // Run again: the Run now dialog, prefilled with this run's values.
    let dialog_open = RwSignal::new(false);
    let run_loading = RwSignal::new(false);
    let run_started = RwSignal::new(None::<RunStarted>);
    let dialog = (!job_id.is_empty()).then(|| {
        view! {
            <RunNowDialog
                props=RunNowDialogInput {
                    open: dialog_open,
                    job_id: job_id.clone(),
                    job_name: job_name.clone(),
                    initial_params: Signal::derive(move || detail.get().map(|d| d.params)),
                    loading: run_loading,
                    on_started: Callback::new(move |started| run_started.set(Some(started))),
                }
            />
        }
    });
    let job_href = chronon_backend::chronon_job_path(&job_id);

    view! {
        <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
            <div id="chronon-run-detail-breadcrumb" data-testid="chronon-run-breadcrumb">
                <Breadcrumb>
                    <BreadcrumbItem>
                        <Link href="/chronon/jobs">"Jobs"</Link>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                    <BreadcrumbItem>
                        <Link href=job_href>{job_name}</Link>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                    <BreadcrumbItem>
                        <BreadcrumbButton current=true>{move || title.get()}</BreadcrumbButton>
                    </BreadcrumbItem>
                </Breadcrumb>
            </div>

            <RunStartedNotice notice=run_started />

            <div id="chronon-run-detail-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center wrap=FlexWrap::Wrap>
                    <Flex align=FlexAlign::Center gap=SpacingSize::Size120.flex_gap()>
                        <div data-testid="chronon-run-title"><Title3>{move || title.get()}</Title3></div>
                        {move || status.get().map(|s| view! { <RunStatusBadge status=s /> })}
                    </Flex>
                    <Show when=move || can_run.get() fallback=|| ()>
                        <div data-testid="run-again-button">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                disabled=Signal::derive(move || run_loading.get())
                                on_click=Callback::new(move |_| dialog_open.set(true))
                            >
                                "Run again"
                            </Button>
                        </div>
                    </Show>
                </Flex>
            </div>
            {dialog}

            <div id="chronon-run-detail-timing">
                {move || detail.get().map(|d| view! { <RunInfoGrid detail=d /> })}
            </div>

            <RunAttemptsCard
                root_run_id=root_run_id
                current_run_id=current_run_id
                refresh=attempts_refresh
                caption=retry_caption
            />

            <div id="chronon-run-detail-output">
                {move || detail.get().map(|d| view! {
                    <RunLogsDisplay
                        logs=d.run.logs
                        stderr=d.run.stderr
                        error_message=d.run.error_message
                        running=matches!(d.run.status, RunStatus::Running | RunStatus::Pending)
                    />
                })}
            </div>
        </Flex>
    }
}
