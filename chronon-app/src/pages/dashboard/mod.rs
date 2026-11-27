mod coming_up;
mod needs_attention;
mod recent_runs_table;
mod stats_grid;

pub use coming_up::ComingUpCard;
pub use needs_attention::NeedsAttentionCard;
pub use recent_runs_table::RecentRunsTable;
pub use stats_grid::ChrononStatsGrid;

use leptos::prelude::*;
use orbital::components::{
    Body1, ContentContainer, Skeleton, SkeletonItem, SkeletonItemSize, SpacingSize, Title3,
};
use orbital::primitives::{Flex, TableCell, TableCellLayout, TableRow};

use crate::live::use_chronon_poll_tick;
use crate::server::{
    get_attention, get_coming_up, get_dashboard_stats, get_recent_runs, Attention, ComingUpRun,
    DashboardStats, RunSummary,
};

/// Upcoming runs the Coming up card lists.
const COMING_UP_COUNT: u32 = 5;
/// Runs the Recent runs card lists.
const RECENT_RUN_COUNT: u32 = 6;

/// The dashboard's four loads, shared so one coordinator outage shows one error.
#[derive(Clone, Copy)]
pub struct DashboardData {
    pub attention: Resource<Result<Attention, ServerFnError>>,
    pub stats: Resource<Result<DashboardStats, ServerFnError>>,
    pub coming_up: Resource<Result<Vec<ComingUpRun>, ServerFnError>>,
    pub recent: Resource<Result<Vec<RunSummary>, ServerFnError>>,
}

impl DashboardData {
    /// True when Needs attention, the stats and Coming up all failed: the coordinator is
    /// down, and the page shows one error with Try again in place of three.
    pub fn outage(self) -> bool {
        matches!(self.attention.get(), Some(Err(_)))
            && matches!(self.stats.get(), Some(Err(_)))
            && matches!(self.coming_up.get(), Some(Err(_)))
    }

    /// Loads everything again.
    pub fn refetch(self) {
        self.attention.refetch();
        self.stats.refetch();
        self.coming_up.refetch();
        self.recent.refetch();
    }
}

/// Dashboard page: what needs attention, the stat cards, what runs next and the newest runs.
#[component]
pub fn ChrononDashboardPage() -> impl IntoView {
    let poll_tick = use_chronon_poll_tick();
    // Keyed on nothing, so polling refetches in place and nothing remounts.
    let data = DashboardData {
        attention: Resource::new(|| (), |()| async move { get_attention().await }),
        stats: Resource::new(|| (), |()| async move { get_dashboard_stats().await }),
        coming_up: Resource::new(
            || (),
            |()| async move { get_coming_up(COMING_UP_COUNT).await },
        ),
        recent: Resource::new(
            || (),
            |()| async move { get_recent_runs(RECENT_RUN_COUNT).await },
        ),
    };

    Effect::new(move |_| {
        if poll_tick.get() > 0 {
            data.refetch();
        }
    });

    view! {
        <ContentContainer data_testid="chronon-dashboard">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                    <Title3>"Chronon"</Title3>
                    <Body1>"Scheduled jobs on this site: what needs you, and what runs next."</Body1>
                </Flex>
                <NeedsAttentionCard data=data />
                <ChrononStatsGrid data=data />
                <ComingUpCard data=data />
                <RecentRunsTable data=data />
            </Flex>
        </ContentContainer>
    }
}

/// Placeholder table rows while a dashboard table loads; the real headers stay above them.
#[component]
fn TableSkeletonRows(
    /// Rows to draw.
    rows: usize,
    /// Cells per row.
    cols: usize,
) -> impl IntoView {
    let size = Signal::from(SkeletonItemSize::S16);
    // Skeleton is a div, so it goes inside each cell: a div directly in <tbody> breaks
    // hydration.
    (0..rows)
        .map(|_| {
            view! {
                <TableRow>
                    {(0..cols).map(|_| view! {
                        <TableCell>
                            <TableCellLayout>
                                <Skeleton>
                                    <SkeletonItem size=size />
                                </Skeleton>
                            </TableCellLayout>
                        </TableCell>
                    }).collect_view()}
                </TableRow>
            }
        })
        .collect_view()
}
