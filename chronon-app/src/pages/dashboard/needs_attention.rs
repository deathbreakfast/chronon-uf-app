use leptos::prelude::*;
use orbital::components::{
    Card, CardContent, CardHeader, Skeleton, SkeletonItem, SkeletonItemSize, SpacingSize, Subtitle2,
};
use orbital::primitives::{
    Button, ButtonAppearance, Flex, Link, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarTitle,
};

use super::DashboardData;
use crate::server::{AttentionItem, AttentionKind};

/// Needs attention: one bar per job whose last run failed, that's paused, or that missed a
/// run, or a success bar when nothing applies. Also holds the single coordinator-outage bar.
#[component]
pub fn NeedsAttentionCard(
    /// The dashboard's loads.
    data: DashboardData,
) -> impl IntoView {
    let skeleton_size = Signal::from(SkeletonItemSize::S40);
    view! {
        <div id="chronon-dashboard-attention" data-testid="chronon-dashboard-attention">
        <Card>
            <CardHeader>
                <Subtitle2>"Needs attention"</Subtitle2>
            </CardHeader>
            <CardContent>
                <Transition fallback=move || view! {
                    <Skeleton>
                        <SkeletonItem size=skeleton_size />
                    </Skeleton>
                }>
                    {move || data.attention.get().map(|r| match r {
                        Ok(attention) if attention.items.is_empty() => {
                            let runs = match attention.runs_today {
                                1 => "1 run so far today".to_string(),
                                n => format!("{n} runs so far today"),
                            };
                            view! {
                                <div data-testid="chronon-attention-clear">
                                    <MessageBar intent=MessageBarIntent::Success>
                                        <MessageBarBody>
                                            <MessageBarTitle>"Nothing needs attention"</MessageBarTitle>
                                            {format!("No failed, paused or late jobs. {runs}.")}
                                        </MessageBarBody>
                                    </MessageBar>
                                </div>
                            }.into_any()
                        }
                        Ok(attention) => view! {
                            <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                                {attention.items.into_iter().map(|item| view! { <AttentionBar item=item /> }).collect_view()}
                            </Flex>
                        }.into_any(),
                        Err(e) if data.outage() => view! {
                            <div data-testid="chronon-dashboard-outage">
                                <MessageBar intent=MessageBarIntent::Error>
                                    <MessageBarBody>
                                        <MessageBarTitle>"Chronon isn't answering"</MessageBarTitle>
                                        {format!("The dashboard couldn't reach the Chronon coordinator: {e}")}
                                    </MessageBarBody>
                                    <MessageBarActions>
                                        <div data-testid="chronon-dashboard-retry">
                                            <Button
                                                appearance=ButtonAppearance::Secondary
                                                on_click=Callback::new(move |_| data.refetch())
                                            >
                                                "Try again"
                                            </Button>
                                        </div>
                                    </MessageBarActions>
                                </MessageBar>
                            </div>
                        }.into_any(),
                        Err(e) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                <MessageBarBody>
                                    {format!("Couldn't check which jobs need attention: {e}")}
                                </MessageBarBody>
                            </MessageBar>
                        }.into_any(),
                    })}
                </Transition>
            </CardContent>
        </Card>
        </div>
    }
}

/// One Needs attention line: what happened, then Open the job (and See the run for a failure).
#[component]
fn AttentionBar(item: AttentionItem) -> impl IntoView {
    let (intent, kind) = match item.kind {
        AttentionKind::Failed => (MessageBarIntent::Error, "failed"),
        AttentionKind::Paused => (MessageBarIntent::Warning, "paused"),
        AttentionKind::Missed => (MessageBarIntent::Warning, "missed"),
    };
    let job_href = chronon_backend::chronon_job_path(&item.job_id);
    let run_href = item
        .run_id
        .as_deref()
        .map(chronon_backend::chronon_run_path);
    view! {
        <div data-testid="chronon-attention-item" data-kind=kind>
            <MessageBar intent=intent>
                <MessageBarBody>
                    <MessageBarTitle>{item.title}</MessageBarTitle>
                    {item.detail}
                </MessageBarBody>
                <MessageBarActions>
                    <span data-testid="chronon-attention-open-job">
                        <Link href=job_href>"Open the job"</Link>
                    </span>
                    {run_href.map(|href| view! {
                        <span data-testid="chronon-attention-see-run">
                            <Link href=href>"See the run"</Link>
                        </span>
                    })}
                </MessageBarActions>
            </MessageBar>
        </div>
    }
}
