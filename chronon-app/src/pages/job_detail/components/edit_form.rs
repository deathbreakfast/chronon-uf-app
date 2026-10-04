use leptos::prelude::*;
use orbital::components::{Card, CardHeader, FormHint, SpacingSize, Subtitle2};
use orbital::primitives::{
    Flex, Input, InputAppearance, Label, Switch, Textarea, TextareaAppearance, TextareaResize,
};
use turf::inline_style_sheet_values;

use crate::components::JobPoolSelect;

/// Consolidated input for `JobEditForm`.
#[derive(Clone, Copy)]
pub struct JobEditFormInput {
    /// Two-way bound job name
    pub form_job_name: RwSignal<String>,
    /// Read-only script name (mirrored into `RwSignal` for Input binding)
    pub script_display_signal: RwSignal<String>,
    /// Two-way bound cron expression
    pub form_cron: RwSignal<String>,
    /// Two-way bound timezone
    pub form_timezone: RwSignal<String>,
    /// Two-way bound parameters JSON string
    pub form_params_str: RwSignal<String>,
    /// Two-way bound enabled state
    pub form_enabled: RwSignal<bool>,
    /// Two-way bound worker pool id
    pub form_pool: RwSignal<String>,
}

/// Edit form card for modifying job configuration.
///
/// Shown when the user enters edit mode. Uses a `Card` with `CardHeader`
/// and `Flex` layout for form fields.
#[component]
pub fn JobEditForm(
    /// Form state.
    form: JobEditFormInput,
) -> impl IntoView {
    let JobEditFormInput {
        form_job_name,
        script_display_signal,
        form_cron,
        form_timezone,
        form_params_str,
        form_enabled,
        form_pool,
    } = form;

    let params_textarea_appearance = TextareaAppearance {
        placeholder: MaybeProp::from("{}"),
        resize: Signal::from(TextareaResize::Vertical),
        ..Default::default()
    };

    let (style_sheet, class_names) = inline_style_sheet_values! {
        .JobDetailCard {
            width: 100%;
            max-width: 100%;
            margin: 0 0 24px 0;
            box-sizing: border-box;
        }

        .Content {
            padding: 0 16px 16px 16px;
        }

    };

    view! {
        <style>{style_sheet}</style>
        <Card class=class_names.job_detail_card>
            <CardHeader>
                <Subtitle2>"Edit Job"</Subtitle2>
            </CardHeader>
            <Flex vertical=true gap=SpacingSize::Size160.flex_gap() class=class_names.content>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Job Name"</Label>
                    <Input bind=form_job_name />
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Script"</Label>
                    <Input appearance=InputAppearance { disabled: Signal::from(true), ..Default::default() } bind=script_display_signal  />
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Cron Expression"</Label>
                    <Input bind=form_cron />
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Timezone"</Label>
                    <Input appearance=InputAppearance::with_placeholder("UTC") bind=form_timezone />
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Worker pool"</Label>
                    <JobPoolSelect pool=form_pool testid="job-edit-pool" />
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Parameters (JSON)"</Label>
                    <Textarea bind=form_params_str appearance=params_textarea_appearance />
                    <FormHint>
                        "Enter parameters as JSON object (e.g., {\"key\": \"value\"})"
                    </FormHint>
                </Flex>
                <Flex vertical=true gap=SpacingSize::Size60.flex_gap()>
                    <Label>"Enabled"</Label>
                    <Switch bind=form_enabled />
                </Flex>
            </Flex>
        </Card>
    }
}
