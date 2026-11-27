use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use orbital::components::{
    Body1, Caption1, Card, CardContent, CardHeader, EmptyState, SpacingSize, Subtitle2,
};
use orbital::primitives::{
    Flex, FlexAlign, FlexJustify, Link, MessageBar, MessageBarIntent, Table, TableBody, TableCell,
    TableHeader, TableHeaderCell, TableRow,
};

use super::{DashboardData, TableSkeletonRows};
use crate::components::RunStatusBadge;
use crate::server::RunSummary;

const SKELETON_ROW_COUNT: usize = 6;

/// `Today, 09:35` for an RFC 3339 time.
fn started_label(started_at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(started_at).map_or_else(
        |_| started_at.to_string(),
        |t| chronon_backend::day_time_label(t.with_timezone(&chrono::Utc), chrono::Utc::now()),
    )
}

/// Recent runs: the newest runs with retries folded in, who started them, and a row click to
/// the run.
#[component]
pub fn RecentRunsTable(
    /// The dashboard's loads.
    data: DashboardData,
) -> impl IntoView {
    let runs_res = data.recent;
    view! {
        <div id="chronon-dashboard-recent-runs">
        <Card>
            <CardHeader>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <Subtitle2>"Recent runs"</Subtitle2>
                    <span data-testid="chronon-recent-runs-all-runs">
                        <Link href=crate::paths::RUNS>"All runs"</Link>
                    </span>
                </Flex>
            </CardHeader>
            <CardContent>
                <Transition fallback=move || view! { <RecentRunsRows rows=None /> }>
                    {move || runs_res.get().map(|r| match r {
                        Ok(list) if list.is_empty() => view! {
                            <EmptyState
                                message="No runs yet."
                                description="Runs appear here after jobs run. Create a job or use Run now."
                            />
                        }.into_any(),
                        Ok(list) => view! { <RecentRunsRows rows=Some(list) /> }.into_any(),
                        Err(_) if data.outage() => ().into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                "Failed to load runs: " {err.to_string()}
                            </MessageBar>
                        }.into_any(),
                    })}
                </Transition>
            </CardContent>
        </Card>
        </div>
    }
}

/// Job / Result / Started / Started by table; `None` shows skeleton rows under the headers.
#[component]
fn RecentRunsRows(rows: Option<Vec<RunSummary>>) -> impl IntoView {
    let navigate = use_navigate();
    let nav_store = StoredValue::new(navigate);
    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .Row { cursor: pointer; }
        .Row:hover { background: var(--colorNeutralBackground1Hover); }
    };

    view! {
        <style>{style_sheet}</style>
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"Job"</TableHeaderCell>
                    <TableHeaderCell>"Result"</TableHeaderCell>
                    <TableHeaderCell>"Started"</TableHeaderCell>
                    <TableHeaderCell>"Started by"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {rows.map_or_else(
                    || view! { <TableSkeletonRows rows=SKELETON_ROW_COUNT cols=4 /> }.into_any(),
                    |rows| rows.into_iter().map(|run| {
                        let run_href = chronon_backend::chronon_run_path(&run.root_run_id);
                        let job_href = chronon_backend::chronon_job_path(&run.job_id);
                        let attempts = (run.attempts > 1)
                            .then(|| format!("\u{b7} {n} of {n} attempts", n = run.attempts));
                        let nav = nav_store.get_value();
                        view! {
                            <TableRow
                                class=class_names.row
                                attr:data-testid="chronon-recent-run-row"
                                on:click=move |_| nav(&run_href, NavigateOptions::default())
                            >
                                <TableCell>
                                    // The job link opens the job, not the run.
                                    <span on:click=|ev| ev.stop_propagation()>
                                        <Link href=job_href>{run.job_name}</Link>
                                    </span>
                                </TableCell>
                                <TableCell>
                                    <Flex align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                                        <RunStatusBadge status=run.result />
                                        {attempts.map(|a| view! { <Caption1>{a}</Caption1> })}
                                    </Flex>
                                </TableCell>
                                <TableCell>
                                    <Body1>{started_label(&run.started_at)}</Body1>
                                </TableCell>
                                <TableCell>
                                    <Caption1>{run.started_by.short_label()}</Caption1>
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view().into_any(),
                )}
            </TableBody>
        </Table>
    }
}
