use leptos::prelude::*;
use orbital::components::{Body1, Caption1, Card, CardContent, CardHeader, EmptyState, Subtitle2};
use orbital::primitives::{
    Flex, FlexAlign, FlexJustify, Link, MessageBar, MessageBarIntent, Table, TableBody, TableCell,
    TableHeader, TableHeaderCell, TableRow,
};

use super::{DashboardData, TableSkeletonRows};
use crate::server::ComingUpRun;

const SKELETON_ROW_COUNT: usize = 5;

/// Coming up: the next runs across every job, with the schedule in words.
#[component]
pub fn ComingUpCard(
    /// The dashboard's loads.
    data: DashboardData,
) -> impl IntoView {
    view! {
        <div id="chronon-dashboard-coming-up" data-testid="chronon-dashboard-coming-up">
        <Card>
            <CardHeader>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <Subtitle2>"Coming up"</Subtitle2>
                    <span data-testid="chronon-coming-up-all-jobs">
                        <Link href=crate::paths::JOBS>"All jobs"</Link>
                    </span>
                </Flex>
            </CardHeader>
            <CardContent>
                <Transition fallback=move || view! { <ComingUpTable rows=None /> }>
                    {move || data.coming_up.get().map(|r| match r {
                        Ok(rows) if rows.is_empty() => view! {
                            <EmptyState
                                message="Nothing is scheduled."
                                description="Active jobs with a schedule show their next runs here."
                            />
                        }.into_any(),
                        Ok(rows) => view! { <ComingUpTable rows=Some(rows) /> }.into_any(),
                        Err(_) if data.outage() => ().into_any(),
                        Err(e) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                {format!("Couldn't load the next runs: {e}")}
                            </MessageBar>
                        }.into_any(),
                    })}
                </Transition>
            </CardContent>
        </Card>
        </div>
    }
}

/// The When / Job / Schedule table; `None` shows skeleton rows under the real headers.
#[component]
fn ComingUpTable(rows: Option<Vec<ComingUpRun>>) -> impl IntoView {
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"When"</TableHeaderCell>
                    <TableHeaderCell>"Job"</TableHeaderCell>
                    <TableHeaderCell>"Schedule"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {rows.map_or_else(
                    || view! { <TableSkeletonRows rows=SKELETON_ROW_COUNT cols=3 /> }.into_any(),
                    |rows| rows.into_iter().map(|run| {
                        let schedule = if run.first_run {
                            format!("{} \u{b7} first run", run.schedule_text)
                        } else {
                            run.schedule_text
                        };
                        let href = chronon_backend::chronon_job_path(&run.job_id);
                        view! {
                            <TableRow attr:data-testid="chronon-coming-up-row">
                                <TableCell>
                                    <Flex vertical=true>
                                        <Body1>{run.when}</Body1>
                                        <Caption1>{run.in_words}</Caption1>
                                    </Flex>
                                </TableCell>
                                <TableCell>
                                    <Link href=href>{run.job_name}</Link>
                                </TableCell>
                                <TableCell>
                                    <Caption1>{schedule}</Caption1>
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view().into_any(),
                )}
            </TableBody>
        </Table>
    }
}
