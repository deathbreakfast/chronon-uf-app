pub mod components;
pub mod form_state;

use std::collections::HashMap;

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use orbital::components::{ContentContainer, SpacingSize, Title3};
use orbital::primitives::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Flex, MessageBar,
    MessageBarBody, MessageBarIntent, MessageBarTitle,
};

use crate::server::{
    create_job, get_scripts, CreateJobRequest, CreateJobScheduleType, ScheduleField,
    SchedulePreview, Script, DEFAULT_POOL,
};
use components::{
    ActionsSection, AdvancedOptionsSection, BasicInfoSection, ParametersSection, RunOnceFields,
    ScheduleSection,
};
use form_state::{
    build_create_params_by_field, collect_param_values, errors_summary, run_once_instant,
    seed_param_signals, FieldErrors,
};

/// Label for a field-error key in the summary at the top of the form.
fn field_label(key: &str) -> String {
    match key {
        "job_name" => "Job name".to_string(),
        "script" => "Script".to_string(),
        "cron" => "Cron".to_string(),
        "timezone" => "Timezone".to_string(),
        "run_once" => "Date and time".to_string(),
        other => other.strip_prefix("param:").unwrap_or(other).to_string(),
    }
}

/// Schedule type for job creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScheduleType {
    /// Recurring schedule driven by a cron expression.
    #[default]
    Cron,
    /// One-shot execution at a specific time.
    RunOnce,
    /// No automatic schedule; the job only runs when triggered manually.
    Manual,
}

/// Job creation page
#[allow(clippy::too_many_lines)]
#[component]
pub fn ChrononJobCreatePage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_store = StoredValue::new(navigate);
    let scripts_res = Resource::new(|| (), |()| async move { get_scripts().await });

    // Form state
    let job_name = RwSignal::new(String::new());
    let selected_script = RwSignal::new(String::new());
    let schedule_type = RwSignal::new(ScheduleType::Cron);
    let cron_expression = RwSignal::new(String::from("0 3 * * *"));
    let timezone = RwSignal::new(String::from("UTC"));
    let preview = RwSignal::new(SchedulePreview::default());
    let run_once = RunOnceFields {
        date: RwSignal::new(String::new()),
        time: RwSignal::new(String::from("02:00")),
        timezone: RwSignal::new(String::from("UTC")),
    };
    let param_signals = RwSignal::new(HashMap::<String, RwSignal<String>>::new());
    let param_errors = RwSignal::new(HashMap::<String, String>::new());
    let errors = RwSignal::new(FieldErrors::new());

    // Advanced options (use String for input binding)
    let show_advanced = RwSignal::new(false);
    let concurrency = RwSignal::new(String::from("1"));
    let timeout_seconds = RwSignal::new(String::from("300"));
    let max_retries = RwSignal::new(String::from("3"));
    let pool = RwSignal::new(String::from(DEFAULT_POOL));
    let create_loading = RwSignal::new(false);
    let create_error = RwSignal::new(None::<String>);

    // Seed param defaults when the selected script changes.
    Effect::new(move || {
        let script_name = selected_script.get();
        let scripts = scripts_res.get().and_then(Result::ok).unwrap_or_default();
        if script_name.is_empty() {
            param_signals.set(HashMap::new());
            return;
        }
        if let Some(script) = scripts.iter().find(|s| s.name == script_name) {
            param_signals.set(seed_param_signals(&script.params));
        }
    });

    let scripts = Memo::new(move |_| scripts_res.get().and_then(Result::ok).unwrap_or_default());

    // Action callbacks
    let on_cancel = Callback::new(move |_: ev::MouseEvent| {
        navigate_store.with_value(|navigate| {
            navigate(crate::paths::JOBS, NavigateOptions::default());
        });
    });
    let on_create = Callback::new(move |_: ev::MouseEvent| {
        if create_loading.get() {
            return;
        }
        create_error.set(None);

        // Check every field at once so each error sits on its field.
        let mut found = FieldErrors::new();
        if let Err(e) = chronon_backend::validate_job_name(&job_name.get()) {
            let message = if job_name.get().trim().is_empty() {
                "Enter a job name.".to_string()
            } else {
                e.to_string()
            };
            found.push(("job_name".into(), message));
        }
        let scripts: Vec<Script> = scripts_res.get().and_then(Result::ok).unwrap_or_default();
        let script = scripts
            .iter()
            .find(|s| s.name == selected_script.get())
            .cloned();
        if script.is_none() {
            found.push(("script".into(), "Choose a script.".into()));
        }
        let mut run_once_at = None;
        match schedule_type.get() {
            ScheduleType::Cron => {
                if let Err(e) =
                    chronon_backend::validate_cron(&cron_expression.get(), &timezone.get())
                {
                    let key = if e.field == ScheduleField::Timezone {
                        "timezone"
                    } else {
                        "cron"
                    };
                    found.push((key.into(), e.message));
                }
            }
            ScheduleType::RunOnce => {
                match run_once_instant(
                    &run_once.date.get(),
                    &run_once.time.get(),
                    &run_once.timezone.get(),
                ) {
                    None => found.push(("run_once".into(), "Pick a date and a time.".into())),
                    Some(at) if at <= chrono::Utc::now() => {
                        found.push(("run_once".into(), "Pick a time in the future.".into()));
                    }
                    Some(at) => run_once_at = Some(at.to_rfc3339()),
                }
            }
            ScheduleType::Manual => {}
        }
        let values = collect_param_values(&param_signals.get());
        let params = match script
            .as_ref()
            .map(|s| build_create_params_by_field(&s.params, &values))
        {
            Some(Ok(params)) => {
                param_errors.set(HashMap::new());
                params
            }
            Some(Err(by_field)) => {
                // Keep the script's parameter order in the summary.
                for p in script
                    .as_ref()
                    .map(|s| s.params.clone())
                    .unwrap_or_default()
                {
                    if let Some(message) = by_field.get(&p.name) {
                        found.push((format!("param:{}", p.name), message.clone()));
                    }
                }
                param_errors.set(by_field);
                serde_json::json!({})
            }
            None => serde_json::json!({}),
        };
        errors.set(found.clone());
        if !found.is_empty() {
            return;
        }

        create_loading.set(true);
        let is_cron = matches!(schedule_type.get(), ScheduleType::Cron);
        let payload = CreateJobRequest {
            job_name: job_name.get().trim().to_string(),
            script_name: selected_script.get(),
            schedule_type: match schedule_type.get() {
                ScheduleType::Cron => CreateJobScheduleType::Cron,
                ScheduleType::RunOnce => CreateJobScheduleType::RunOnce,
                ScheduleType::Manual => CreateJobScheduleType::Manual,
            },
            cron_expr: is_cron.then(|| cron_expression.get()),
            timezone: is_cron
                .then(|| timezone.get())
                .filter(|tz| !tz.trim().is_empty()),
            run_once_at,
            params,
            concurrency: concurrency.get().parse::<u32>().unwrap_or(1),
            timeout_seconds: timeout_seconds.get().parse::<u32>().unwrap_or(300),
            max_retries: max_retries.get().parse::<u32>().unwrap_or(3),
            pool: Some(pool.get()),
        };

        leptos::task::spawn_local_scoped(async move {
            match create_job(payload).await {
                Ok(id) => navigate_store.with_value(|navigate| {
                    navigate(
                        &format!("{}?created=1", chronon_backend::chronon_job_path(&id)),
                        NavigateOptions::default(),
                    );
                }),
                Err(err) => create_error.set(Some(err.to_string())),
            }
            create_loading.set(false);
        });
    });

    let errors_signal = Signal::derive(move || errors.get());

    view! {
        <ContentContainer max_width="900px" data_testid="chronon-job-create-page">
            <div id="chronon-job-create-page">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <div id="chronon-job-create-back" data-testid="job-create-breadcrumb">
                    <Breadcrumb>
                        <BreadcrumbItem>
                            <a href=crate::paths::JOBS>
                                <BreadcrumbButton>"Jobs"</BreadcrumbButton>
                            </a>
                        </BreadcrumbItem>
                        <BreadcrumbDivider />
                        <BreadcrumbItem>
                            <BreadcrumbButton current=true>"New job"</BreadcrumbButton>
                        </BreadcrumbItem>
                    </Breadcrumb>
                </div>

                <Title3>"New job"</Title3>

                <Show when=move || !errors.get().is_empty()>
                    <div data-testid="create-errors-summary">
                        <MessageBar intent=MessageBarIntent::Error>
                            <MessageBarTitle>{move || errors_summary(&errors.get())}</MessageBarTitle>
                            <MessageBarBody>
                                {move || errors
                                    .get()
                                    .iter()
                                    .map(|(key, _)| field_label(key))
                                    .collect::<Vec<_>>()
                                    .join(", ")}
                            </MessageBarBody>
                        </MessageBar>
                    </div>
                </Show>

                <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                    <div id="chronon-job-create-basic">
                    <BasicInfoSection
                        job_name=job_name
                        selected_script=selected_script
                        scripts_res=scripts_res
                        errors=errors_signal
                    />
                    </div>

                    <Show when=move || scripts_res.get().and_then(Result::ok).is_some()>
                        <div id="chronon-job-create-params">
                        <ParametersSection
                            selected_script=selected_script
                            scripts=scripts.get()
                            param_signals=param_signals
                            param_errors=param_errors
                        />
                        </div>
                    </Show>

                    <div id="chronon-job-create-schedule">
                    <ScheduleSection
                        schedule_type=schedule_type
                        cron_expression=cron_expression
                        timezone=timezone
                        preview=preview
                        run_once=run_once
                        errors=errors_signal
                    />
                    </div>

                    <div id="chronon-job-create-advanced">
                    <AdvancedOptionsSection
                        show_advanced=show_advanced
                        concurrency=concurrency
                        timeout_seconds=timeout_seconds
                        max_retries=max_retries
                        pool=pool
                    />
                    </div>

                    <ActionsSection
                        on_cancel=on_cancel
                        on_create=on_create
                        create_loading=create_loading
                    />
                    <Show when=move || create_error.get().is_some()>
                        <MessageBar intent=MessageBarIntent::Error>
                            {move || create_error.get().unwrap_or_default()}
                        </MessageBar>
                    </Show>
                </Flex>
            </Flex>
            </div>
        </ContentContainer>
    }
}
