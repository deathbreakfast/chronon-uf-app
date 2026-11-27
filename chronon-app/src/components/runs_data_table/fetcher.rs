use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use leptos::prelude::*;
use orbital::primitives::PageFetcher;
use orbital_data::{DataRecord, DataValue};
use orbital_paging::{FilterLogicWire, FilterQuery, FilterRuleParam, Page, PageRequest};

use crate::server::{get_job_runs_page, get_runs_page, RUNS_PAGE_SIZE};

use super::filters::RunsTableFilterState;
use super::mapper::run_to_record;
use super::RunsTableScope;

pub const RUNS_TABLE_PAGE_SIZE: u32 = RUNS_PAGE_SIZE;

/// Adds the toolbar's result/time/job rules to the table's request.
fn with_filter_rules(
    mut request: PageRequest,
    filters: Option<RunsTableFilterState>,
) -> PageRequest {
    let Some(filters) = filters else {
        return request;
    };
    let mut query = request.filter.take().unwrap_or(FilterQuery {
        items: Vec::new(),
        logic: FilterLogicWire::And,
    });
    for (field, value) in filters.rules() {
        query.items.push(FilterRuleParam {
            field: field.to_string(),
            operator: "is".to_string(),
            value: DataValue::Text(value),
        });
    }
    request.filter = Some(query);
    request
}

/// Page fetcher for the runs table. Filter values are read when a page loads, so changing
/// them refetches through the table's refresh signal instead of rebuilding the table.
pub fn build_runs_fetcher(
    scope: RunsTableScope,
    filters: Option<RunsTableFilterState>,
) -> PageFetcher {
    Arc::new(move |request: PageRequest| {
        let scope = scope.clone();
        let request = with_filter_rules(request, filters);
        Box::pin(async move {
            let page = match scope {
                RunsTableScope::All => get_runs_page(request).await?,
                RunsTableScope::ForJob(job_id) => get_job_runs_page(job_id, request).await?,
            };
            Ok(Page {
                items: page.items.into_iter().map(run_to_record).collect(),
                has_more: page.has_more,
                total_count: page.total_count,
                next_request_offset: page.next_request_offset,
                next_cursor: page.next_cursor,
            })
        }) as Pin<Box<dyn Future<Output = Result<Page<DataRecord>, ServerFnError>> + Send>>
    })
}
