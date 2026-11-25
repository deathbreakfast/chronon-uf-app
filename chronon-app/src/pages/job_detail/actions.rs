use std::collections::HashMap;

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;

use crate::pages::job_create::form_state::{build_create_params_by_field, collect_param_values};
use crate::server::{
    run_job_now, update_job, JobStatus, ScriptParam, UpdateJobOutcome, UpdateJobRequest,
};

use super::components::format_conflict;
use super::loaded::JobPageCtx;

#[derive(Clone, Copy)]
pub(super) struct JobDetailFormState {
    pub job_name: RwSignal<String>,
    pub pool: RwSignal<String>,
    pub params: RwSignal<HashMap<String, RwSignal<String>>>,
    pub param_errors: RwSignal<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq)]
struct UpdateFormValues {
    job_name: String,
    cron_expr: String,
    timezone: Option<String>,
    params: serde_json::Value,
    enabled: bool,
    pool: String,
    expected_revision: u32,
}

fn build_update_payload_from_values(values: UpdateFormValues) -> UpdateJobRequest {
    UpdateJobRequest {
        job_name: values.job_name,
        cron_expr: Some(values.cron_expr).filter(|c| !c.trim().is_empty()),
        timezone: values.timezone.filter(|t| !t.is_empty()),
        params: chronon_backend::normalized_params(values.params),
        enabled: values.enabled,
        pool: Some(values.pool),
        expected_revision: Some(values.expected_revision),
    }
}

/// Opens the Run now dialog. Every job gets the dialog, with or without parameters, so the
/// person sees what will run before it runs.
pub(super) fn open_run_now(open: RwSignal<bool>, loading: RwSignal<bool>) {
    if !loading.get_untracked() {
        open.set(true);
    }
}

/// Queues a run with the dialog's values, then hands the new run id to `on_started`.
pub(super) fn submit_run_now(
    job_id: String,
    params: serde_json::Value,
    loading: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_started: Callback<String>,
) {
    spawn_local_scoped(async move {
        match run_job_now(job_id, Some(params)).await {
            Ok(run_id) => {
                loading.set(false);
                on_started.run(run_id);
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                loading.set(false);
            }
        }
    });
}

pub(super) fn make_edit_callback(
    ctx: JobPageCtx,
    form: JobDetailFormState,
    save_error: RwSignal<Option<String>>,
    seed_params: impl Fn() -> HashMap<String, RwSignal<String>> + Send + Sync + 'static,
) -> Callback<()> {
    Callback::new(move |()| {
        if ctx.is_editing.get() {
            return;
        }
        let job = ctx.job.get_untracked();
        save_error.set(None);
        form.job_name.set(job.name.clone());
        form.pool
            .set(crate::server::effective_job_pool(job.pool.as_deref()).to_string());
        form.params.set(seed_params());
        form.param_errors.set(HashMap::new());
        ctx.is_editing.set(true);
    })
}

pub(super) fn make_save_callback(
    ctx: JobPageCtx,
    form: JobDetailFormState,
    script_params: Memo<Vec<ScriptParam>>,
    save_loading: RwSignal<bool>,
    save_error: RwSignal<Option<String>>,
) -> Callback<ev::MouseEvent> {
    Callback::new(move |_: ev::MouseEvent| {
        if save_loading.get() {
            return;
        }
        save_error.set(None);
        let values = collect_param_values(&form.params.get());
        let params = match build_create_params_by_field(&script_params.get(), &values) {
            Ok(params) => {
                form.param_errors.set(HashMap::new());
                params
            }
            Err(errors) => {
                form.param_errors.set(errors);
                save_error.set(Some("Fix the marked parameters before saving.".to_string()));
                return;
            }
        };
        let job = ctx.job.get_untracked();
        let payload = build_update_payload_from_values(UpdateFormValues {
            job_name: form.job_name.get(),
            cron_expr: job.cron.clone(),
            timezone: job.timezone.clone(),
            params,
            enabled: job.status == JobStatus::Active,
            pool: form.pool.get(),
            expected_revision: job.revision,
        });
        save_loading.set(true);
        spawn_local_scoped(async move {
            match update_job(job.id.clone(), payload).await {
                Ok(UpdateJobOutcome::Saved { revision, .. }) => {
                    ctx.flash
                        .set(Some(format!("Configuration saved as revision {revision}.")));
                    ctx.refresh.run(());
                    ctx.is_editing.set(false);
                }
                Ok(conflict @ UpdateJobOutcome::Conflict { .. }) => {
                    save_error.set(Some(format_conflict(&conflict)));
                }
                Err(e) => save_error.set(Some(e.to_string())),
            }
            save_loading.set(false);
        });
    })
}

pub(super) fn make_cancel_callback(
    ctx: JobPageCtx,
    save_error: RwSignal<Option<String>>,
) -> Callback<ev::MouseEvent> {
    Callback::new(move |_: ev::MouseEvent| {
        save_error.set(None);
        ctx.is_editing.set(false);
    })
}

#[cfg(test)]
mod tests {
    use super::{build_update_payload_from_values, UpdateFormValues};

    #[test]
    fn build_update_payload_normalizes_empty_timezone_and_cron() {
        let payload = build_update_payload_from_values(UpdateFormValues {
            job_name: "job-a".to_string(),
            cron_expr: String::new(),
            timezone: Some(String::new()),
            params: serde_json::Value::Null,
            enabled: true,
            pool: "chronon-mve-a".to_string(),
            expected_revision: 3,
        });

        assert_eq!(payload.job_name, "job-a");
        assert_eq!(payload.cron_expr, None);
        assert_eq!(payload.timezone, None);
        assert_eq!(payload.params, serde_json::json!({}));
        assert!(payload.enabled);
        assert_eq!(payload.pool.as_deref(), Some("chronon-mve-a"));
        assert_eq!(payload.expected_revision, Some(3));
    }
}
