use crate::components::{RunsDataTable, RunsTableChrome, RunsTableFilterState, RunsTableScope};
use crate::server::{get_job_week, DayResult, JobWeekDay, JOB_RUNS_PAGE_SIZE};
use leptos::prelude::*;
use orbital::components::{
    Caption1, Card, CardContent, CardHeader, CardSectionBorder, SpacingSize, Subtitle2,
};
use orbital::primitives::{
    Badge, BadgeAppearance, BadgeColor, Flex, FlexAlign, FlexJustify, FlexWrap, Link, MessageBar,
    MessageBarActions, MessageBarBody, MessageBarIntent, MessageBarTitle,
};

/// The job's Runs card: a link to all of its runs, the last seven days as badges, a line
/// that sums up the recent runs, and the job's runs table (retries folded in).
#[component]
pub fn JobRecentRuns(
    /// The job ID to fetch runs for
    job_id: String,
    /// Bumped when live run events arrive for this job (refreshes table without remount).
    refresh_signal: Signal<u32>,
    /// Show the result and time filters above the table (the job page's Runs tab).
    #[prop(optional)]
    show_filters: bool,
) -> impl IntoView {
    let filters = show_filters.then(|| {
        let filters = RunsTableFilterState::from_url("any");
        filters.sync_to_url();
        filters
    });
    let week_job_id = job_id.clone();
    let week = LocalResource::new(move || {
        let id = week_job_id.clone();
        let _ = refresh_signal.get();
        async move { get_job_week(id).await.ok() }
    });
    let all_runs_href = format!(
        "/chronon/runs?job={}&time=any",
        chronon_backend::encode_ops_path_segment(&job_id)
    );

    view! {
        <div id="chronon-job-detail-recent-runs">
        <Card attr:data-testid="chronon-recent-runs-card">
            <CardHeader>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <Subtitle2>"Runs"</Subtitle2>
                    <div data-testid="chronon-job-all-runs-link">
                        <Link href=all_runs_href>"All runs of this job"</Link>
                    </div>
                </Flex>
            </CardHeader>
            <CardSectionBorder />
            <CardContent>
                <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                    <Flex
                        align=FlexAlign::Center
                        gap=SpacingSize::Size80.flex_gap()
                        wrap=FlexWrap::Wrap
                        attr:data-testid="chronon-job-week"
                    >
                        <Caption1>"Last 7 days"</Caption1>
                        <Suspense fallback=|| ()>
                        {move || {
                            week.get()
                                .flatten()
                                .map(|w| w.days.into_iter().map(|day| view! { <DayBadge day=day /> }).collect_view())
                        }}
                        </Suspense>
                    </Flex>
                    <div data-testid="chronon-job-runs-summary">
                        <Caption1>
                            <Suspense fallback=|| ()>
                                {move || week.get().flatten().map(|w| w.summary).unwrap_or_default()}
                            </Suspense>
                        </Caption1>
                    </div>
                    <RunsDataTable
                        scope=RunsTableScope::ForJob(job_id)
                        chrome=RunsTableChrome {
                            show_job_column: false,
                            show_card_header: false,
                            fill_height: false,
                            infinite_scroll: true,
                        }
                        max_height=400.0
                        page_size=JOB_RUNS_PAGE_SIZE
                        refresh_signal=refresh_signal
                        filters=filters
                    />
                </Flex>
            </CardContent>
        </Card>
        </div>
    }
}

/// One day of the last seven: filled green when it worked, red when it failed.
#[component]
fn DayBadge(day: JobWeekDay) -> impl IntoView {
    let (appearance, color, word) = match day.result {
        DayResult::Worked => (BadgeAppearance::Filled, BadgeColor::Success, "worked"),
        DayResult::Failed => (BadgeAppearance::Filled, BadgeColor::Danger, "failed"),
        DayResult::None => (BadgeAppearance::Outline, BadgeColor::Subtle, "no run"),
    };
    let title = format!("{}: {word}", day.label);
    view! {
        <span title=title.clone() aria-label=title data-testid="chronon-job-week-day" data-result=word>
            <Badge appearance=appearance color=color>{day.label}</Badge>
        </span>
    }
}

/// Error banner for the job page's Overview when the newest run failed: what happened and
/// a way to the run. Renders nothing while the job is healthy.
#[component]
pub fn FailingBanner(
    /// Job id.
    job_id: String,
    /// Bumped when live run events arrive for this job.
    #[prop(optional)]
    refresh_signal: Option<Signal<u32>>,
) -> impl IntoView {
    let week = LocalResource::new(move || {
        let id = job_id.clone();
        if let Some(signal) = refresh_signal {
            let _ = signal.get();
        }
        async move { get_job_week(id).await.ok() }
    });
    // Its own boundary: the banner loads after the page, never holding the page back.
    let banner = move || {
        week.get().flatten().and_then(|w| w.failing).map(|failing| {
            let href = chronon_backend::chronon_run_path(&failing.run_id);
            view! {
                <div data-testid="chronon-job-failing-banner">
                    <MessageBar intent=MessageBarIntent::Error>
                        <MessageBarBody>
                            <MessageBarTitle>{failing.title}</MessageBarTitle>
                            {failing.detail}
                        </MessageBarBody>
                        <MessageBarActions>
                            <Link href=href>"See the run"</Link>
                        </MessageBarActions>
                    </MessageBar>
                </div>
            }
        })
    };
    view! { <Suspense fallback=|| ()>{banner}</Suspense> }
}
