use leptos::ev;
use leptos::prelude::*;
use orbital::components::{Card, CardHeader, SpacingSize, Subtitle2};
use orbital::primitives::{
    Button, ButtonAppearance, Field, Flex, Input, InputAppearance, MessageBar, MessageBarIntent,
};
use turf::inline_style_sheet_values;

use crate::components::JobPoolSelect;
use crate::pages::job_create::components::TypedParamFields;
use crate::server::ScriptParam;

use super::super::actions::JobDetailFormState;
use super::super::loaded::JobPageCtx;

/// Edit form for the job's name, worker pool and parameters (one typed field each).
///
/// The schedule changes through Edit schedule and the state through Pause or Resume.
#[component]
pub fn JobEditForm(
    /// Form state.
    form: JobDetailFormState,
    /// Shared page state.
    ctx: JobPageCtx,
    /// The script's declared parameters.
    script_params: Memo<Vec<ScriptParam>>,
    /// Save in progress.
    save_loading: RwSignal<bool>,
    /// Save error, if any.
    save_error: RwSignal<Option<String>>,
    /// Save click.
    on_save: Callback<ev::MouseEvent>,
    /// Cancel click.
    on_cancel: Callback<ev::MouseEvent>,
) -> impl IntoView {
    let script_name = RwSignal::new(ctx.job.with_untracked(|j| j.script_name.clone()));
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .Content {
            padding: 0 16px 16px 16px;
        }
    };

    view! {
        <style>{style_sheet}</style>
        <div data-testid="job-edit-form">
        <Card>
            <CardHeader>
                <Subtitle2>"Edit configuration"</Subtitle2>
            </CardHeader>
            <Flex vertical=true gap=SpacingSize::Size160.flex_gap() class=class_names.content>
                <Field label="Job name">
                    <Input bind=form.job_name />
                </Field>
                <Field label="Script">
                    <Input
                        appearance=InputAppearance { disabled: Signal::from(true), ..Default::default() }
                        bind=script_name
                    />
                </Field>
                <Field label="Worker pool">
                    <JobPoolSelect pool=form.pool testid="job-edit-pool" />
                </Field>
                <TypedParamFields
                    params=Signal::derive(move || script_params.get())
                    values=form.params
                    errors=form.param_errors
                />
                <Show when=move || save_error.get().is_some()>
                    <MessageBar intent=MessageBarIntent::Error>
                        {move || save_error.get().unwrap_or_default()}
                    </MessageBar>
                </Show>
                <Flex gap=SpacingSize::Size120.flex_gap()>
                    <div id="chronon-job-detail-save" data-testid="save-job-button">
                        <Button
                            appearance=ButtonAppearance::Primary
                            disabled=save_loading
                            on_click=Callback::new(move |e| on_save.run(e))
                        >
                            {move || if save_loading.get() { "Saving..." } else { "Save" }}
                        </Button>
                    </div>
                    <div id="chronon-job-detail-cancel" data-testid="cancel-edit-button">
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=save_loading
                            on_click=Callback::new(move |e| on_cancel.run(e))
                        >
                            "Cancel"
                        </Button>
                    </div>
                </Flex>
            </Flex>
        </Card>
        </div>
    }
}
