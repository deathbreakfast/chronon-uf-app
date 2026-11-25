use leptos::prelude::*;
use orbital::components::Subtitle2;
use orbital::primitives::{
    Field, Grid, GridConfig, GridItem, Input, InputAppearance, MessageBar, MessageBarIntent,
    Select, Skeleton, SkeletonItem,
};
use turf::inline_style_sheet_values;

use crate::components::FieldErrorSlot;
use crate::pages::job_create::form_state::{error_for, FieldErrors};
use crate::server::Script;

use super::FormSectionCard;

/// Skeleton placeholder for the script select while scripts load.
#[component]
fn ScriptSelectSkeleton() -> impl IntoView {
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .SelectSkeleton { width: 100%; height: 32px; }
    };

    view! {
        <style>{style_sheet}</style>
        <Skeleton>
            <SkeletonItem class=class_names.select_skeleton />
        </Skeleton>
    }
}

/// Basic job metadata fields (name + script selection).
#[component]
pub fn BasicInfoSection(
    /// Two-way signal holding the job name.
    job_name: RwSignal<String>,
    /// Two-way signal holding the selected script.
    selected_script: RwSignal<String>,
    /// Resource that loads the scripts data.
    scripts_res: Resource<Result<Vec<Script>, ServerFnError>>,
    /// Field errors from the last Create attempt.
    errors: Signal<FieldErrors>,
) -> impl IntoView {
    let name_error = Signal::derive(move || error_for(&errors.get(), "job_name"));
    let script_error = Signal::derive(move || error_for(&errors.get(), "script"));
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .SectionTitle {
            margin-bottom: 16px;
        }

        .FormField {
            display: flex;
            flex-direction: column;
            gap: 6px;
        }

        .ScriptSignature {
            font-family: monospace;
            font-size: 12px;
            color: var(--colorNeutralForeground3);
            background: var(--colorNeutralBackground3);
            padding: 8px 12px;
            border-radius: 4px;
            margin-top: 8px;
        }
    };

    view! {
        <style>{style_sheet}</style>
        <FormSectionCard>
            <Subtitle2 block=true class=class_names.section_title>"Basic Information"</Subtitle2>
            <Grid config=GridConfig::with_gaps(1, 0, 16)>
                <GridItem>
                    <div data-testid="job-name">
                        <Field label="Job name" required=true>
                            <Input appearance=InputAppearance::with_placeholder("e.g., nightly-thumbnails")
                                bind=job_name
                            />
                            <FieldErrorSlot error=name_error />
                        </Field>
                    </div>
                </GridItem>

                <GridItem>
                    <div class=class_names.form_field data-testid="job-script">
                        <Field label="Script" required=true>
                        <FieldErrorSlot error=script_error />
                        <Transition fallback=move || view! { <ScriptSelectSkeleton /> }>
                            {move || scripts_res.get().map(|r| match r {
                                Ok(scripts) => {
                                    let scripts_for_sig = scripts.clone();
                                    let selected_script_details = move || {
                                        let name = selected_script.get();
                                        scripts_for_sig.iter().find(|s| s.name == name).cloned()
                                    };
                                    view! {
                                        <>
                                            <Select bind=selected_script>
                                                <option value="">"Select a script..."</option>
                                                {scripts.iter().map(|script| {
                                                    let name = script.name.clone();
                                                    let display = format!(
                                                        "{} ({})",
                                                        script.name,
                                                        script.params.len()
                                                    );
                                                    view! {
                                                        <option value=name>{display}</option>
                                                    }
                                                }).collect_view()}
                                            </Select>

                                            {move || selected_script_details().map(|script| {
                                                view! {
                                                    <div class=class_names.script_signature>
                                                        {format!("{}(valence: Valence{})",
                                                            script.name,
                                                            if script.params.is_empty() {
                                                                String::new()
                                                            } else {
                                                                format!(", {}", script.params.iter()
                                                                    .map(|p| format!("{}: {}", p.name, p.param_type))
                                                                    .collect::<Vec<_>>()
                                                                    .join(", "))
                                                            }
                                                        )}
                                                    </div>
                                                }
                                            })}
                                        </>
                                    }.into_any()
                                }
                                Err(err) => view! {
                                    <MessageBar intent=MessageBarIntent::Error>
                                        "Failed to load scripts: " {err.to_string()}
                                    </MessageBar>
                                }.into_any(),
                            })}
                        </Transition>
                        </Field>
                    </div>
                </GridItem>
            </Grid>
        </FormSectionCard>
    }
}
