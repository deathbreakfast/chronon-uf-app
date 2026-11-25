//! Job detail page orchestration and section composition.

mod actions;
mod components;

// Run detail's Run again opens the same dialog.
pub use components::{RunNowDialog, RunNowDialogInput, RunStarted, RunStartedNotice};
mod display;
mod loaded;
mod skeleton;

use skeleton::JobDetailPageSkeleton;

use leptos::prelude::*;
use leptos_router::components::A;
use orbital::components::{ContentContainer, EmptyState, EmptyStateCallToAction};
use orbital::primitives::{Button, ButtonAppearance, MessageBar, MessageBarIntent};

use crate::live::{chronon_job_run_subscription, ChrononJobRunLiveSource};
use crate::server::{get_job, get_scripts};
use loaded::JobDetailLoaded;

/// Job detail page — orchestrator.
///
/// Loads the job once per `job_id` and hands it to `JobDetailLoaded`, which keeps its
/// own copy and reloads it after saves, so tabs, dialogs and messages never remount.
#[component]
pub fn ChrononJobDetailPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let job_id = move || params.get().get("job_id").unwrap_or_default();

    let job_res = Resource::new(job_id, |id| async move { get_job(id).await });
    let scripts_res = Resource::new(|| (), |()| async { get_scripts().await });

    let live = chronon_job_run_subscription();
    let runs_refresh = RwSignal::new(0u32);
    let job_id_for_live = Memo::new(move |_| {
        let id = job_id();
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    });

    Effect::new(move |_| {
        let _ = live.trigger.get();
        let id = job_id();
        if id.is_empty() {
            return;
        }
        if let Some(ev) = live.latest_event.get() {
            if crate::live::chronon_run_event_matches_job(&ev, &id) {
                runs_refresh.update(|n| *n += 1);
            }
        }
    });

    let runs_refresh_signal = Signal::derive(move || runs_refresh.get());

    view! {
        <ChrononJobRunLiveSource
            job_id=Signal::derive(move || job_id_for_live.get())
            trigger=live.trigger
            latest_event=live.latest_event
        />
        <ContentContainer data_testid="chronon-job-detail">
            <Transition fallback=move || view! { <JobDetailPageSkeleton /> }>
                {move || job_res.get().map(|r| match r {
                        Ok(Some(job)) => view! {
                            <JobDetailLoaded
                                job=job
                                scripts_res=scripts_res
                                live=live
                                runs_refresh_signal=runs_refresh_signal
                            />
                        }
                        .into_any(),
                        Ok(None) => view! {
                            <div data-testid="job-not-found">
                                <EmptyState
                                    message="Job not found."
                                    description="It may have been deleted, or the link has a typo."
                                    icon=icondata::AiSearchOutlined
                                >
                                    <EmptyStateCallToAction slot:call_to_action>
                                        <A href=crate::paths::JOBS>
                                            <Button appearance=ButtonAppearance::Primary>
                                                "Back to Jobs"
                                            </Button>
                                        </A>
                                    </EmptyStateCallToAction>
                                </EmptyState>
                            </div>
                        }
                        .into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                "Failed to load job: " {err.to_string()}
                            </MessageBar>
                        }
                        .into_any(),
                    })}
            </Transition>
        </ContentContainer>
    }
}
