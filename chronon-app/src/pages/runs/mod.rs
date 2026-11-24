use leptos::prelude::*;
use orbital::components::{Caption1, Card, CardContent, ContentContainer, SpacingSize, Title3};
use orbital::primitives::Flex;

use crate::components::{
    chronon_card_content, chronon_table_page_layout, RunsDataTable, RunsTableChrome,
    RunsTableFilterState, RunsTableScope,
};
use crate::live::use_chronon_poll_tick;
use crate::server::get_runs_caption;

/// Runs list page — every run with its retries folded in, filterable by result, time and job.
#[component]
pub fn ChrononRunsPage() -> impl IntoView {
    let poll_tick = use_chronon_poll_tick();
    let refresh_signal = Signal::derive(move || poll_tick.get());

    let filters = RunsTableFilterState::from_url("24h");
    filters.sync_to_url();

    // The caption follows the time and job filters only; it never touches the table.
    let caption = crate::utils::client_fetch(move || {
        let time = filters.time.get();
        let job = filters.job_id();
        let _ = poll_tick.get();
        async move { get_runs_caption(time, job).await.ok() }
    });

    let (card_content_style, card_content_class) = chronon_card_content();
    let (page_style, page_classes) = chronon_table_page_layout();
    let fill_card_content = format!("{} {}", card_content_class, page_classes.card_content);

    view! {
        <style>{card_content_style}</style>
        <style>{page_style}</style>
        <div id="chronon-runs-page">
        <ContentContainer class=page_classes.page data_testid="chronon-runs-page">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap() class=page_classes.body>
                <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                    <Title3>"Runs"</Title3>
                    <div data-testid="chronon-runs-caption">
                        <Caption1>
                            {move || {
                                caption
                                    .get()
                                    .unwrap_or_else(|| {
                                        "Every scheduled and manual run, with retries folded into the run they retry."
                                            .to_string()
                                    })
                            }}
                        </Caption1>
                    </div>
                </Flex>
                <Card class=page_classes.card>
                    <CardContent class=fill_card_content>
                        <div id="chronon-runs-search">
                            <RunsDataTable
                                scope=RunsTableScope::All
                                chrome=RunsTableChrome {
                                    show_job_column: true,
                                    show_card_header: false,
                                    fill_height: true,
                                    infinite_scroll: false,
                                }
                                refresh_signal=refresh_signal
                                filters=Some(filters)
                            />
                        </div>
                    </CardContent>
                </Card>
            </Flex>
        </ContentContainer>
        </div>
    }
}
