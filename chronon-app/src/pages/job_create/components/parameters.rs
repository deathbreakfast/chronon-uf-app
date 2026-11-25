use std::collections::HashMap;

use leptos::prelude::*;
use orbital::components::{Caption1, Caption2, FormHint, SpacingSize};
use orbital::primitives::{Field, Flex, Input, Switch};
use turf::inline_style_sheet_values;

use crate::components::{ChrononHelpSectionHeader, FieldErrorSlot};
use crate::pages::job_create::form_state::param_type_words;
use crate::server::{Script, ScriptParam};

use super::FormSectionCard;

/// One typed field per script parameter: the type in words, its error on the field,
/// and a Switch for yes-or-no values.
#[component]
pub fn TypedParamFields(
    /// Declared parameters, in order.
    #[prop(into)]
    params: Signal<Vec<ScriptParam>>,
    /// Field text per parameter name (`"true"` / `"false"` for yes-or-no).
    values: RwSignal<HashMap<String, RwSignal<String>>>,
    /// Error per parameter name.
    errors: RwSignal<HashMap<String, String>>,
) -> impl IntoView {
    view! {
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            {move || {
                params
                    .get()
                    .into_iter()
                    .filter_map(|param| {
                        let name = param.name.clone();
                        let signal = values.with(|m| m.get(&name).copied())?;
                        let words = param_type_words(&param.param_type);
                        let error_name = name.clone();
                        let error = Signal::derive(move || errors.with(|e| e.get(&error_name).cloned()));
                        let control = if param.param_type == "bool" {
                            let on = RwSignal::new(signal.get_untracked() == "true");
                            if signal.get_untracked().is_empty() {
                                signal.set("false".to_string());
                            }
                            Effect::new(move |_| signal.set(on.get().to_string()));
                            view! { <Switch bind=on /> }.into_any()
                        } else {
                            view! { <Input bind=signal /> }.into_any()
                        };
                        Some(view! {
                            <div data-testid=format!("param-{name}")>
                                <Field label=name.clone() required=param.required>
                                    {control}
                                    <FieldErrorSlot error=error />
                                </Field>
                                <FormHint>{words}</FormHint>
                            </div>
                        })
                    })
                    .collect_view()
            }}
        </Flex>
    }
}

/// Parameter editor for the currently selected script.
#[component]
pub fn ParametersSection(
    /// Two-way signal holding the selected script.
    selected_script: RwSignal<String>,
    /// List of scripts.
    scripts: Vec<Script>,
    /// Two-way signal holding the param signals.
    param_signals: RwSignal<HashMap<String, RwSignal<String>>>,
    /// Error per parameter name.
    param_errors: RwSignal<HashMap<String, String>>,
) -> impl IntoView {
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .Muted {
            color: var(--colorNeutralForeground3);
        }
    };

    let selected_script_details = move || {
        let name = selected_script.get();
        scripts.iter().find(|s| s.name == name).cloned()
    };

    view! {
        <style>{style_sheet}</style>
        {move || {
            selected_script_details().map(|script| {
                if script.params.is_empty() {
                    view! {
                        <FormSectionCard>
                            <ChrononHelpSectionHeader title="Parameters" />
                            <Caption2 italic=true class=class_names.muted>
                                "This script has no parameters."
                            </Caption2>
                        </FormSectionCard>
                    }.into_any()
                } else {
                    let params = script.params;
                    view! {
                        <FormSectionCard>
                            <ChrononHelpSectionHeader title="Parameters" />
                            <TypedParamFields
                                params=Signal::derive(move || params.clone())
                                values=param_signals
                                errors=param_errors
                            />
                            <Caption1>
                                "Values are stored with the job. Anyone who can view the job can read them."
                            </Caption1>
                        </FormSectionCard>
                    }.into_any()
                }
            })
        }}
    }
}
