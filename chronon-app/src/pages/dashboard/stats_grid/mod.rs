mod kpis;
mod skeleton;

use leptos::prelude::*;
use orbital::components::{AutoGrid, StatCard};
use orbital::primitives::{MessageBar, MessageBarIntent};

use kpis::{dashboard_stat_memos, KPI_DEFS};
use skeleton::ChrononStatsSkeleton;

/// Minimum stat card width in the grid.
pub const STAT_CARD_MIN: &str = "170px";

/// Stat cards: failing jobs, runs today, the share that worked in 24 hours, running now and
/// paused.
#[component]
pub fn ChrononStatsGrid(
    /// The dashboard's loads.
    data: super::DashboardData,
) -> impl IntoView {
    let stats_res = data.stats;
    let memos = StoredValue::new(dashboard_stat_memos(stats_res));

    view! {
        <Transition fallback=move || view! { <ChrononStatsSkeleton /> }>
            {move || stats_res.get().map(|r| match r {
                Ok(_) => view! {
                    <div id="chronon-dashboard-stats">
                    <AutoGrid min=STAT_CARD_MIN>
                        {KPI_DEFS.iter().enumerate().map(|(i, def)| {
                            let value = Signal::derive(move || {
                                memos.with_value(|m| m[i].get())
                            });
                            let label = def.label;
                            let variant = def.variant;
                            let test_id = def.test_id;
                            view! {
                                <div data-testid=test_id>
                                    {def.icon.map_or_else(|| view! {
                                        <StatCard
                                            label=label
                                            value=value
                                            variant=variant
                                        />
                                    }.into_any(), |icon| view! {
                                        <StatCard
                                            label=label
                                            value=value
                                            icon=icon
                                            variant=variant
                                        />
                                    }.into_any())}
                                </div>
                            }
                        }).collect_view()}
                    </AutoGrid>
                    </div>
                }.into_any(),
                // A coordinator outage already shows one error under Needs attention.
                Err(_) if data.outage() => ().into_any(),
                Err(e) => view! {
                    <MessageBar intent=MessageBarIntent::Error>
                        "Failed to load stats: " {e.to_string()}
                    </MessageBar>
                }.into_any(),
            })}
        </Transition>
    }
}
