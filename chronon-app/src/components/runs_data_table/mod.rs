mod columns;
mod fetcher;
mod mapper;

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use orbital::components::{
    Card, CardContent, CardHeader, CardSectionBorder, MessageBar, MessageBarIntent, Subtitle2,
};
use orbital::primitives::{
    DataTable, DataTableEmptyView, DataTableEvents, DataTableFeatures, DataTableHeaderChromeConfig,
    DataTableNoResultsView, DataTableSource, DataTableToolbarConfig, PagingMode,
};
use turf::inline_style_sheet_values;

use crate::components::chronon_card_content;
use columns::runs_table_columns;
use fetcher::{build_runs_fetcher, RUNS_TABLE_PAGE_SIZE};

/// Scope for the shared runs `DataTable`.
#[derive(Clone)]
pub enum RunsTableScope {
    All,
    ForJob(String),
}

/// Layout / chrome flags for [`RunsDataTable`] (avoids a long boolean prop list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct RunsTableChrome {
    /// Whether to show the job column.
    pub show_job_column: bool,
    /// Whether to wrap the table in a card header.
    pub show_card_header: bool,
    /// When true, the table grows to fill a flex parent (list pages).
    pub fill_height: bool,
    /// When true, use infinite scroll (embedded recent-runs card).
    pub infinite_scroll: bool,
}

impl Default for RunsTableChrome {
    fn default() -> Self {
        Self {
            show_job_column: true,
            show_card_header: true,
            fill_height: false,
            infinite_scroll: false,
        }
    }
}

/// Shared runs DataTable with search, filters, and table layout.
#[component]
pub fn RunsDataTable(
    /// Scope to filter by.
    scope: RunsTableScope,
    /// Column / chrome flags. Defaults to job column + card header.
    #[prop(optional)]
    chrome: Option<RunsTableChrome>,
    /// Optional max height.
    #[prop(optional)]
    max_height: Option<f64>,
    /// Page size.
    #[prop(default = RUNS_TABLE_PAGE_SIZE)]
    page_size: u32,
    /// Optional card title.
    #[prop(optional)]
    card_title: Option<String>,
    /// External refresh nonce — bumps table fetch without remounting.
    #[prop(optional)]
    refresh_signal: Option<Signal<u32>>,
) -> impl IntoView {
    let chrome = chrome.unwrap_or_default();
    let navigate = use_navigate();

    let on_row_click = Callback::new(move |(id,): (String,)| {
        navigate(
            &chronon_backend::chronon_run_path(&id),
            NavigateOptions::default(),
        );
    });

    let data_source = DataTableSource::Server {
        fetcher: build_runs_fetcher(scope),
        page_size,
    };

    let (card_content_style, card_content_class) = chronon_card_content();
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .RunsTableSection {
            width: 100%;
        }
        .RunsTableCard {
            width: 100%;
            margin-top: var(--spacingVerticalL, 24px);
        }
    };

    let bounded_max_height = max_height.unwrap_or(600.0);
    let paging = if chrome.infinite_scroll {
        PagingMode::InfiniteScroll
    } else {
        PagingMode::Paged
    };
    let table_refresh = refresh_signal.unwrap_or_else(|| Signal::derive(|| 0u32));

    let table = view! {
        <RunsFilledDataTable
            data_source=data_source
            paging=paging
            fill_height=chrome.fill_height
            bounded_max_height=bounded_max_height
            show_job_column=chrome.show_job_column
            on_row_click=on_row_click
            table_refresh=table_refresh
            card_content_style=card_content_style.to_string()
            style_sheet=style_sheet.to_string()
            section_class=class_names.runs_table_section.to_string()
        />
    };

    if chrome.show_card_header {
        let title = card_title.unwrap_or_else(|| "Recent Runs".to_string());
        view! {
            <div id="chronon-job-detail-recent-runs">
            <Card class=class_names.runs_table_card attr:data-testid="chronon-recent-runs-card">
                <CardHeader>
                    <Subtitle2>{title}</Subtitle2>
                </CardHeader>
                <CardSectionBorder />
                <CardContent class=card_content_class.clone()>
                    {table}
                </CardContent>
            </Card>
            </div>
        }
        .into_any()
    } else {
        view! { {table} }.into_any()
    }
}

/// Inner DataTable body shared by flex-fill and max-height layouts.
#[component]
fn RunsFilledDataTable(
    data_source: DataTableSource,
    paging: PagingMode,
    fill_height: bool,
    bounded_max_height: f64,
    show_job_column: bool,
    on_row_click: Callback<(String,)>,
    table_refresh: Signal<u32>,
    card_content_style: String,
    style_sheet: String,
    section_class: String,
) -> impl IntoView {
    let columns = runs_table_columns(show_job_column);
    let events = DataTableEvents {
        on_row_click: Some(on_row_click),
        ..Default::default()
    };
    let toolbar = DataTableToolbarConfig {
        quick_search: true,
        filter_panel: true,
        column_picker: false,
        pivot: false,
        export_menu: false,
    };
    let header_chrome = DataTableHeaderChromeConfig {
        column_menu: false,
        column_filter_button: false,
        column_hide: false,
        show_table_grid: true,
    };

    if fill_height {
        view! {
            <style>{card_content_style}</style>
            <style>{style_sheet}</style>
            <div class=section_class id="chronon-runs-data-table" data-testid="chronon-runs-data-table">
                <DataTable
                    data_source=data_source
                    paging=paging
                    flex=true
                    features=DataTableFeatures::MULTI_FILTER
                    columns=columns
                    sortable=false
                    toolbar_config=toolbar
                    header_chrome=header_chrome
                    events=events
                    refresh_signal=table_refresh
                >
                    <DataTableEmptyView slot>
                        <MessageBar intent=MessageBarIntent::Info>
                            "No runs recorded yet."
                        </MessageBar>
                    </DataTableEmptyView>
                    <DataTableNoResultsView slot>
                        <MessageBar intent=MessageBarIntent::Info>
                            "No runs match your search or filters."
                        </MessageBar>
                    </DataTableNoResultsView>
                </DataTable>
            </div>
        }
        .into_any()
    } else {
        view! {
            <style>{card_content_style}</style>
            <style>{style_sheet}</style>
            <div class=section_class id="chronon-runs-data-table" data-testid="chronon-runs-data-table">
                <DataTable
                    data_source=data_source
                    paging=paging
                    max_height=bounded_max_height
                    features=DataTableFeatures::MULTI_FILTER
                    columns=columns
                    sortable=false
                    toolbar_config=toolbar
                    header_chrome=header_chrome
                    events=events
                    refresh_signal=table_refresh
                >
                    <DataTableEmptyView slot>
                        <MessageBar intent=MessageBarIntent::Info>
                            "No runs recorded yet."
                        </MessageBar>
                    </DataTableEmptyView>
                    <DataTableNoResultsView slot>
                        <MessageBar intent=MessageBarIntent::Info>
                            "No runs match your search or filters."
                        </MessageBar>
                    </DataTableNoResultsView>
                </DataTable>
            </div>
        }
        .into_any()
    }
}
