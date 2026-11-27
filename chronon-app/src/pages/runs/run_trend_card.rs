use leptos::prelude::*;
use orbital::components::{
    Button, Caption1, Card, CardContent, CardHeader, Flex, FlexGap, FlexWrap, InfoLabel,
    InfoLabelInfo, MessageBar, MessageBarIntent, Skeleton, SkeletonItem, Subtitle2,
};
use orbital::primitives::ButtonAppearance;

use super::charts::line_chart_from_series;
use crate::server::get_run_stats_series;

const RANGE_24H: i64 = 86_400;
const RANGE_7D: i64 = 604_800;

/// Runs page card showing a success/failure run-count trend chart with a selectable time range.
#[component]
pub fn RunTrendCard() -> impl IntoView {
    let range_secs = RwSignal::new(RANGE_7D);
    let res = Resource::new(
        move || range_secs.get(),
        |secs| async move { get_run_stats_series(secs).await },
    );

    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .ChartSkeleton { width: 100%; height: 280px; }
        .Toolbar { margin-bottom: var(--spacingVerticalM); }
    };

    view! {
        <style>{style_sheet}</style>
        <div id="chronon-run-trend-card">
        <Card>
            <CardHeader>
                <InfoLabel>
                    <Subtitle2>"Run outcomes"</Subtitle2>
                    <InfoLabelInfo slot>
                        <Caption1>
                            "Successful and failed runs bucketed in UTC. Chart data is sampled from recent run history."
                        </Caption1>
                    </InfoLabelInfo>
                </InfoLabel>
            </CardHeader>
            <CardContent>
                <Flex class=class_names.toolbar gap=FlexGap::Small wrap=FlexWrap::Wrap>
                    <div data-testid="run-trend-range-24h">
                        <Button
                            appearance=Signal::derive(move || {
                                if range_secs.get() == RANGE_24H {
                                    ButtonAppearance::Primary
                                } else {
                                    ButtonAppearance::Secondary
                                }
                            })
                            on:click=move |_| range_secs.set(RANGE_24H)
                        >
                            "24h"
                        </Button>
                    </div>
                    <div data-testid="run-trend-range-7d">
                        <Button
                            appearance=Signal::derive(move || {
                                if range_secs.get() == RANGE_7D {
                                    ButtonAppearance::Primary
                                } else {
                                    ButtonAppearance::Secondary
                                }
                            })
                            on:click=move |_| range_secs.set(RANGE_7D)
                        >
                            "7d"
                        </Button>
                    </div>
                </Flex>

                <div data-testid="chronon-run-trend-chart">
                    <Transition fallback=move || view! {
                        <Skeleton>
                            <SkeletonItem class=class_names.chart_skeleton />
                        </Skeleton>
                    }>
                        {move || res.get().map(|r| match r {
                            Ok(series) if series.iter().all(|s| s.points.is_empty()) => view! {
                                <MessageBar intent=MessageBarIntent::Info>
                                    "No runs in this time range."
                                </MessageBar>
                            }.into_any(),
                            Ok(series) => {
                                let use_daily = range_secs.get() > RANGE_24H;
                                view! {
                                    {line_chart_from_series(&series, 280.0, use_daily)}
                                }.into_any()
                            }
                            Err(e) => view! {
                                <MessageBar intent=MessageBarIntent::Error>
                                    "Failed to load chart: " {e.to_string()}
                                </MessageBar>
                            }.into_any(),
                        })}
                    </Transition>
                </div>
            </CardContent>
        </Card>
        </div>
    }
}
