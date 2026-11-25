use leptos::ev;
use leptos::prelude::*;
use orbital::components::{Caption1, SpacingSize, Title3};
use orbital::primitives::{
    Button, ButtonAppearance, Flex, FlexAlign, FlexJustify, FlexWrap, Menu, MenuItem, MenuTrigger,
    MessageBar, MessageBarIntent,
};

use crate::components::JobStatusBadge;
use crate::server::JobStatus;

use super::super::loaded::JobPageCtx;

/// Header for the job page: name, status, owner and script, and the actions.
///
/// Run now shows for people who can run (and never on a System-owned job); Pause or
/// Resume and the More menu show for people who can edit.
#[component]
pub fn JobDetailHeader(
    /// Shared page state.
    ctx: JobPageCtx,
    /// Run-now button loading state.
    run_now_loading: RwSignal<bool>,
    /// Run-now error message, if any.
    run_now_error: RwSignal<Option<String>>,
    /// Whether Run now is disabled.
    run_now_disabled: Memo<bool>,
    /// Called when Run now is clicked.
    on_run_now: Callback<ev::MouseEvent>,
    /// Opens Configuration's edit form.
    on_edit: Callback<()>,
) -> impl IntoView {
    let job = ctx.job;
    let access = ctx.access;
    let status = Signal::derive(move || job.with(|j| j.status));
    let system_owned = Signal::derive(move || job.with(|j| j.system_owned));
    let caption = move || {
        job.with(|j| {
            // The owner is hidden from people without Chronon admin.
            if j.owner_label.is_empty() {
                format!("Script {}", j.script_name)
            } else {
                format!("Owner: {} · Script {}", j.owner_label, j.script_name)
            }
        })
    };
    let on_more = move |value: String| match value.as_str() {
        "edit-schedule" => ctx.schedule_dialog_open.set(true),
        "edit-config" => on_edit.run(()),
        _ => {}
    };

    view! {
        <div id="chronon-job-detail-header" data-testid="job-header">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Start wrap=FlexWrap::Wrap gap=SpacingSize::Size160.flex_gap()>
                <Flex vertical=true gap=SpacingSize::Size40.flex_gap()>
                    <Flex align=FlexAlign::Center gap=SpacingSize::Size120.flex_gap()>
                        <Title3>{move || job.with(|j| j.name.clone())}</Title3>
                        {move || view! { <JobStatusBadge status=status.get() /> }}
                    </Flex>
                    <Caption1 attr:data-testid="job-header-caption">{caption}</Caption1>
                </Flex>

                <Flex align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                    <Show
                        when=move || !system_owned.get()
                        fallback=|| view! {
                            <Caption1 attr:data-testid="job-system-owned">
                                "Runs on schedule only. It belongs to the platform."
                            </Caption1>
                        }
                    >
                        <Show when=move || access.get().can_run>
                            <div id="chronon-job-detail-run-now" data-testid="run-now-button">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    icon=icondata::AiPlayCircleOutlined
                                    on_click=Callback::new(move |e| on_run_now.run(e))
                                    disabled=run_now_disabled
                                >
                                    {move || if run_now_loading.get() { "Running..." } else { "Run now" }}
                                </Button>
                            </div>
                        </Show>
                    </Show>
                    <Show when=move || access.get().can_edit>
                        <div id="chronon-job-detail-pause" data-testid="pause-resume-button">
                            <Button
                                appearance=ButtonAppearance::Secondary
                                icon=Signal::derive(move || {
                                    if status.get() == JobStatus::Active {
                                        icondata::AiPauseCircleOutlined
                                    } else {
                                        icondata::AiPlayCircleOutlined
                                    }
                                })
                                on_click=Callback::new(move |_| ctx.pause_dialog_open.set(true))
                            >
                                {move || if status.get() == JobStatus::Active { "Pause" } else { "Resume" }}
                            </Button>
                        </div>
                        <div data-testid="job-more-menu">
                            <Menu on_select=on_more>
                                <MenuTrigger slot>
                                    <Button
                                        appearance=ButtonAppearance::Subtle
                                        icon=icondata::AiMoreOutlined
                                        attr:aria-label="More actions"
                                    />
                                </MenuTrigger>
                                <MenuItem value=String::from("edit-schedule")>"Edit schedule"</MenuItem>
                                <MenuItem value=String::from("edit-config")>"Edit configuration"</MenuItem>
                            </Menu>
                        </div>
                    </Show>
                </Flex>
            </Flex>
            <Show when=move || run_now_error.get().is_some()>
                <MessageBar intent=MessageBarIntent::Error>
                    {move || format!("Failed to run job: {}", run_now_error.get().unwrap_or_default())}
                </MessageBar>
            </Show>
        </div>
    }
}
