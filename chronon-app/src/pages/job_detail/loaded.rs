//! Loaded job detail body (extracted to keep the page orchestrator small).

use leptos::prelude::*;

use super::actions::{
    make_cancel_callback, make_edit_callback, make_save_callback, parse_run_now_params_input,
    restore_form_state, run_job_now_with_optional_params, JobDetailDefaults, JobDetailFormState,
};
use super::components::{
    JobDetailHeader, JobDetailHeaderInput, JobEditForm, JobEditFormInput, JobInfoCard,
    JobInfoCardInput, JobRecentRuns, RunNowDialog, RunNowDialogInput,
};
use super::display::{
    display_timezone as format_timezone, normalized_params, pretty_json, snapshot_params,
    snapshot_string,
};
use crate::live::{
    chronon_run_event_is_status, chronon_run_event_matches_job, ChrononJobRunSubscription,
};
use crate::server::{effective_job_pool, get_job, Job, JobRevision, JobStatus, Script};

/// Reactive body for a successfully loaded job.
#[allow(clippy::too_many_lines)]
#[component]
pub(super) fn JobDetailLoaded(
    /// Loaded job DTO.
    job: Job,
    /// Job resource (for refetch after save).
    job_res: Resource<Result<Option<Job>, ServerFnError>>,
    /// Revisions resource.
    revisions_res: Resource<Result<Vec<JobRevision>, ServerFnError>>,
    /// Scripts catalog resource (run-now params prompt).
    scripts_res: Resource<Result<Vec<Script>, ServerFnError>>,
    /// Live run subscription handle for this page.
    live: ChrononJobRunSubscription,
    /// Bumped when live events arrive for this job.
    runs_refresh_signal: Signal<u32>,
) -> impl IntoView {
    let job_name = job.name.clone();
    let job_cron = job.cron.clone();
    let job_script = job.script_name.clone();
    let job_timezone = job.timezone.clone().unwrap_or_default();
    let job_params = normalized_params(&job.params);
    let job_pool = effective_job_pool(job.pool.as_deref()).to_string();
    let job_enabled_init = job.status == JobStatus::Active;
    let defaults = JobDetailDefaults {
        name: job_name.clone(),
        cron: job_cron.clone(),
        timezone: job_timezone.clone(),
        params: job_params.clone(),
        enabled: job_enabled_init,
        pool: job_pool.clone(),
    };
    let defaults_store = StoredValue::new(defaults.clone());
    let job_id_for_state = job.id.clone();
    let job_id_for_run = job.id.clone();
    let job_revision_from_server = job.revision;

    // ── Edit mode state ─────────────────────────
    let is_editing = RwSignal::new(false);

    // ── Form state ──────────────────────────────
    let form = JobDetailFormState {
        job_name: RwSignal::new(job_name.clone()),
        cron: RwSignal::new(job_cron.clone()),
        timezone: RwSignal::new(job_timezone.clone()),
        enabled: RwSignal::new(job_enabled_init),
        params: RwSignal::new(job_params.clone()),
        params_str: RwSignal::new(pretty_json(&job_params)),
        pool: RwSignal::new(job_pool.clone()),
    };
    let form_job_name = form.job_name;
    let form_cron = form.cron;
    let form_timezone = form.timezone;
    let form_enabled = form.enabled;
    let form_params = form.params;
    let form_params_str = form.params_str;
    let form_pool = form.pool;

    // Sync textarea string -> form_params JSON value
    Effect::new(move || {
        let text = form_params_str.get();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            form_params.set(serde_json::json!({}));
        } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            form_params.set(val);
        }
    });

    // ── Revision selector state ─────────────────
    let latest_revision_num = Memo::new(move |_| {
        revisions_res
            .get()
            .and_then(|result| {
                result
                    .ok()
                    .and_then(|revisions| revisions.iter().map(|r| r.revision_number).max())
            })
            .unwrap_or(job_revision_from_server)
    });

    let selected_revision_num = RwSignal::new(job_revision_from_server);
    let revision_str = RwSignal::new(job_revision_from_server.to_string());

    // When revisions refresh, default to the latest revision
    Effect::new(move |_| {
        let latest = latest_revision_num.get();
        selected_revision_num.set(latest);
        revision_str.set(latest.to_string());
    });

    // When user changes revision via Select, sync and handle side effects
    let form_for_revision_restore = form.clone();
    Effect::new(move || {
        let val = revision_str.get();
        if let Ok(num) = val.parse::<u32>() {
            selected_revision_num.set(num);
            let latest = latest_revision_num.get_untracked();
            if num < latest {
                is_editing.set(false);
                defaults_store.with_value(|defaults| {
                    restore_form_state(&form_for_revision_restore, defaults);
                });
            }
        }
    });

    let is_viewing_old_revision = Memo::new(move |_| {
        let latest = latest_revision_num.get();
        selected_revision_num.get() < latest
    });

    // ── Display memos (snapshot or current) ─────
    let selected_snapshot = Memo::new(move |_| {
        let rev_num = selected_revision_num.get();
        revisions_res.get().and_then(|result| {
            result.ok().and_then(|revisions| {
                revisions
                    .into_iter()
                    .find(|r| r.revision_number == rev_num)
                    .map(|r| r.snapshot_json)
            })
        })
    });

    let display_job_name = Memo::new(move |_| {
        if is_editing.get() {
            form_job_name.get()
        } else if let Some(snapshot) = selected_snapshot.get() {
            snapshot_string(&snapshot, "job_name").unwrap_or_else(|| job_name.clone())
        } else {
            job_name.clone()
        }
    });

    let display_script_name = Memo::new(move |_| {
        selected_snapshot.get().map_or_else(
            || job_script.clone(),
            |snapshot| {
                snapshot_string(&snapshot, "script_name").unwrap_or_else(|| job_script.clone())
            },
        )
    });

    // Mirror Memo into RwSignal for Thaw Input binding (readonly field)
    let script_display_signal = RwSignal::new(display_script_name.get_untracked());
    Effect::new(move || {
        script_display_signal.set(display_script_name.get());
    });

    let display_cron = Memo::new(move |_| {
        if is_editing.get() {
            form_cron.get()
        } else if let Some(snapshot) = selected_snapshot.get() {
            snapshot_string(&snapshot, "cron_expr").unwrap_or_else(|| job_cron.clone())
        } else {
            job_cron.clone()
        }
    });

    let display_timezone = Memo::new(move |_| {
        if is_editing.get() {
            format_timezone(&form_timezone.get())
        } else if let Some(snapshot) = selected_snapshot.get() {
            snapshot_string(&snapshot, "timezone").unwrap_or_else(|| "UTC".to_string())
        } else {
            format_timezone(&job_timezone)
        }
    });

    let display_pool = Memo::new(move |_| {
        if is_editing.get() {
            form_pool.get()
        } else if let Some(snapshot) = selected_snapshot.get() {
            effective_job_pool(snapshot_string(&snapshot, "pool").as_deref()).to_string()
        } else {
            job_pool.clone()
        }
    });

    let job_params_for_display = job_params.clone();
    let display_params = Memo::new(move |_| {
        if is_editing.get() {
            form_params.get()
        } else if let Some(snapshot) = selected_snapshot.get() {
            snapshot_params(&snapshot).unwrap_or_else(|| job_params_for_display.clone())
        } else {
            job_params_for_display.clone()
        }
    });

    let last_run = RwSignal::new(
        job.last_run_at
            .clone()
            .unwrap_or_else(|| "Never".to_string()),
    );
    let next_run = RwSignal::new(job.next_run_at.unwrap_or_else(|| "\u{2014}".to_string()));
    let enabled_signal = RwSignal::new(job_enabled_init);

    let live_store = StoredValue::new(live);
    let job_id_for_status = job_id_for_run.clone();
    Effect::new(move |_| {
        let live = live_store.get_value();
        let _ = live.trigger.get();
        if let Some(ev) = live.latest_event.get() {
            if chronon_run_event_matches_job(&ev, &job_id_for_status)
                && chronon_run_event_is_status(&ev)
            {
                let id = job_id_for_status.clone();
                leptos::task::spawn_local_scoped(async move {
                    if let Ok(Some(job)) = get_job(id).await {
                        last_run.set(
                            job.last_run_at
                                .clone()
                                .unwrap_or_else(|| "Never".to_string()),
                        );
                        next_run.set(job.next_run_at.unwrap_or_else(|| "\u{2014}".to_string()));
                    }
                });
            }
        }
    });

    // ── Action state ────────────────────────────
    let run_now_loading = RwSignal::new(false);
    let run_now_error = RwSignal::new(None::<String>);
    let run_now_dialog_open = RwSignal::new(false);
    let run_now_params_str = RwSignal::new(pretty_json(&job_params));

    let script_name_for_lookup = display_script_name.get_untracked();
    let script_has_params = Memo::new(move |_| {
        scripts_res
            .get()
            .and_then(Result::ok)
            .and_then(|scripts| {
                scripts
                    .into_iter()
                    .find(|script| script.name == script_name_for_lookup)
            })
            // Prefer prompting when metadata is unavailable so required
            // params are never silently omitted.
            .is_none_or(|script| !script.params.is_empty())
    });

    let save_loading = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);

    let run_now_disabled = Memo::new(move |_| {
        run_now_loading.get() || is_viewing_old_revision.get() || is_editing.get()
    });

    // ── Action callbacks ────────────────────────
    let job_defaults_for_dialog = job_params;
    let run_job_id_for_click = job_id_for_run.clone();
    let on_run_now = Callback::new(move |_: leptos::ev::MouseEvent| {
        if run_now_loading.get() {
            return;
        }
        run_now_error.set(None);
        if script_has_params.get() {
            run_now_params_str.set(pretty_json(&job_defaults_for_dialog));
            run_now_dialog_open.set(true);
            return;
        }
        run_now_loading.set(true);
        run_job_now_with_optional_params(
            run_job_id_for_click.clone(),
            None,
            run_now_loading,
            run_now_error,
            None,
        );
    });
    let run_job_id_for_submit = job_id_for_run;
    let on_run_now_submit = Callback::new(move |()| {
        if run_now_loading.get() {
            return;
        }
        let params = match parse_run_now_params_input(&run_now_params_str.get()) {
            Ok(params) => params,
            Err(message) => {
                run_now_error.set(Some(message));
                return;
            }
        };
        run_now_error.set(None);
        run_now_loading.set(true);
        run_job_now_with_optional_params(
            run_job_id_for_submit.clone(),
            Some(params),
            run_now_loading,
            run_now_error,
            Some(Callback::new(move |()| run_now_dialog_open.set(false))),
        );
    });
    let on_run_now_cancel = Callback::new(move |()| {
        run_now_dialog_open.set(false);
        run_now_error.set(None);
    });
    let on_edit = make_edit_callback(form.clone(), defaults.clone(), save_error, is_editing);
    let job_refetch = Callback::new(move |()| job_res.refetch());
    let revisions_refetch = Callback::new(move |()| revisions_res.refetch());
    let on_save = make_save_callback(
        job_id_for_state,
        form.clone(),
        save_loading,
        save_error,
        is_editing,
        job_refetch,
        revisions_refetch,
    );
    let on_cancel = make_cancel_callback(form, defaults, save_error, is_editing);

    // ── Build props structs ──────────────────
    let header_props = JobDetailHeaderInput {
        display_job_name,
        job_status: job.status,
        revisions_res,
        latest_revision_num,
        revision_str,
        is_viewing_old_revision,
        is_editing,
        enabled_signal,
        run_now_loading,
        run_now_error,
        run_now_disabled,
        on_run_now,
        save_loading,
        save_error,
        on_save,
        on_cancel,
        on_edit,
    };

    let edit_form_props = JobEditFormInput {
        form_job_name,
        script_display_signal,
        form_cron,
        form_timezone,
        form_params_str,
        form_enabled,
        form_pool,
    };

    let info_card_props = JobInfoCardInput {
        display_script_name,
        display_cron,
        display_timezone,
        display_pool,
        display_params,
        last_run,
        next_run,
    };

    // ── Render sub-components ───────────────────
    let job_id_for_runs = job.id;

    view! {
        <JobDetailHeader props=header_props />
        <RunNowDialog
            props=RunNowDialogInput {
                open: run_now_dialog_open,
                params_str: run_now_params_str,
                loading: run_now_loading,
                error: run_now_error,
                on_submit: on_run_now_submit,
                on_cancel: on_run_now_cancel,
            }
        />

        <Show when=move || is_editing.get() fallback=|| ()>
            <JobEditForm form=edit_form_props />
        </Show>

        <Show when=move || !is_editing.get() fallback=|| ()>
            <JobInfoCard display=info_card_props />
        </Show>

        <Show when=move || !is_editing.get() fallback=|| ()>
            <JobRecentRuns
                job_id=job_id_for_runs.clone()
                refresh_signal=runs_refresh_signal
            />
        </Show>
    }
}
