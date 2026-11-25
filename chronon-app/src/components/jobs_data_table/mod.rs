mod columns;
mod fetcher;
mod mapper;

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use orbital::components::{EmptyState, EmptyStateCallToAction, MessageBar, MessageBarIntent};
use orbital::primitives::{Button, ButtonAppearance, Flex};
use orbital::primitives::{
    DataTable, DataTableEmptyView, DataTableEvents, DataTableFeatures, DataTableHeaderChromeConfig,
    DataTableNoResultsView, DataTableSource, DataTableToolbarConfig, PagingMode,
};
use turf::inline_style_sheet_values;

use columns::jobs_table_columns;
use fetcher::{build_jobs_fetcher, JOBS_TABLE_PAGE_SIZE};

/// Jobs list DataTable with search, filters, and table layout.
#[component]
pub fn JobsDataTable(
    /// Whether the empty state offers Create job.
    #[prop(into)]
    can_create: Signal<bool>,
) -> impl IntoView {
    let navigate = use_navigate();

    let on_row_click = Callback::new(move |(id,): (String,)| {
        navigate(
            &chronon_backend::chronon_job_path(&id),
            NavigateOptions::default(),
        );
    });

    let data_source = DataTableSource::Server {
        fetcher: build_jobs_fetcher(),
        page_size: JOBS_TABLE_PAGE_SIZE,
    };

    let (style_sheet, class_names) = inline_style_sheet_values! {
        .JobsTableSection {
            width: 100%;
        }
    };

    view! {
        <style>{style_sheet}</style>
        <div class=class_names.jobs_table_section id="chronon-jobs-data-table" data-testid="chronon-jobs-data-table">
            <DataTable
                data_source=data_source
                paging=PagingMode::Paged
                flex=true
                features=DataTableFeatures::MULTI_FILTER
                columns=jobs_table_columns()
                sortable=false
                toolbar_config=DataTableToolbarConfig {
                    quick_search: true,
                    filter_panel: true,
                    column_picker: false,
                    pivot: false,
                    export_menu: false,
                }
                header_chrome=DataTableHeaderChromeConfig {
                    column_menu: false,
                    column_filter_button: false,
                    column_hide: false,
                    show_table_grid: true,
                }
                events=DataTableEvents {
                    on_row_click: Some(on_row_click),
                    ..Default::default()
                }
            >
                <DataTableEmptyView slot>
                    <div data-testid="jobs-empty-state">
                        <EmptyState
                            message="No jobs yet"
                            description="A job runs one of your scripts on a schedule, once at a set time, or only when someone uses Run now."
                            icon=icondata::AiClockCircleOutlined
                        >
                            <EmptyStateCallToAction slot:call_to_action>
                                <Flex gap=orbital::components::SpacingSize::Size120.flex_gap()>
                                    <Show when=move || can_create.get()>
                                        <A href=crate::paths::JOBS_NEW>
                                            <Button appearance=ButtonAppearance::Primary>"Create job"</Button>
                                        </A>
                                    </Show>
                                    <A href=crate::paths::SCRIPTS>
                                        <Button appearance=ButtonAppearance::Secondary>"See scripts"</Button>
                                    </A>
                                </Flex>
                            </EmptyStateCallToAction>
                        </EmptyState>
                    </div>
                </DataTableEmptyView>
                <DataTableNoResultsView slot>
                    <MessageBar intent=MessageBarIntent::Info>
                        "No jobs match your search or filters."
                    </MessageBar>
                </DataTableNoResultsView>
            </DataTable>
        </div>
    }
}
