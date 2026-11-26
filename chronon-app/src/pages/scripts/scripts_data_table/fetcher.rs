use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use leptos::prelude::*;
use orbital::primitives::PageFetcher;
use orbital_data::DataRecord;
use orbital_paging::{Page, PageRequest};

use crate::server::{get_scripts_page, SCRIPTS_PAGE_SIZE};

use super::mapper::script_to_record;

pub const SCRIPTS_TABLE_PAGE_SIZE: u32 = SCRIPTS_PAGE_SIZE;

pub fn build_scripts_fetcher() -> PageFetcher {
    Arc::new(|request: PageRequest| {
        Box::pin(async move {
            let page = get_scripts_page(request).await?;
            Ok(Page {
                items: page.items.into_iter().map(script_to_record).collect(),
                has_more: page.has_more,
                total_count: page.total_count,
                next_request_offset: page.next_request_offset,
                next_cursor: page.next_cursor,
            })
        }) as Pin<Box<dyn Future<Output = Result<Page<DataRecord>, ServerFnError>> + Send>>
    })
}
