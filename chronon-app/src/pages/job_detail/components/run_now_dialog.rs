use leptos::prelude::*;
use orbital::components::{Body1, Caption1, Caption2, SpacingSize};
use orbital::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, FieldValidationMessage, FieldValidationState, Flex, Grid, GridConfig,
    GridItem, Input, Link, MessageBar, MessageBarActions, MessageBarBody, MessageBarIntent,
    MessageBarTitle, Spinner,
};
use orbital_motion::{MotionCurve, OrbitalPresence, PresenceMotion};

use crate::server::{get_run_now_facts, RunNowFacts, ScriptParam};

use super::super::actions::submit_run_now;

/// What a Run now hands back to the page once the run is queued.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunStarted {
    /// Id of the new run.
    pub run_id: String,
    /// Worker pool it runs in.
    pub pool: String,
}

/// Props for [`RunNowDialog`].
#[derive(Clone)]
pub struct RunNowDialogInput {
    /// Open state; the page's Run now (or Run again) sets it.
    pub open: RwSignal<bool>,
    /// Job to run.
    pub job_id: String,
    /// Job name for the title.
    pub job_name: String,
    /// Values to start from (Run again uses the run's values); `None` uses the stored ones.
    pub initial_params: Signal<Option<serde_json::Value>>,
    /// True while the run is being queued (the page's Run now button shows it too).
    pub loading: RwSignal<bool>,
    /// Called once the run is queued; the dialog has closed by then.
    pub on_started: Callback<RunStarted>,
}

/// One typed field: the script parameter, its text, and the stored value as text.
#[derive(Clone)]
struct ParamField {
    param: ScriptParam,
    text: RwSignal<String>,
    stored: String,
}

impl ParamField {
    fn parsed(&self) -> Result<Option<serde_json::Value>, String> {
        chronon_backend::coerce_script_param(
            &self.param.param_type,
            &self.text.get(),
            self.param.required,
        )
    }
}

fn build_fields(facts: &RunNowFacts, start: Option<&serde_json::Value>) -> Vec<ParamField> {
    facts
        .script_params
        .iter()
        .map(|param| {
            let stored = chronon_backend::param_field_text(facts.stored_params.get(&param.name));
            let initial = start
                .map(|values| chronon_backend::param_field_text(values.get(&param.name)))
                .unwrap_or_else(|| stored.clone());
            ParamField {
                param: param.clone(),
                text: RwSignal::new(initial),
                stored,
            }
        })
        .collect()
}

/// Run now: says what will run (script, pool, timeout, retries), takes one typed field per
/// script parameter holding the stored value, and states that the schedule doesn't change.
#[allow(clippy::too_many_lines)]
#[component]
pub fn RunNowDialog(
    /// Bundled props for the component.
    props: RunNowDialogInput,
) -> impl IntoView {
    let RunNowDialogInput {
        open,
        job_id,
        job_name,
        initial_params,
        loading,
        on_started,
    } = props;
    let error = RwSignal::new(None::<String>);

    // The dialog owns its data: opening it loads the facts on the client, nothing else on
    // the page.
    let facts_job_id = job_id.clone();
    let facts = crate::utils::client_fetch(move || {
        let is_open = open.get();
        let id = facts_job_id.clone();
        async move {
            Some(if is_open {
                Some(get_run_now_facts(id).await.map_err(|e| e.to_string()))
            } else {
                None
            })
        }
    });
    // Fields are rebuilt from each load: the stored values, or the run's values for Run again.
    let fields = RwSignal::new(Vec::<ParamField>::new());
    let loaded = RwSignal::new(None::<RunNowFacts>);
    Effect::new(move |_| match facts.get() {
        Some(Some(Ok(f))) => {
            let start = initial_params.get_untracked();
            fields.set(build_fields(&f, start.as_ref()));
            loaded.set(Some(f));
        }
        _ => loaded.set(None),
    });
    Effect::new(move |_| {
        if open.get() {
            error.set(None);
        }
    });

    let has_field_error = Memo::new(move |_| fields.get().iter().any(|f| f.parsed().is_err()));
    let run_disabled = Signal::derive(move || {
        loading.get() || has_field_error.get() || !loaded.get().is_some_and(|f| f.can_run_now)
    });

    let submit_job_id = job_id;
    let on_submit = Callback::new(move |_| {
        if run_disabled.get_untracked() {
            return;
        }
        let mut params = serde_json::Map::new();
        for field in fields.get_untracked() {
            if let Ok(Some(value)) = field.parsed() {
                params.insert(field.param.name, value);
            }
        }
        let pool = loaded.get_untracked().map(|f| f.pool).unwrap_or_default();
        error.set(None);
        loading.set(true);
        submit_run_now(
            submit_job_id.clone(),
            serde_json::Value::Object(params),
            loading,
            error,
            Callback::new(move |run_id: String| {
                open.set(false);
                on_started.run(RunStarted {
                    run_id,
                    pool: pool.clone(),
                });
            }),
        );
    });

    let open_signal = open.read_only();
    let panel_motion =
        Signal::from(PresenceMotion::fade_scale().with_curve(MotionCurve::DecelerateMid));
    let title = format!("Run {job_name} now?");

    view! {
        <Dialog open=open>
            <OrbitalPresence appear=true show=open_signal motion=panel_motion>
                <DialogSurface>
                <DialogBody>
                    <DialogTitle>{title}</DialogTitle>
                    <DialogContent>
                        <div data-testid="run-now-dialog">
                            {move || match facts.get() {
                                Some(Some(Ok(f))) => view! { <RunNowBody facts=f fields=fields /> }.into_any(),
                                Some(Some(Err(e))) => view! {
                                    <MessageBar intent=MessageBarIntent::Error>{e}</MessageBar>
                                }.into_any(),
                                _ => view! { <Spinner /> }.into_any(),
                            }}
                            <Show when=move || error.get().is_some() fallback=|| ()>
                                <MessageBar intent=MessageBarIntent::Error>
                                    {move || error.get().unwrap_or_default()}
                                </MessageBar>
                            </Show>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=Signal::derive(move || loading.get())
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            "Cancel"
                        </Button>
                        <div data-testid="run-now-submit">
                            <Button
                                appearance=ButtonAppearance::Primary
                                disabled=run_disabled
                                on_click=on_submit
                            >
                                {move || if loading.get() { "Starting…" } else { "Run now" }}
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
            </OrbitalPresence>
        </Dialog>
    }
}

/// Dialog body once the facts are in: facts grid, typed fields, schedule line.
#[component]
fn RunNowBody(facts: RunNowFacts, fields: RwSignal<Vec<ParamField>>) -> impl IntoView {
    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .Label { color: var(--colorNeutralForeground3); }
    };
    let timeout = facts
        .timeout
        .clone()
        .unwrap_or_else(|| "No time limit".to_string());
    let no_params = facts.script_params.is_empty();
    view! {
        <style>{style_sheet}</style>
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            <div data-testid="run-now-facts">
                <Grid config=GridConfig::with_gaps(2, 16, 8)>
                    <GridItem><Caption2 class=class_names.label>"Script"</Caption2></GridItem>
                    <GridItem><Body1>{facts.script_name.clone()}</Body1></GridItem>
                    <GridItem><Caption2 class=class_names.label>"Worker pool"</Caption2></GridItem>
                    <GridItem><Body1>{facts.pool.clone()}</Body1></GridItem>
                    <GridItem><Caption2 class=class_names.label>"Times out after"</Caption2></GridItem>
                    <GridItem><Body1>{timeout}</Body1></GridItem>
                    <GridItem><Caption2 class=class_names.label>"If it fails"</Caption2></GridItem>
                    <GridItem><Body1>{facts.if_it_fails.clone()}</Body1></GridItem>
                </Grid>
            </div>
            {no_params.then(|| view! { <Caption1>"This script takes no parameters."</Caption1> })}
            {move || fields.get().into_iter().map(|field| view! { <ParamInput field=field /> }).collect_view()}
            {(!facts.can_run_now).then(|| view! {
                <MessageBar intent=MessageBarIntent::Warning>
                    "This job runs on its schedule only. Nobody can start it by hand."
                </MessageBar>
            })}
            {facts.waits_for_running.then(|| view! {
                <div data-testid="run-now-waits">
                    <MessageBar intent=MessageBarIntent::Warning>
                        "A run of this job is already going. The job runs one at a time, so this run waits until that one finishes."
                    </MessageBar>
                </div>
            })}
            <div data-testid="run-now-schedule-line">
                <MessageBar intent=MessageBarIntent::Info>{facts.schedule_line.clone()}</MessageBar>
            </div>
            <Caption1>"The run records that you started it and the values it used."</Caption1>
        </Flex>
    }
}

/// One typed parameter field with its error, or a hint when it differs from the stored value.
#[component]
fn ParamInput(field: ParamField) -> impl IntoView {
    let label = format!("{} ({})", field.param.name, field.param.param_type);
    let testid = format!("run-now-param-{}", field.param.name);
    let text = field.text;
    let stored = field.stored.clone();
    let check = field.clone();
    let state = Memo::new(move |_| match check.parsed() {
        Err(message) => Some(FieldValidationState::Error(message)),
        Ok(_) => None,
    });
    let hint = move || {
        let now = text.get();
        let now = now.trim();
        (state.get().is_none() && now != stored.trim()).then(|| {
            let kept = if stored.is_empty() {
                "empty".to_string()
            } else {
                stored.clone()
            };
            format!("Stored value: {kept}. This run uses {now}; the stored value stays {kept}.")
        })
    };
    view! {
        <div data-testid=testid>
            <Field label=label required=field.param.required>
                <Input bind=text />
            </Field>
            {move || state.get().map(|s| view! { <FieldValidationMessage state=s /> })}
            {move || hint().map(|h| view! { <Caption1>{h}</Caption1> })}
        </div>
    }
}

/// Success message the job page shows after Run now: the run started, where it runs, that
/// the schedule is unchanged, and a way to open the run.
#[component]
pub fn RunStartedNotice(
    /// Set by the Run now dialog's `on_started`; cleared by Dismiss.
    notice: RwSignal<Option<RunStarted>>,
) -> impl IntoView {
    move || {
        notice.get().map(|started| {
            let href = chronon_backend::chronon_run_path(&started.run_id);
            view! {
                <div data-testid="chronon-run-started-notice">
                    <MessageBar intent=MessageBarIntent::Success>
                        <MessageBarBody>
                            <MessageBarTitle>"Run started"</MessageBarTitle>
                            {format!(
                                "It runs in the {} pool. The schedule is unchanged.",
                                started.pool
                            )}
                        </MessageBarBody>
                        <MessageBarActions>
                            <div data-testid="chronon-run-started-open">
                                <Link href=href>"Open the run"</Link>
                            </div>
                            <Button
                                appearance=ButtonAppearance::Transparent
                                on_click=Callback::new(move |_| notice.set(None))
                            >
                                "Dismiss"
                            </Button>
                        </MessageBarActions>
                    </MessageBar>
                </div>
            }
        })
    }
}
