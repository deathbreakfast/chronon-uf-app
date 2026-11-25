use leptos::prelude::*;
use orbital::components::{FormHint, SpacingSize, Subtitle2};
use orbital::primitives::{
    Combobox, ComboboxBind, ComboboxOption, DatePicker, DatePickerBind, Field, Flex, Input,
    InputAppearance, InputType, MessageBar, MessageBarBody, MessageBarIntent, OptionBind, Radio,
    RadioGroup, RadioGroupBind,
};

use crate::components::{
    relative_to_now, ChrononHelpSectionHeader, FieldErrorSlot, NextRunsTable, ScheduleFields,
};
use crate::pages::job_create::form_state::{error_for, run_once_instant, FieldErrors};
use crate::pages::job_create::ScheduleType;
use crate::server::SchedulePreview;

use super::FormSectionCard;

/// Run-once inputs: date, time and timezone.
#[derive(Clone, Copy)]
pub struct RunOnceFields {
    /// `YYYY-MM-DD`, empty until a date is picked.
    pub date: RwSignal<String>,
    /// `HH:MM`.
    pub time: RwSignal<String>,
    /// IANA timezone the date and time are in.
    pub timezone: RwSignal<String>,
}

/// Schedule choice (On a schedule / Once / Only from Run now) and its fields.
#[allow(clippy::too_many_lines)]
#[component]
pub fn ScheduleSection(
    /// Two-way signal holding the schedule type.
    schedule_type: RwSignal<ScheduleType>,
    /// Cron expression for On a schedule.
    cron_expression: RwSignal<String>,
    /// Timezone for On a schedule.
    timezone: RwSignal<String>,
    /// Debounced preview for the cron and timezone.
    preview: RwSignal<SchedulePreview>,
    /// Date, time and timezone for Once.
    run_once: RunOnceFields,
    /// Field errors from the last Create attempt.
    errors: Signal<FieldErrors>,
) -> impl IntoView {
    let schedule_option = RwSignal::new(Some(
        match schedule_type.get_untracked() {
            ScheduleType::Cron => "cron",
            ScheduleType::RunOnce => "run_once",
            ScheduleType::Manual => "manual",
        }
        .to_string(),
    ));

    Effect::new(move || {
        let val = schedule_option.get().unwrap_or_else(|| "cron".to_string());
        let st = match val.as_str() {
            "run_once" => ScheduleType::RunOnce,
            "manual" => ScheduleType::Manual,
            _ => ScheduleType::Cron,
        };
        schedule_type.set(st);
    });

    // The picker's value type stays inside Orbital; keep a plain date string beside it.
    let picked = DatePickerBind::default().value;
    let picked_for_effect = picked.clone();
    let picked = StoredValue::new(picked);
    Effect::new(move |_| {
        let date = picked_for_effect
            .get()
            .and_then(|d| d.wall_date())
            .map(|d| d.to_string())
            .unwrap_or_default();
        run_once.date.set(date);
    });
    let once_tz_selected = RwSignal::new(vec![run_once.timezone.get_untracked()]);
    Effect::new(move |_| {
        if let Some(picked) = once_tz_selected.get().into_iter().next() {
            run_once.timezone.set(picked);
        }
    });

    let first_runs = Signal::derive(move || {
        preview.with(|p| p.next.iter().take(3).cloned().collect::<Vec<_>>())
    });
    let once_error = Signal::derive(move || error_for(&errors.get(), "run_once"));
    let once_sentence = move || {
        run_once_instant(
            &run_once.date.get(),
            &run_once.time.get(),
            &run_once.timezone.get(),
        )
        .map(|at| {
            format!(
                "Runs once on {}, {}.",
                at.format("%a, %b %-d at %H:%M UTC"),
                relative_to_now(&at.to_rfc3339())
            )
        })
    };

    view! {
        <FormSectionCard>
            <ChrononHelpSectionHeader title="Schedule" />
            <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                <div data-testid="schedule-type">
                    <RadioGroup bind=RadioGroupBind {
                        value: OptionBind::Signal(schedule_option),
                        name: "schedule_type".into(),
                        ..Default::default()
                    }>
                        <Radio value="cron" label="On a schedule" />
                        <Radio value="run_once" label="Once" />
                        <Radio value="manual" label="Only from Run now" />
                    </RadioGroup>
                </div>

                {move || match schedule_type.get() {
                    ScheduleType::Cron => view! {
                        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                            <ScheduleFields cron=cron_expression timezone=timezone preview=preview />
                            <Show when=move || !first_runs.get().is_empty()>
                                <Subtitle2>"First runs"</Subtitle2>
                                <NextRunsTable runs=first_runs testid="create-first-runs" />
                            </Show>
                        </Flex>
                    }.into_any(),
                    ScheduleType::RunOnce => view! {
                        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                            <div data-testid="run-once-date">
                                <Field label="Date" required=true>
                                    <DatePicker bind=DatePickerBind { value: picked.get_value(), ..Default::default() } />
                                    <FieldErrorSlot error=once_error />
                                </Field>
                            </div>
                            <div data-testid="run-once-time">
                                <Field label="Time" required=true>
                                    <Input
                                        bind=run_once.time
                                        appearance=InputAppearance {
                                            input_type: Signal::from(InputType::Time),
                                            ..Default::default()
                                        }
                                    />
                                </Field>
                            </div>
                            <div data-testid="run-once-timezone">
                                <Field label="Timezone">
                                    <Combobox bind=ComboboxBind::new(run_once.timezone, once_tz_selected)>
                                        {chrono_tz::TZ_VARIANTS
                                            .iter()
                                            .map(|tz| view! { <ComboboxOption text=tz.name().to_string() /> })
                                            .collect_view()}
                                    </Combobox>
                                </Field>
                            </div>
                            {move || once_sentence().map(|text| view! {
                                <div data-testid="run-once-sentence">
                                    <MessageBar intent=MessageBarIntent::Info>
                                        <MessageBarBody>{text}</MessageBarBody>
                                    </MessageBar>
                                </div>
                            })}
                        </Flex>
                    }.into_any(),
                    ScheduleType::Manual => view! {
                        <FormHint>"This job runs only when someone uses Run now."</FormHint>
                    }.into_any(),
                }}
            </Flex>
        </FormSectionCard>
    }
}
