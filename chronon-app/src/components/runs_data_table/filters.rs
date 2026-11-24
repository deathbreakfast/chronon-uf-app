//! Result, time and job filters for the runs table, kept in the URL query.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::NavigateOptions;
use orbital::components::{
    Combobox, ComboboxAppearance, ComboboxBind, ComboboxOption, SpacingSize,
};
use orbital::primitives::{Field, Flex, FlexAlign, FlexWrap, Select};

use crate::server::get_jobs;

/// URL keys for the runs filters.
const RESULT_KEY: &str = "result";
const TIME_KEY: &str = "time";
const JOB_KEY: &str = "job";

/// Filter values for the runs table. Values match the server's filter fields: `result`
/// (`any`, `failed`, `worked`, `running`), `time` (`24h`, `7d`, `30d`, `any`) and `job`
/// (a job id, empty for every job).
#[derive(Clone, Copy)]
pub struct RunsTableFilterState {
    /// Result filter value.
    pub result: RwSignal<String>,
    /// Time filter value.
    pub time: RwSignal<String>,
    /// Selected job ids (at most one).
    pub job: RwSignal<Vec<String>>,
}

impl RunsTableFilterState {
    /// Reads the starting values from the URL once; `default_time` applies when the URL has
    /// no `time`. Later changes are written back with [`Self::sync_to_url`].
    #[must_use]
    pub fn from_url(default_time: &str) -> Self {
        let query = use_query_map().get_untracked();
        Self {
            result: RwSignal::new(query.get(RESULT_KEY).unwrap_or_else(|| "any".into())),
            time: RwSignal::new(
                query
                    .get(TIME_KEY)
                    .unwrap_or_else(|| default_time.to_string()),
            ),
            job: RwSignal::new(query.get(JOB_KEY).into_iter().collect()),
        }
    }

    /// The job id filter, if one is picked.
    #[must_use]
    pub fn job_id(self) -> Option<String> {
        self.job.get().into_iter().find(|j| !j.is_empty())
    }

    /// Writes filter changes into the URL with `replace`, so the page never navigates away
    /// or remounts.
    pub fn sync_to_url(self) {
        let navigate = use_navigate();
        let location = use_location();
        Effect::new(move |prev: Option<()>| {
            let result = self.result.get();
            let time = self.time.get();
            let job = self.job_id();
            if prev.is_none() {
                return;
            }
            let mut params = location.query.get_untracked();
            params.replace(RESULT_KEY, result);
            params.replace(TIME_KEY, time);
            match job {
                Some(job) => params.replace(JOB_KEY, job),
                None => {
                    params.remove(JOB_KEY);
                }
            }
            let href = format!(
                "{}{}",
                location.pathname.get_untracked(),
                params.to_query_string()
            );
            navigate(
                &href,
                NavigateOptions {
                    replace: true,
                    scroll: false,
                    ..Default::default()
                },
            );
        });
    }

    /// Filter rules for the server, in `PageRequest::filter` form.
    #[must_use]
    pub fn rules(self) -> Vec<(&'static str, String)> {
        let mut rules = vec![
            (RESULT_KEY, self.result.get_untracked()),
            (TIME_KEY, self.time.get_untracked()),
        ];
        if let Some(job) = self.job.get_untracked().into_iter().find(|j| !j.is_empty()) {
            rules.push((JOB_KEY, job));
        }
        rules
    }
}

/// Toolbar row above the runs table: result and time Selects, and a job Combobox when the
/// table lists every job's runs.
#[component]
pub fn RunsTableFilterBar(
    /// Filter state the table reads.
    filters: RunsTableFilterState,
    /// Show the job Combobox (all-runs scope).
    show_job: bool,
) -> impl IntoView {
    let job_text = RwSignal::new(String::new());
    view! {
        <div data-testid="chronon-runs-filters">
        <Flex
            gap=SpacingSize::Size120.flex_gap()
            align=FlexAlign::End
            wrap=FlexWrap::Wrap
        >
            <div data-testid="chronon-runs-filter-result">
                <Field label="Result">
                    <Select bind=filters.result>
                        <option value="any">"Any result"</option>
                        <option value="failed">"Failed"</option>
                        <option value="worked">"Worked"</option>
                        <option value="running">"Running"</option>
                    </Select>
                </Field>
            </div>
            <div data-testid="chronon-runs-filter-time">
                <Field label="Time">
                    <Select bind=filters.time>
                        <option value="24h">"Last 24 hours"</option>
                        <option value="7d">"Last 7 days"</option>
                        <option value="30d">"Last 30 days"</option>
                        <option value="any">"Any time"</option>
                    </Select>
                </Field>
            </div>
            {show_job.then(|| view! { <JobFilterCombobox filters=filters job_text=job_text /> })}
        </Flex>
        </div>
    }
}

/// Job picker for the all-runs scope. Owns its job list resource so loading it never
/// touches the table.
#[component]
fn JobFilterCombobox(filters: RunsTableFilterState, job_text: RwSignal<String>) -> impl IntoView {
    let jobs = crate::utils::client_fetch(|| async move { get_jobs().await.ok() });
    view! {
        <div data-testid="chronon-runs-filter-job">
            <Field label="Job">
                <Combobox
                    bind=ComboboxBind::new(job_text, filters.job)
                    appearance=ComboboxAppearance {
                        placeholder: MaybeProp::from("Every job"),
                        clearable: true,
                        ..Default::default()
                    }
                >
                    {move || {
                        jobs.get()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|job| view! { <ComboboxOption text=job.name value=job.id /> })
                            .collect_view()
                    }}
                </Combobox>
            </Field>
        </div>
    }
}
