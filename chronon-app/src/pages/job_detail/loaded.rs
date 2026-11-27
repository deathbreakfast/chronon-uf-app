//! Loaded job detail body (extracted to keep the page orchestrator small).

use std::collections::HashMap;

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::NavigateOptions;
use orbital::components::{Caption1, SpacingSize};
use orbital::primitives::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Flex, MessageBar,
    MessageBarBody, MessageBarIntent, Tab, TabList,
};
use uf_product::access::{AccessCard, AccessDenied};

use super::actions::{
    make_cancel_callback, make_edit_callback, make_save_callback, open_run_now, JobDetailFormState,
};
use super::components::{
    FailingBanner, JobDetailHeader, JobEditForm, JobInfoCard, JobRecentRuns, PauseDialog,
    PausedBanner, RunNowDialog, RunNowDialogInput, RunStarted, RunStartedNotice, ScheduleCard,
    ScheduleDialog,
};
use super::display::{created_message, param_signals_for_edit};
use crate::live::{
    chronon_run_event_is_status, chronon_run_event_matches_job, ChrononJobRunSubscription,
};
use crate::server::{
    effective_job_pool, get_job, get_job_access, get_job_pause_info, ChrononJobAccess, Job,
    PauseInfo, Script,
};

/// Shared state every job-page section reads.
#[derive(Clone, Copy)]
pub(super) struct JobPageCtx {
    /// Latest copy of the job; replaced after saves and live status events.
    pub job: RwSignal<Job>,
    /// What the signed-in person may do.
    pub access: RwSignal<ChrononJobAccess>,
    /// Success message shown above the Schedule card.
    pub flash: RwSignal<Option<String>>,
    /// Who paused the job, when it's paused.
    pub pause_info: RwSignal<Option<PauseInfo>>,
    /// Reloads `job` (and pause details) from the server.
    pub refresh: Callback<()>,
    /// Opens the Edit schedule dialog.
    pub schedule_dialog_open: RwSignal<bool>,
    /// Opens the Pause or Resume confirmation.
    pub pause_dialog_open: RwSignal<bool>,
    /// Switches Configuration into its edit form.
    pub is_editing: RwSignal<bool>,
}

const TABS: [&str; 4] = ["overview", "runs", "history", "access"];

/// What the denied state says View would let the reader do on this page.
const VIEW_PURPOSE: &str = "see this job's runs, parameters and output";

/// Reactive body for a successfully loaded job.
#[allow(clippy::too_many_lines)]
#[component]
pub(super) fn JobDetailLoaded(
    /// Loaded job DTO.
    job: Job,
    /// Scripts catalog resource (parameter types, run-now params prompt).
    scripts_res: Resource<Result<Vec<Script>, ServerFnError>>,
    /// Live run subscription handle for this page.
    live: ChrononJobRunSubscription,
    /// Bumped when live events arrive for this job.
    runs_refresh_signal: Signal<u32>,
) -> impl IntoView {
    let job_id = job.id.clone();
    let query = use_query_map();
    let created = query.with_untracked(|q| q.get("created").is_some());
    let flash = RwSignal::new(created.then(|| created_message(&job)));
    let job_sig = RwSignal::new(job);
    let access = RwSignal::new(ChrononJobAccess::default());
    let pause_info = RwSignal::new(None::<PauseInfo>);

    let access_id = StoredValue::new(job_id.clone());
    Effect::new(move |_| {
        let id = access_id.get_value();
        leptos::task::spawn_local_scoped(async move {
            if let Ok(a) = get_job_access(Some(id)).await {
                access.set(a);
            }
        });
    });

    // Pause details follow the job's status and revision.
    let pause_key = Memo::new(move |_| job_sig.with(|j| (j.id.clone(), j.status, j.revision)));
    Effect::new(move |_| {
        let (id, _, _) = pause_key.get();
        leptos::task::spawn_local_scoped(async move {
            if let Ok(info) = get_job_pause_info(id).await {
                pause_info.set(info);
            }
        });
    });

    let refresh_id = StoredValue::new(job_id.clone());
    let refresh = Callback::new(move |()| {
        let id = refresh_id.get_value();
        leptos::task::spawn_local_scoped(async move {
            if let Ok(Some(job)) = get_job(id).await {
                job_sig.set(job);
            }
        });
    });

    let is_editing = RwSignal::new(false);
    let ctx = JobPageCtx {
        job: job_sig,
        access,
        flash,
        pause_info,
        refresh,
        schedule_dialog_open: RwSignal::new(false),
        pause_dialog_open: RwSignal::new(false),
        is_editing,
    };

    // Live status events change Last run / Next run.
    let live_store = StoredValue::new(live);
    let job_id_for_status = job_id.clone();
    Effect::new(move |_| {
        let live = live_store.get_value();
        let _ = live.trigger.get();
        if let Some(ev) = live.latest_event.get() {
            if chronon_run_event_matches_job(&ev, &job_id_for_status)
                && chronon_run_event_is_status(&ev)
            {
                refresh.run(());
            }
        }
    });

    // ── Tabs (kept in ?tab=) ────────────────────
    let tab = RwSignal::new(
        query
            .with_untracked(|q| q.get("tab"))
            .filter(|t| TABS.contains(&t.as_str()))
            .unwrap_or_else(|| "overview".to_string()),
    );
    let navigate = use_navigate();
    let location = use_location();
    Effect::new(move |prev: Option<String>| {
        let current = tab.get();
        if prev.as_ref().is_some_and(|p| p != &current) {
            let path = location.pathname.get_untracked();
            navigate(
                &format!("{path}?tab={current}"),
                NavigateOptions {
                    replace: true,
                    scroll: false,
                    ..Default::default()
                },
            );
        }
        current
    });

    // ── Configuration edit form ─────────────────
    let initial = job_sig.get_untracked();
    let form = JobDetailFormState {
        job_name: RwSignal::new(initial.name.clone()),
        pool: RwSignal::new(effective_job_pool(initial.pool.as_deref()).to_string()),
        params: RwSignal::new(HashMap::default()),
        param_errors: RwSignal::new(HashMap::default()),
    };
    let script_params = Memo::new(move |_| {
        let script_name = job_sig.with(|j| j.script_name.clone());
        scripts_res
            .get()
            .and_then(Result::ok)
            .and_then(|scripts| scripts.into_iter().find(|s| s.name == script_name))
            .map(|s| s.params)
            .unwrap_or_default()
    });
    let save_loading = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let on_edit = make_edit_callback(ctx, form, save_error, move || {
        param_signals_for_edit(
            &script_params.get_untracked(),
            &job_sig.get_untracked().params,
        )
    });
    let on_save = make_save_callback(ctx, form, script_params, save_loading, save_error);
    let on_cancel = make_cancel_callback(ctx, save_error);

    // ── Run now ─────────────────────────────────
    let run_now_loading = RwSignal::new(false);
    let run_now_error = RwSignal::new(None::<String>);
    let run_now_dialog_open = RwSignal::new(false);
    let run_started = RwSignal::new(None::<RunStarted>);
    let run_now_disabled = Memo::new(move |_| run_now_loading.get() || is_editing.get());
    let on_run_now = Callback::new(move |_: leptos::ev::MouseEvent| {
        run_now_error.set(None);
        open_run_now(run_now_dialog_open, run_now_loading);
    });

    let job_id_overview = StoredValue::new(job_id.clone());
    let job_id_banner = StoredValue::new(job_id.clone());
    let job_id_for_dialog = job_id.clone();
    let job_id_runs = StoredValue::new(job_id);
    let job_name = Signal::derive(move || job_sig.with(|j| j.name.clone()));
    // Runs, parameters and output need View; the name, owner and schedule never do.
    let can_view = Signal::derive(move || access.with(|a| a.can_view));
    let access_known = Signal::derive(move || access.with(|a| !a.role.is_empty()));
    let resource = StoredValue::new(crate::permissions::job_ref_for(&job_sig.get_untracked()));
    let denied = move || {
        view! {
            <Show when=move || access_known.get()>
                <AccessDenied resource=resource.get_value() action="View" purpose=VIEW_PURPOSE />
            </Show>
        }
    };

    view! {
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            <div data-testid="job-breadcrumb">
                <Breadcrumb>
                    <BreadcrumbItem>
                        <a href=crate::paths::JOBS>
                            <BreadcrumbButton>"Jobs"</BreadcrumbButton>
                        </a>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                    <BreadcrumbItem>
                        <BreadcrumbButton current=true>{job_name}</BreadcrumbButton>
                    </BreadcrumbItem>
                </Breadcrumb>
            </div>

            <JobDetailHeader
                ctx=ctx
                run_now_loading=run_now_loading
                run_now_error=run_now_error
                run_now_disabled=run_now_disabled
                on_run_now=on_run_now
                on_edit=on_edit
            />

            <RunStartedNotice notice=run_started />

            <div data-testid="job-tabs">
                <TabList selected_value=tab>
                    <Tab value="overview">"Overview"</Tab>
                    <Tab value="runs">"Runs"</Tab>
                    <Tab value="history">"History"</Tab>
                    <Tab value="access">"Access"</Tab>
                </TabList>
            </div>

            // Panels stay mounted and are hidden, so switching tabs never reloads them.
            <div data-testid="job-tab-overview" hidden=move || tab.get() != "overview">
                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                    <Show when=move || flash.get().is_some()>
                        <div data-testid="job-flash">
                            <MessageBar intent=MessageBarIntent::Success>
                                <MessageBarBody>{move || flash.get().unwrap_or_default()}</MessageBarBody>
                            </MessageBar>
                        </div>
                    </Show>
                    <Show when=move || can_view.get()>
                        <FailingBanner job_id=job_id_banner.get_value() refresh_signal=runs_refresh_signal />
                    </Show>
                    <PausedBanner ctx=ctx />
                    <ScheduleCard ctx=ctx />
                    <Show when=move || can_view.get() fallback=denied>
                        <div data-testid="job-overview-recent-runs">
                            <Show when=move || tab.get() != "runs">
                                <JobRecentRuns job_id=job_id_overview.get_value() refresh_signal=runs_refresh_signal />
                            </Show>
                        </div>
                        <Show
                            when=move || is_editing.get()
                            fallback=move || view! { <JobInfoCard ctx=ctx script_params=script_params on_edit=on_edit /> }
                        >
                            <JobEditForm
                                form=form
                                ctx=ctx
                                script_params=script_params
                                save_loading=save_loading
                                save_error=save_error
                                on_save=on_save
                                on_cancel=on_cancel
                            />
                        </Show>
                    </Show>
                </Flex>
            </div>
            <div data-testid="job-tab-runs" hidden=move || tab.get() != "runs">
                // Mounted only on this tab, so the page never holds two runs cards.
                <Show when=move || tab.get() == "runs">
                    <Show when=move || can_view.get() fallback=denied>
                        <JobRecentRuns
                            job_id=job_id_runs.get_value()
                            refresh_signal=runs_refresh_signal
                            show_filters=true
                        />
                    </Show>
                </Show>
            </div>
            <div data-testid="job-tab-history" hidden=move || tab.get() != "history">
                <Caption1>"Revision history for this job will appear here."</Caption1>
            </div>
            // Mounted with the page like the other panels, so opening the tab doesn't reload it.
            <div data-testid="job-tab-access" hidden=move || tab.get() != "access">
                <AccessCard resource=resource.get_value() />
            </div>
        </Flex>

        <RunNowDialog
            props=RunNowDialogInput {
                open: run_now_dialog_open,
                job_id: job_id_for_dialog,
                job_name: job_sig.with_untracked(|j| j.name.clone()),
                initial_params: Signal::derive(|| None),
                loading: run_now_loading,
                on_started: Callback::new(move |started| run_started.set(Some(started))),
            }
        />
        <ScheduleDialog ctx=ctx />
        <PauseDialog ctx=ctx />
    }
}
