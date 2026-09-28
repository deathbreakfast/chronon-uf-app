mod columns;
mod fetcher;
mod mapper;

use leptos::prelude::*;
use orbital::components::MessageBar;
use orbital::primitives::{
    DataTable, DataTableEmptyView, DataTableFeatures, DataTableHeaderChromeConfig,
    DataTableNoResultsView, DataTableSource, DataTableToolbarConfig, ListViewConfig,
    MessageBarIntent, PagingMode,
};

use columns::scripts_table_columns;
use fetcher::{build_scripts_fetcher, SCRIPTS_TABLE_PAGE_SIZE};

/// Scripts registry DataTable with list view.
#[component]
pub fn ScriptsDataTable(
    /// When true, the table grows to fill a flex parent.
    #[prop(default = false)]
    fill_height: bool,
) -> impl IntoView {
    let data_source = DataTableSource::Server {
        fetcher: build_scripts_fetcher(),
        page_size: SCRIPTS_TABLE_PAGE_SIZE,
    };

    view! {
        <div id="chronon-scripts-data-table" data-testid="chronon-scripts-data-table">
            <DataTable
                data_source=data_source
                paging=PagingMode::Paged
                flex=fill_height
                features=DataTableFeatures::LIST_VIEW | DataTableFeatures::MULTI_FILTER
                list_view=ListViewConfig::new("name")
                    .with_secondary_fields(vec![
                        "signature".into(),
                        "description".into(),
                        "params_summary".into(),
                    ])
                columns=scripts_table_columns()
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
            >
                <DataTableEmptyView slot>
                    <MessageBar intent=MessageBarIntent::Info>
                        "No scripts registered. Add scripts using the #[chronon_coordinator_macros::script] macro."
                    </MessageBar>
                </DataTableEmptyView>
                <DataTableNoResultsView slot>
                    <MessageBar intent=MessageBarIntent::Info>
                        "No scripts match your search or filters."
                    </MessageBar>
                </DataTableNoResultsView>
            </DataTable>
        </div>
    }
}
