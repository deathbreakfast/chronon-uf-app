use crate::components::chronon_card_content;
use leptos::prelude::*;
use orbital::components::{
    Body1, Caption1, Caption2, Card, CardContent, CardHeader, CardSectionBorder, Grid, GridConfig,
    InfoLabel, InfoLabelInfo, Subtitle2, Text, TextFont, TextTag,
};
use orbital::primitives::GridItem;
use turf::inline_style_sheet_values;

/// Consolidated input for `JobInfoCard`.
#[derive(Clone, Copy)]
pub struct JobInfoCardInput {
    /// Script name to display
    pub display_script_name: Memo<String>,
    /// Cron expression to display
    pub display_cron: Memo<String>,
    /// Timezone to display
    pub display_timezone: Memo<String>,
    /// Worker pool to display
    pub display_pool: Memo<String>,
    /// Parameters JSON value to display
    pub display_params: Memo<serde_json::Value>,
    /// Last run time string
    pub last_run: RwSignal<String>,
    /// Next run time string
    pub next_run: RwSignal<String>,
}

/// Read-only metadata card showing current job configuration.
#[component]
#[allow(clippy::too_many_lines)]
pub fn JobInfoCard(
    /// Display value.
    display: JobInfoCardInput,
) -> impl IntoView {
    let JobInfoCardInput {
        display_script_name,
        display_cron,
        display_timezone,
        display_pool,
        display_params,
        last_run,
        next_run,
    } = display;

    let (card_content_style, card_content_class) = chronon_card_content();
    let (style_sheet, class_names) = inline_style_sheet_values! {
        .JobDetailCard {
            width: 100%;
            max-width: 100%;
            margin: 0 0 24px 0;
            box-sizing: border-box;
        }

        .Label {
            color: var(--orb-color-text-tertiary);
        }

        .CronCode {
            font-family: var(--orb-type-family-mono);
            color: var(--orb-color-code-fg);
            background: var(--orb-color-code-bg);
            padding: var(--orb-space-block-2xs) var(--orb-space-inline-sm);
            border-radius: var(--orb-radius-md);
        }

        .ParamsPre {
            margin: 0;
            padding: var(--orb-space-block-sm) var(--orb-space-inline-md);
            background: var(--orb-color-surface-subtle);
            border-radius: var(--orb-radius-md);
            font-family: var(--orb-type-family-mono);
            font-size: var(--orb-type-size-sm);
            line-height: var(--orb-type-line-md);
            white-space: pre-wrap;
            word-break: break-all;
            max-height: 200px;
            overflow-y: auto;
        }
    };

    view! {
        <style>{card_content_style}</style>
        <style>{style_sheet}</style>
        <div id="chronon-job-detail-config">
        <Card class=class_names.job_detail_card>
            <CardHeader>
                <InfoLabel>
                    <Subtitle2>"Configuration"</Subtitle2>
                    <InfoLabelInfo slot>
                        <Caption1>
                            "Read-only snapshot of the selected revision: script, schedule, worker pool, parameters, and recent run times."
                        </Caption1>
                    </InfoLabelInfo>
                </InfoLabel>
            </CardHeader>
            <CardSectionBorder />
            <CardContent class=card_content_class>
                <Grid config=GridConfig::with_gaps(2, 16, 8)>
                    <GridItem><Caption2 class=class_names.label>"Script"</Caption2></GridItem>
                    <GridItem><Body1>{move || display_script_name.get()}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Schedule"</Caption2></GridItem>
                    <GridItem>
                        <Text tag=TextTag::Code class=class_names.cron_code>{move || display_cron.get()}</Text>
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Timezone"</Caption2></GridItem>
                    <GridItem><Body1>{move || display_timezone.get()}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Worker pool"</Caption2></GridItem>
                    <GridItem><Body1 attr:data-testid="job-detail-pool">{move || display_pool.get()}</Body1></GridItem>

                    <GridItem>
                        <InfoLabel>
                            <Caption2 class=class_names.label>"Parameters"</Caption2>
                            <InfoLabelInfo slot>
                                <Caption1>
                                    "JSON object passed to the script at run time. Run Now uses these values as defaults."
                                </Caption1>
                            </InfoLabelInfo>
                        </InfoLabel>
                    </GridItem>
                    <GridItem>
                        {move || {
                            let params = display_params.get();
                            if params.as_object().is_some_and(serde_json::Map::is_empty) {
                                view! {
                                    <Body1>"None"</Body1>
                                }.into_any()
                            } else {
                                let params_str = serde_json::to_string_pretty(&params)
                                    .unwrap_or_else(|_| "Invalid JSON".to_string());
                                view! {
                                    <Text tag=TextTag::Pre font=TextFont::Monospace class=class_names.params_pre>
                                        {params_str}
                                    </Text>
                                }.into_any()
                            }
                        }}
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Last Run"</Caption2></GridItem>
                    <GridItem><Body1>{move || last_run.get()}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Next Run"</Caption2></GridItem>
                    <GridItem><Body1>{move || next_run.get()}</Body1></GridItem>
                </Grid>
            </CardContent>
        </Card>
        </div>
    }
}
