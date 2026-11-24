mod attempts;
mod content;
mod run_info_grid;
mod run_logs_display;
mod skeleton;

pub use attempts::RunAttemptsCard;
pub use run_info_grid::RunInfoGrid;
pub use run_logs_display::RunLogsDisplay;

use content::RunDetailContent;
use leptos::prelude::*;
use orbital::components::{ContentContainer, EmptyState, EmptyStateCallToAction};
use orbital::primitives::{Link, MessageBar, MessageBarIntent};

use crate::live::{
    chronon_job_run_subscription, chronon_run_event_matches_run, ChrononJobRunLiveSource,
};
use crate::server::get_run_detail;
use skeleton::RunDetailPageSkeleton;

/// Run detail page.
#[component]
pub fn ChrononRunDetailPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let run_id = move || params.get().get("run_id").unwrap_or_default();

    let live = chronon_job_run_subscription();
    let run_res = Resource::new(run_id, |id| async move { get_run_detail(id).await });

    let job_id_for_live = Memo::new(move |_| {
        run_res
            .get()
            .and_then(|r| r.ok().flatten())
            .map(|r| r.run.job_id)
            .filter(|id| !id.is_empty())
    });

    Effect::new(move |_| {
        let _ = live.trigger.get();
        let current_run = run_id();
        if current_run.is_empty() {
            return;
        }
        if let Some(ev) = live.latest_event.get() {
            if chronon_run_event_matches_run(&ev, &current_run) {
                run_res.refetch();
            }
        }
    });

    view! {
        <ChrononJobRunLiveSource
            job_id=Signal::derive(move || job_id_for_live.get())
            trigger=live.trigger
            latest_event=live.latest_event
        />
        <ContentContainer data_testid="chronon-run-detail">
            <Transition fallback=move || view! { <RunDetailPageSkeleton /> }>
                {move || run_res.get().map(|r| match r {
                    Ok(Some(_)) => view! { <RunDetailContent run_res=run_res /> }.into_any(),
                    Ok(None) => view! {
                        <div data-testid="chronon-run-not-found">
                            <EmptyState
                                message="Run not found"
                                description="The run may have been removed, or the link is wrong."
                            >
                                <EmptyStateCallToAction slot:call_to_action>
                                    <Link href="/chronon/runs">"Back to Runs"</Link>
                                </EmptyStateCallToAction>
                            </EmptyState>
                        </div>
                    }.into_any(),
                    Err(err) => view! {
                        <MessageBar intent=MessageBarIntent::Error>
                            "Failed to load run: " {err.to_string()}
                        </MessageBar>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}
