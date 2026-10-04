mod components;
mod form_state;

use std::collections::HashMap;

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use orbital::components::{ContentContainer, SpacingSize, Title3};
use orbital::primitives::{Button, ButtonAppearance, Flex, MessageBar, MessageBarIntent};

use crate::server::{
    create_job, get_scripts, CreateJobRequest, CreateJobScheduleType, Script, DEFAULT_POOL,
};
use components::{
    ActionsSection, AdvancedOptionsSection, BasicInfoSection, ParametersSection, ScheduleSection,
};
use form_state::{build_create_params, collect_param_values, seed_param_signals};

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
    let cron_expression = RwSignal::new(String::from("0 * * * *"));
    let timezone = RwSignal::new(String::from("UTC"));
    let run_once_datetime = RwSignal::new(String::new());
    let param_signals = RwSignal::new(HashMap::<String, RwSignal<String>>::new());

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

        if job_name.get().trim().is_empty() {
            create_error.set(Some("Job name is required.".to_string()));
            return;
        }
        if selected_script.get().trim().is_empty() {
            create_error.set(Some("Please select a script.".to_string()));
            return;
        }
        if matches!(schedule_type.get(), ScheduleType::Cron)
            && cron_expression.get().trim().is_empty()
        {
            create_error.set(Some(
                "Cron expression is required for cron schedules.".to_string(),
            ));
            return;
        }
        if matches!(schedule_type.get(), ScheduleType::RunOnce)
            && run_once_datetime.get().trim().is_empty()
        {
            create_error.set(Some(
                "Run-once datetime is required for run-once schedules.".to_string(),
            ));
            return;
        }

        let scripts: Vec<Script> = scripts_res.get().and_then(Result::ok).unwrap_or_default();
        let script_name = selected_script.get();
        let script = if let Some(script) = scripts.iter().find(|s| s.name == script_name) {
            script.clone()
        } else {
            create_error.set(Some("Please select a script.".to_string()));
            return;
        };

        let values = collect_param_values(&param_signals.get());
        let params = match build_create_params(&script.params, &values) {
            Ok(params) => params,
            Err(err) => {
                create_error.set(Some(err));
                return;
            }
        };

        create_loading.set(true);
        create_error.set(None);
        let payload = CreateJobRequest {
            job_name: job_name.get(),
            script_name: selected_script.get(),
            schedule_type: match schedule_type.get() {
                ScheduleType::Cron => CreateJobScheduleType::Cron,
                ScheduleType::RunOnce => CreateJobScheduleType::RunOnce,
                ScheduleType::Manual => CreateJobScheduleType::Manual,
            },
            cron_expr: if matches!(schedule_type.get(), ScheduleType::Cron) {
                Some(cron_expression.get())
            } else {
                None
            },
            timezone: if matches!(schedule_type.get(), ScheduleType::Cron) {
                let tz = timezone.get();
                if tz.trim().is_empty() {
                    None
                } else {
                    Some(tz)
                }
            } else {
                None
            },
            run_once_at: if matches!(schedule_type.get(), ScheduleType::RunOnce) {
                Some(run_once_datetime.get())
            } else {
                None
            },
            params,
            concurrency: concurrency.get().parse::<u32>().unwrap_or(1),
            timeout_seconds: timeout_seconds.get().parse::<u32>().unwrap_or(300),
            max_retries: max_retries.get().parse::<u32>().unwrap_or(3),
            pool: Some(pool.get()),
        };

        leptos::task::spawn_local_scoped(async move {
            match create_job(payload).await {
                Ok(_) => navigate_store.with_value(|navigate| {
                    navigate(crate::paths::JOBS, NavigateOptions::default());
                }),
                Err(err) => create_error.set(Some(err.to_string())),
            }
            create_loading.set(false);
        });
    });

    view! {
        <ContentContainer max_width="900px" data_testid="chronon-job-create-page">
            <div id="chronon-job-create-page">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <div id="chronon-job-create-back">
                <Button
                    appearance=ButtonAppearance::Subtle
                    icon=icondata::AiArrowLeftOutlined
                    on_click=Callback::new(move |_| {
                        navigate_store.with_value(|navigate| {
                            navigate(crate::paths::JOBS, NavigateOptions::default());
                        });
                    })
                >
                    "Back to Jobs"
                </Button>
                </div>

                <Title3>"Create New Job"</Title3>

                <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                    <div id="chronon-job-create-basic">
                    <BasicInfoSection
                        job_name=job_name
                        selected_script=selected_script
                        scripts_res=scripts_res
                    />
                    </div>

                    <Show when=move || scripts_res.get().and_then(Result::ok).is_some()>
                        <div id="chronon-job-create-params">
                        <ParametersSection
                            selected_script=selected_script
                            scripts=scripts.get()
                            param_signals=param_signals
                        />
                        </div>
                    </Show>

                    <div id="chronon-job-create-schedule">
                    <ScheduleSection
                        schedule_type=schedule_type
                        cron_expression=cron_expression
                        timezone=timezone
                        run_once_datetime=run_once_datetime
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
