use leptos::prelude::*;
use orbital::components::{Caption1, Caption2};
use orbital::primitives::{
    Grid, GridConfig, GridItem, Icon, Input, InputAppearance, InputType, Label,
};
use orbital_motion::{OrbitalPresence, PresenceMotion};
use turf::inline_style_sheet_values;

use crate::components::{ChrononHelpSectionHeader, JobPoolSelect};

use super::FormSectionCard;

/// Expandable advanced job configuration inputs (concurrency, timeout, retries, worker pool).
#[component]
pub fn AdvancedOptionsSection(
    /// Two-way signal controlling whether to show advanced.
    show_advanced: RwSignal<bool>,
    /// Two-way signal holding the concurrency limit.
    concurrency: RwSignal<String>,
    /// Two-way signal holding the timeout seconds.
    timeout_seconds: RwSignal<String>,
    /// Two-way signal holding the max retries.
    max_retries: RwSignal<String>,
    /// Two-way signal holding the worker pool id.
    pool: RwSignal<String>,
) -> impl IntoView {
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .SectionTitle {
            margin-bottom: 16px;
        }

        .FormField {
            display: flex;
            flex-direction: column;
            gap: 6px;
        }

        .AdvancedToggle {
            display: flex;
            align-items: center;
            justify-content: space-between;
            cursor: pointer;
            padding: 8px 0;
        }

        .AdvancedToggle:hover {
            color: var(--colorBrandForeground1);
        }

        .Muted {
            color: var(--colorNeutralForeground3);
        }

        .ExpandedPanel {
            padding-top: 8px;
        }
    };

    let collapse_motion = Signal::from(PresenceMotion::collapse());

    view! {
            <style>{style_sheet}</style>
            <FormSectionCard>
                <div
                    class=class_names.advanced_toggle
                    on:click=move |_| show_advanced.update(|v| *v = !*v)
                >
                    <ChrononHelpSectionHeader
                        title="Advanced Options"
                        info=view! {
                            <Caption1>
                                "Concurrency limits parallel runs. Timeout stops a run after the given seconds. Max retries re-attempts failed runs. Worker pool picks which workers claim the runs."
                            </Caption1>
                        }.into_any()
                    />
                    {move || if show_advanced.get() {
                        view! { <Icon icon=icondata::AiMinusOutlined /> }
                    } else {
                        view! { <Icon icon=icondata::AiPlusOutlined /> }
                    }}
                </div>

                <Show when=move || !show_advanced.get()>
                    <Caption2 class=class_names.muted>
                        "Concurrency: " {move || concurrency.get()}
                        " | Timeout: " {move || timeout_seconds.get()} "s"
                        " | Retries: " {move || max_retries.get()}
                        " | Pool: " {move || pool.get()}
                    </Caption2>
                </Show>

                <OrbitalPresence
                    show=Signal::derive(move || show_advanced.get())
                    motion=collapse_motion
                >
                    <div class=class_names.expanded_panel>
                        <Grid config=GridConfig::with_gaps(3, 16, 16)>
                            <GridItem>
                                <div class=class_names.form_field>
                                    <Label>"Concurrency"</Label>
                                    <Input appearance=InputAppearance { input_type: Signal::from(InputType::Number), ..Default::default() }
                                        bind=concurrency
    attr:max="10"
                                    />
                                </div>
                            </GridItem>
                            <GridItem>
                                <div class=class_names.form_field>
                                    <Label>"Timeout (seconds)"</Label>
                                    <Input appearance=InputAppearance { input_type: Signal::from(InputType::Number), ..Default::default() }
                                        bind=timeout_seconds
    attr:max="3600"
                                    />
                                </div>
                            </GridItem>
                            <GridItem>
                                <div class=class_names.form_field>
                                    <Label>"Max Retries"</Label>
                                    <Input appearance=InputAppearance { input_type: Signal::from(InputType::Number), ..Default::default() }
                                        bind=max_retries
    attr:max="10"
                                    />
                                </div>
                            </GridItem>
                        </Grid>
                        <div class=class_names.form_field>
                            <Label>"Worker pool"</Label>
                            <JobPoolSelect pool=pool testid="job-pool" />
                        </div>
                    </div>
                </OrbitalPresence>
            </FormSectionCard>
        }
}
