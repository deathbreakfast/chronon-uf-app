use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use leptos::prelude::*;
use orbital::primitives::PageFetcher;
use orbital_data::DataRecord;
use orbital_paging::{Page, PageRequest};

use crate::server::{get_jobs_page, JOBS_PAGE_SIZE};

use super::mapper::job_to_record;

pub const JOBS_TABLE_PAGE_SIZE: u32 = JOBS_PAGE_SIZE;

pub fn build_jobs_fetcher() -> PageFetcher {
    Arc::new(|request: PageRequest| {
        Box::pin(async move {
            let page = get_jobs_page(request).await?;
            Ok(Page {
                items: page.items.into_iter().map(job_to_record).collect(),
                has_more: page.has_more,
                total_count: page.total_count,
                next_request_offset: page.next_request_offset,
                next_cursor: page.next_cursor,
            })
        }) as Pin<Box<dyn Future<Output = Result<Page<DataRecord>, ServerFnError>> + Send>>
    })
}
