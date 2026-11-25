use crate::components::chronon_card_content;
use leptos::prelude::*;
use leptos_router::components::A;
use orbital::components::{
    Body1, Caption1, Caption2, Card, CardContent, CardHeader, CardSectionBorder, Grid, GridConfig,
    Subtitle2,
};
use orbital::primitives::{
    Button, ButtonAppearance, Flex, FlexAlign, FlexJustify, GridItem, Table, TableBody, TableCell,
    TableHeader, TableHeaderCell, TableRow,
};
use turf::inline_style_sheet_values;

use crate::pages::job_create::form_state::param_type_words;
use crate::server::{effective_job_pool, ScriptParam};

use super::super::display::{param_value_text, retries_text, timeout_text};
use super::super::loaded::JobPageCtx;

/// Type in words for a stored value whose script parameter isn't known.
fn value_type_words(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Bool(_) => "Yes or no",
        serde_json::Value::Number(n) if n.is_i64() || n.is_u64() => "Whole number",
        serde_json::Value::Number(_) => "Number",
        _ => "Text",
    }
}

/// Configuration card: script, pool, limits, retries and the job's parameters.
///
/// Parameters come from the job itself (not a redacted revision snapshot).
#[component]
#[allow(clippy::too_many_lines)]
pub fn JobInfoCard(
    /// Shared page state.
    ctx: JobPageCtx,
    /// The script's declared parameters, for their types.
    script_params: Memo<Vec<ScriptParam>>,
    /// Opens the edit form.
    on_edit: Callback<()>,
) -> impl IntoView {
    let job = ctx.job;
    let access = ctx.access;
    let (card_content_style, card_content_class) = chronon_card_content();
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .JobDetailCard {
            width: 100%;
            max-width: 100%;
            box-sizing: border-box;
        }

        .Label {
            color: var(--orb-color-text-tertiary);
        }
    };

    let rows = move || {
        let params = job.with(|j| j.params.clone());
        let declared = script_params.get();
        params
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(name, value)| {
                        let type_words = declared.iter().find(|p| &p.name == name).map_or_else(
                            || value_type_words(value),
                            |p| param_type_words(&p.param_type),
                        );
                        (name.clone(), param_value_text(value), type_words)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    view! {
        <style>{card_content_style}</style>
        <style>{style_sheet}</style>
        <div id="chronon-job-detail-config" data-testid="job-config-card">
        <Card class=class_names.job_detail_card>
            <CardHeader>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <Subtitle2>"Configuration"</Subtitle2>
                    <Show when=move || access.get().can_edit>
                        <div id="chronon-job-detail-edit" data-testid="edit-job-button">
                            <Button
                                appearance=ButtonAppearance::Subtle
                                icon=icondata::AiEditOutlined
                                on_click=Callback::new(move |_| on_edit.run(()))
                            >
                                "Edit"
                            </Button>
                        </div>
                    </Show>
                </Flex>
            </CardHeader>
            <CardSectionBorder />
            <CardContent class=card_content_class>
                <Grid config=GridConfig::with_gaps(2, 16, 8)>
                    <GridItem><Caption2 class=class_names.label>"Script"</Caption2></GridItem>
                    <GridItem>
                        <A href=crate::paths::SCRIPTS>{move || job.with(|j| j.script_name.clone())}</A>
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Worker pool"</Caption2></GridItem>
                    <GridItem>
                        <Body1 attr:data-testid="job-detail-pool">
                            {move || job.with(|j| effective_job_pool(j.pool.as_deref()).to_string())}
                        </Body1>
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Runs at once"</Caption2></GridItem>
                    <GridItem><Body1>{move || job.with(|j| j.concurrency.max(1).to_string())}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Times out after"</Caption2></GridItem>
                    <GridItem><Body1>{move || job.with(|j| timeout_text(j.timeout_ms))}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Retries"</Caption2></GridItem>
                    <GridItem>
                        <Body1>{move || job.with(|j| retries_text(j.retry_max_attempts, j.retry_base_delay_ms))}</Body1>
                    </GridItem>
                </Grid>
                <Flex vertical=true gap=orbital::components::SpacingSize::Size80.flex_gap()>
                    <Subtitle2>"Parameters"</Subtitle2>
                    {move || {
                        let rows = rows();
                        if rows.is_empty() {
                            view! { <Body1 attr:data-testid="job-params-empty">"None"</Body1> }.into_any()
                        } else {
                            view! {
                                <div data-testid="job-params-table">
                                    <Table>
                                        <TableHeader>
                                            <TableRow>
                                                <TableHeaderCell>"Name"</TableHeaderCell>
                                                <TableHeaderCell>"Value"</TableHeaderCell>
                                                <TableHeaderCell>"Type"</TableHeaderCell>
                                            </TableRow>
                                        </TableHeader>
                                        <TableBody>
                                            {rows.into_iter().map(|(name, value, kind)| view! {
                                                <TableRow>
                                                    <TableCell>{name}</TableCell>
                                                    <TableCell>{value}</TableCell>
                                                    <TableCell>{kind}</TableCell>
                                                </TableRow>
                                            }).collect_view()}
                                        </TableBody>
                                    </Table>
                                </div>
                            }.into_any()
                        }
                    }}
                    <Caption1>
                        "Values are stored with the job. Anyone who can view this job can read them."
                    </Caption1>
                </Flex>
            </CardContent>
        </Card>
        </div>
    }
}
