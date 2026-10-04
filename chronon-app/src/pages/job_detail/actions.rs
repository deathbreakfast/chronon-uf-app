use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;

use crate::server::{run_job_now, update_job, UpdateJobRequest};

use super::display::{normalized_params, pretty_json};

#[derive(Clone)]
pub(super) struct JobDetailFormState {
    pub job_name: RwSignal<String>,
    pub cron: RwSignal<String>,
    pub timezone: RwSignal<String>,
    pub enabled: RwSignal<bool>,
    pub params: RwSignal<serde_json::Value>,
    pub params_str: RwSignal<String>,
    pub pool: RwSignal<String>,
}

#[derive(Clone)]
pub(super) struct JobDetailDefaults {
    pub name: String,
    pub cron: String,
    pub timezone: String,
    pub params: serde_json::Value,
    pub enabled: bool,
    pub pool: String,
}

pub(super) fn restore_form_state(form: &JobDetailFormState, defaults: &JobDetailDefaults) {
    form.job_name.set(defaults.name.clone());
    form.cron.set(defaults.cron.clone());
    form.timezone.set(defaults.timezone.clone());
    form.params.set(defaults.params.clone());
    form.params_str.set(pretty_json(&defaults.params));
    form.enabled.set(defaults.enabled);
    form.pool.set(defaults.pool.clone());
}

#[derive(Clone, Debug, PartialEq)]
struct UpdateFormValues {
    job_name: String,
    cron_expr: String,
    timezone: String,
    params: serde_json::Value,
    enabled: bool,
    pool: String,
}

fn build_update_payload_from_values(values: UpdateFormValues) -> UpdateJobRequest {
    let params = normalized_params(&values.params);
    UpdateJobRequest {
        job_name: values.job_name,
        cron_expr: Some(values.cron_expr),
        timezone: if values.timezone.is_empty() {
            None
        } else {
            Some(values.timezone)
        },
        params,
        enabled: values.enabled,
        pool: Some(values.pool),
    }
}

fn build_update_payload(form: &JobDetailFormState) -> UpdateJobRequest {
    build_update_payload_from_values(UpdateFormValues {
        job_name: form.job_name.get(),
        cron_expr: form.cron.get(),
        timezone: form.timezone.get(),
        params: form.params.get(),
        enabled: form.enabled.get(),
        pool: form.pool.get(),
    })
}

pub(super) fn parse_run_now_params_input(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    let parsed = if trimmed.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .map_err(|_| "Parameters must be valid JSON.".to_string())?
    };
    if !parsed.is_object() {
        return Err("Parameters must be a JSON object (for example, {}).".to_string());
    }
    Ok(parsed)
}

pub(super) fn run_job_now_with_optional_params(
    job_id: String,
    params: Option<serde_json::Value>,
    run_now_loading: RwSignal<bool>,
    run_now_error: RwSignal<Option<String>>,
    on_success: Option<Callback<()>>,
) {
    let loading = run_now_loading;
    let error = run_now_error;
    spawn_local_scoped(async move {
        match run_job_now(job_id, params).await {
            Ok(_run_id) => {
                if let Some(cb) = on_success {
                    cb.run(());
                }
                loading.set(false);
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                loading.set(false);
            }
        }
    });
}

pub(super) fn make_edit_callback(
    form: JobDetailFormState,
    defaults: JobDetailDefaults,
    save_error: RwSignal<Option<String>>,
    is_editing: RwSignal<bool>,
) -> Callback<ev::MouseEvent> {
    Callback::new(move |_: ev::MouseEvent| {
        if !is_editing.get() {
            save_error.set(None);
            restore_form_state(&form, &defaults);
            is_editing.set(true);
        }
    })
}

pub(super) fn make_save_callback(
    job_id: String,
    form: JobDetailFormState,
    save_loading: RwSignal<bool>,
    save_error: RwSignal<Option<String>>,
    is_editing: RwSignal<bool>,
    job_refetch: Callback<()>,
    revisions_refetch: Callback<()>,
) -> Callback<ev::MouseEvent> {
    Callback::new(move |_: ev::MouseEvent| {
        if save_loading.get() {
            return;
        }
        save_loading.set(true);
        save_error.set(None);
        let payload = build_update_payload(&form);

        let loading = save_loading;
        let err_signal = save_error;
        let editing = is_editing;
        let job_refetch = job_refetch;
        let revisions_refetch = revisions_refetch;
        let update_job_id = job_id.clone();

        spawn_local_scoped(async move {
            match update_job(update_job_id, payload).await {
                Ok(()) => {
                    job_refetch.run(());
                    revisions_refetch.run(());
                    editing.set(false);
                }
                Err(e) => err_signal.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    })
}

pub(super) fn make_cancel_callback(
    form: JobDetailFormState,
    defaults: JobDetailDefaults,
    save_error: RwSignal<Option<String>>,
    is_editing: RwSignal<bool>,
) -> Callback<ev::MouseEvent> {
    Callback::new(move |_: ev::MouseEvent| {
        save_error.set(None);
        is_editing.set(false);
        restore_form_state(&form, &defaults);
    })
}

#[cfg(test)]
mod tests {
    use super::{build_update_payload_from_values, parse_run_now_params_input, UpdateFormValues};

    #[test]
    fn build_update_payload_normalizes_empty_timezone() {
        let payload = build_update_payload_from_values(UpdateFormValues {
            job_name: "job-a".to_string(),
            cron_expr: "0 * * * *".to_string(),
            timezone: String::new(),
            params: serde_json::Value::Null,
            enabled: true,
            pool: "chronon-mve-a".to_string(),
        });

        assert_eq!(payload.job_name, "job-a");
        assert_eq!(payload.cron_expr.as_deref(), Some("0 * * * *"));
        assert_eq!(payload.timezone, None);
        assert_eq!(payload.params, serde_json::json!({}));
        assert!(payload.enabled);
        assert_eq!(payload.pool.as_deref(), Some("chronon-mve-a"));
    }

    #[test]
    fn parse_run_now_params_accepts_empty_input_as_object() {
        let parsed = parse_run_now_params_input("   ").expect("empty input should normalize");
        assert_eq!(parsed, serde_json::json!({}));
    }

    #[test]
    fn parse_run_now_params_rejects_non_object_json() {
        let err = parse_run_now_params_input("[]").expect_err("array payload should fail");
        assert!(err.contains("JSON object"));
    }
}
