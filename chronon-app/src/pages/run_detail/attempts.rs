use leptos::prelude::*;
use orbital::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, CardHeader, CardSectionBorder, SpacingSize,
    Subtitle2,
};
use orbital::primitives::{Flex, FlexAlign, Grid, GridConfig, GridItem, Link};

use crate::components::RunStatusBadge;
use crate::server::{get_run_attempts, RunAttempt};

/// Attempts card: the first try and every retry, with when it started, how it ended and how
/// long it took. The attempt on screen is marked "(this one)".
#[component]
pub fn RunAttemptsCard(
    /// Run id of the first attempt.
    root_run_id: Signal<String>,
    /// Run id of the attempt this page shows.
    current_run_id: Signal<String>,
    /// Bumped when the run changes (live events), so retries appear as they start.
    refresh: Signal<u32>,
    /// Caption under the list, such as `Retries run 30 s apart. The job allows 3.`
    caption: Signal<String>,
) -> impl IntoView {
    // Keyed on the run it lists (and the live refresh) only.
    let attempts = LocalResource::new(move || {
        let id = root_run_id.get();
        let _ = refresh.get();
        async move { get_run_attempts(id).await.unwrap_or_default() }
    });
    view! {
        <div id="chronon-run-detail-attempts" data-testid="chronon-run-attempts">
        <Card>
            <CardHeader><Subtitle2>"Attempts"</Subtitle2></CardHeader>
            <CardSectionBorder />
            <CardContent>
                <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                    <Grid config=GridConfig::with_gaps(4, 16, 8)>
                        <Suspense fallback=|| ()>
                        {move || {
                            attempts.get().unwrap_or_default().into_iter().map(|a| {
                                view! { <AttemptRow attempt=a current=current_run_id.get_untracked() /> }
                            }).collect_view()
                        }}
                        </Suspense>
                    </Grid>
                    <Caption1>{move || caption.get()}</Caption1>
                </Flex>
            </CardContent>
        </Card>
        </div>
    }
}

#[component]
fn AttemptRow(attempt: RunAttempt, current: String) -> impl IntoView {
    let this_one = attempt.run_id == current;
    let href = chronon_backend::chronon_run_path(&attempt.run_id);
    let duration = attempt
        .duration_ms
        .map_or_else(|| "\u{2014}".to_string(), chronon_backend::duration_words);
    view! {
        <GridItem>
            <Flex align=FlexAlign::Center gap=SpacingSize::Size40.flex_gap() attr:data-testid="chronon-run-attempt">
                {if this_one {
                    view! { <Body1>{attempt.label}</Body1><Body1Strong>"(this one)"</Body1Strong> }.into_any()
                } else {
                    view! { <Link href=href>{attempt.label}</Link> }.into_any()
                }}
            </Flex>
        </GridItem>
        <GridItem><Body1>{crate::utils::format_started_at(&attempt.started_at)}</Body1></GridItem>
        <GridItem><RunStatusBadge status=attempt.result /></GridItem>
        <GridItem><Body1>{duration}</Body1></GridItem>
    }
}
