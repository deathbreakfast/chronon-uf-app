//! Shared schedule fields: Repeats, At, Cron with its sentence, and Timezone.
//!
//! The job page's Edit schedule dialog and the create form both mount
//! [`ScheduleFields`]. It writes the cron and timezone signals the parent owns and
//! keeps a debounced [`SchedulePreview`] from `preview_schedule` in `preview`.

use std::time::Duration;

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use orbital::components::{Caption1, FormHint, SpacingSize};
use orbital::primitives::{
    Combobox, ComboboxBind, ComboboxOption, Field, FieldInjection, FieldValidationState, Flex,
    Input, InputAppearance, InputType, Select, Table, TableBody, TableCell, TableHeader,
    TableHeaderCell, TableRow,
};

use crate::server::{preview_schedule, ScheduleField, SchedulePreview};

/// Repeats presets, in the order the Select lists them.
const PRESETS: [(&str, &str); 6] = [
    ("hourly", "Every hour"),
    ("daily", "Every day"),
    ("weekdays", "Every weekday"),
    ("weekly", "Every week"),
    ("monthly", "Every month"),
    ("custom", "Custom"),
];

/// Cron for a preset at `HH:MM`, or `None` for Custom or a bad time.
fn preset_cron(preset: &str, at: &str) -> Option<String> {
    let (h, m) = at.split_once(':')?;
    let h: u32 = h.trim().parse().ok().filter(|h| *h < 24)?;
    let m: u32 = m.trim().parse().ok().filter(|m| *m < 60)?;
    match preset {
        "hourly" => Some(format!("{m} * * * *")),
        "daily" => Some(format!("{m} {h} * * *")),
        "weekdays" => Some(format!("{m} {h} * * 1-5")),
        "weekly" => Some(format!("{m} {h} * * 1")),
        "monthly" => Some(format!("{m} {h} 1 * *")),
        _ => None,
    }
}

/// Preset and `HH:MM` a cron matches, or `("custom", "")`.
fn detect_preset(cron: &str) -> (&'static str, String) {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    let num = |s: &str| s.parse::<u32>().ok();
    if let [m, h, dom, mon, dow] = parts[..] {
        if let Some(m) = num(m).filter(|m| *m < 60) {
            if h == "*" && dom == "*" && mon == "*" && dow == "*" {
                return ("hourly", format!("00:{m:02}"));
            }
            if let Some(h) = num(h).filter(|h| *h < 24) {
                let at = format!("{h:02}:{m:02}");
                match (dom, mon, dow) {
                    ("*", "*", "*") => return ("daily", at),
                    ("*", "*", "1-5") => return ("weekdays", at),
                    ("*", "*", "1") => return ("weekly", at),
                    ("1", "*", "*") => return ("monthly", at),
                    _ => {}
                }
            }
        }
    }
    ("custom", String::new())
}

/// `Wed, Nov 25 at 03:00 UTC` for an RFC 3339 time.
#[must_use]
pub fn format_run_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339).map_or_else(
        |_| rfc3339.to_string(),
        |t| {
            t.with_timezone(&Utc)
                .format("%a, %b %-d at %H:%M UTC")
                .to_string()
        },
    )
}

/// `in 17 h`, `in 5 min`, `in 3 days` (or `… ago`) relative to now.
#[must_use]
pub fn relative_to_now(rfc3339: &str) -> String {
    let Ok(t) = DateTime::parse_from_rfc3339(rfc3339) else {
        return String::new();
    };
    let secs = t
        .with_timezone(&Utc)
        .signed_duration_since(Utc::now())
        .num_seconds();
    let (future, secs) = (secs >= 0, secs.unsigned_abs());
    let amount = if secs < 3_600 {
        format!("{} min", (secs / 60).max(1))
    } else if secs < 48 * 3_600 {
        format!("{} h", secs / 3_600)
    } else {
        format!("{} days", secs / 86_400)
    };
    if future {
        format!("in {amount}")
    } else {
        format!("{amount} ago")
    }
}

/// Puts `error` on the enclosing Orbital `Field` (validation state and message).
#[component]
pub fn FieldErrorSlot(
    /// The message, or `None` for a valid field.
    #[prop(into)]
    error: Signal<Option<String>>,
) -> impl IntoView {
    let injection = FieldInjection::use_context();
    Effect::new(move |_| {
        let state = error
            .get()
            .map_or(Ok(()), |e| Err(FieldValidationState::Error(e)));
        if let Some(injection) = &injection {
            injection.update_validation_state(state);
        }
    });
}

/// Table of upcoming run times (UTC) with a relative caption beside each.
///
/// When `before` is set the table has Now and After you save columns and times that
/// differ are in strong text.
#[component]
pub fn NextRunsTable(
    /// Upcoming runs (RFC 3339), after the change when `before` is set.
    #[prop(into)]
    runs: Signal<Vec<String>>,
    /// Runs before the change, for a two-column preview.
    #[prop(optional, into)]
    before: Option<Signal<Vec<String>>>,
    /// Root test id.
    #[prop(into)]
    testid: String,
) -> impl IntoView {
    view! {
        <div data-testid=testid>
            <Table>
                <TableHeader>
                    <TableRow>
                        {before.map(|_| view! { <TableHeaderCell>"Now"</TableHeaderCell> })}
                        <TableHeaderCell>
                            {if before.is_some() { "After you save" } else { "Runs at" }}
                        </TableHeaderCell>
                        <TableHeaderCell>"When"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || {
                        let after = runs.get();
                        let prior = before.map(|b| b.get());
                        let rows = after.len().max(prior.as_ref().map_or(0, Vec::len));
                        (0..rows)
                            .map(|i| {
                                let new = after.get(i).cloned().unwrap_or_default();
                                let old = prior.as_ref().map(|p| p.get(i).cloned().unwrap_or_default());
                                let changed = old.as_ref().is_some_and(|o| o != &new);
                                let new_text = format_run_time(&new);
                                let rel = relative_to_now(&new);
                                view! {
                                    <TableRow>
                                        {old.map(|o| view! { <TableCell>{format_run_time(&o)}</TableCell> })}
                                        <TableCell>
                                            {if changed {
                                                view! { <strong data-changed="true">{new_text}</strong> }.into_any()
                                            } else {
                                                view! { <span>{new_text}</span> }.into_any()
                                            }}
                                        </TableCell>
                                        <TableCell><Caption1>{rel}</Caption1></TableCell>
                                    </TableRow>
                                }
                            })
                            .collect_view()
                    }}
                </TableBody>
            </Table>
        </div>
    }
}

/// Repeats, At, Cron (with the sentence) and Timezone, bound to the parent's signals.
#[allow(clippy::too_many_lines)]
#[component]
pub fn ScheduleFields(
    /// Cron expression the fields edit.
    cron: RwSignal<String>,
    /// IANA timezone the fields edit.
    timezone: RwSignal<String>,
    /// Latest preview for `cron` and `timezone` (debounced 300 ms).
    preview: RwSignal<SchedulePreview>,
) -> impl IntoView {
    let (initial_preset, initial_at) = detect_preset(&cron.get_untracked());
    let repeats = RwSignal::new(initial_preset.to_string());
    let at = RwSignal::new(if initial_at.is_empty() {
        "03:00".to_string()
    } else {
        initial_at
    });
    // Last cron the presets wrote; any other change came from typing in the field.
    let last_built = StoredValue::new(cron.get_untracked());

    Effect::new(move |_| {
        let preset = repeats.get();
        let time = at.get();
        if let Some(built) = preset_cron(&preset, &time) {
            last_built.set_value(built.clone());
            if cron.get_untracked() != built {
                cron.set(built);
            }
        }
    });
    Effect::new(move |_| {
        let typed = cron.get();
        if typed != last_built.get_value() && repeats.get_untracked() != "custom" {
            repeats.set("custom".to_string());
        }
    });

    // Debounced preview: only the newest request may write `preview`.
    let generation = StoredValue::new(0u64);
    Effect::new(move |_| {
        let expr = cron.get();
        let tz = timezone.get();
        let gen = generation.get_value() + 1;
        generation.set_value(gen);
        // The timeout fires outside the reactive owner; re-enter it so the request is
        // dropped with the fields.
        let owner = Owner::current();
        set_timeout(
            move || {
                if generation.get_value() != gen {
                    return;
                }
                let Some(owner) = owner else {
                    return;
                };
                owner.with(|| {
                    leptos::task::spawn_local_scoped(async move {
                        let result = preview_schedule(expr, tz).await;
                        if generation.get_value() == gen {
                            if let Ok(p) = result {
                                preview.set(p);
                            }
                        }
                    });
                });
            },
            Duration::from_millis(300),
        );
    });

    let cron_error = Signal::derive(move || {
        preview
            .get()
            .error
            .filter(|e| e.field != ScheduleField::Timezone)
            .map(|e| e.message)
    });
    let tz_error = Signal::derive(move || {
        preview
            .get()
            .error
            .filter(|e| e.field == ScheduleField::Timezone)
            .map(|e| e.message)
    });
    let tz_selected = RwSignal::new(vec![timezone.get_untracked()]);
    Effect::new(move |_| {
        if let Some(picked) = tz_selected.get().into_iter().next() {
            if timezone.get_untracked() != picked {
                timezone.set(picked);
            }
        }
    });

    view! {
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            <div data-testid="schedule-repeats">
                <Field label="Repeats">
                    <Select bind=repeats>
                        {PRESETS
                            .iter()
                            .map(|(value, label)| view! { <option value=*value>{*label}</option> })
                            .collect_view()}
                    </Select>
                </Field>
            </div>
            <Show when=move || repeats.get() != "custom">
                <div data-testid="schedule-at">
                    <Field label="At">
                        <Input
                            bind=at
                            appearance=InputAppearance {
                                input_type: Signal::from(InputType::Time),
                                ..Default::default()
                            }
                        />
                    </Field>
                </div>
            </Show>
            <div data-testid="cron-expr">
                <Field label="Cron">
                    <Input bind=cron appearance=InputAppearance::with_placeholder("0 3 * * *") />
                    <FieldErrorSlot error=cron_error />
                </Field>
                <FormHint test_id="schedule-sentence">
                    {move || preview.get().sentence}
                </FormHint>
            </div>
            <div data-testid="schedule-timezone">
                <Field label="Timezone">
                    <Combobox bind=ComboboxBind::new(timezone, tz_selected)>
                        {chrono_tz::TZ_VARIANTS
                            .iter()
                            .map(|tz| view! { <ComboboxOption text=tz.name().to_string() /> })
                            .collect_view()}
                    </Combobox>
                    <FieldErrorSlot error=tz_error />
                </Field>
            </div>
        </Flex>
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_preset, format_run_time, preset_cron};

    #[test]
    fn presets_round_trip() {
        for preset in ["hourly", "daily", "weekdays", "weekly", "monthly"] {
            let cron = preset_cron(preset, "04:30").unwrap();
            assert_eq!(detect_preset(&cron).0, preset, "{cron}");
        }
        assert_eq!(preset_cron("daily", "04:30").unwrap(), "30 4 * * *");
        assert_eq!(detect_preset("0 3 * * *"), ("daily", "03:00".to_string()));
        assert_eq!(detect_preset("*/5 * * * *").0, "custom");
        assert!(preset_cron("custom", "04:30").is_none());
        assert!(preset_cron("daily", "25:00").is_none());
    }

    #[test]
    fn run_time_formats_in_utc() {
        assert_eq!(
            format_run_time("2026-11-25T03:00:00+00:00"),
            "Wed, Nov 25 at 03:00 UTC"
        );
    }
}
