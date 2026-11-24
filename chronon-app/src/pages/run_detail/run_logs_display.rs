use leptos::prelude::*;
use orbital::components::{
    Body1, Caption1, Card, CardContent, CardHeader, SpacingSize, Subtitle2, Text, TextTag,
};
use orbital::primitives::{Flex, MessageBar, MessageBarBody, MessageBarIntent, MessageBarTitle};

/// Output section for a run: who may read it, the error the script stopped with, then
/// standard output and standard error. A running run shows its output so far.
#[component]
pub fn RunLogsDisplay(
    /// Captured standard output.
    #[prop(optional_no_strip)]
    logs: Option<String>,
    /// Captured standard error output.
    #[prop(optional_no_strip)]
    stderr: Option<String>,
    /// Optional error message.
    #[prop(optional_no_strip)]
    error_message: Option<String>,
    /// True while the run is still going.
    #[prop(optional)]
    running: bool,
) -> impl IntoView {
    let has_any = error_message.is_some() || logs.is_some() || stderr.is_some();

    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .LogsPre {
            font-family: var(--orb-type-family-mono);
            font-size: var(--orb-type-size-sm);
            line-height: var(--orb-type-line-md);
            padding: var(--orb-space-block-md) var(--orb-space-inline-md);
            margin: 0;
            background: var(--orb-color-surface-subtle);
            border-radius: var(--orb-radius-md);
            overflow-x: auto;
            white-space: pre-wrap;
            word-break: break-all;
        }
    };
    let pre_class = class_names.logs_pre;

    view! {
        <style>{style_sheet}</style>
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            <Subtitle2>{if running { "Output so far" } else { "Output" }}</Subtitle2>
            <Caption1>
                "Script output can hold personal data. Only people who can view this job can read it."
            </Caption1>

            {error_message.map(|message| view! {
                <MessageBar intent=MessageBarIntent::Error>
                    <MessageBarBody>
                        <MessageBarTitle>"The script stopped with an error"</MessageBarTitle>
                        {message}
                    </MessageBarBody>
                </MessageBar>
            })}

            {logs.map(|text| view! {
                <Card attr:data-testid="chronon-run-stdout">
                    <CardHeader><Subtitle2>"Standard output"</Subtitle2></CardHeader>
                    <CardContent>
                        <Text tag=TextTag::Pre class=pre_class>{text}</Text>
                    </CardContent>
                </Card>
            })}

            {stderr.map(|text| view! {
                <Card attr:data-testid="chronon-run-stderr">
                    <CardHeader><Subtitle2>"Standard error"</Subtitle2></CardHeader>
                    <CardContent>
                        <Text tag=TextTag::Pre class=pre_class>{text}</Text>
                    </CardContent>
                </Card>
            })}

            {(!has_any).then(|| view! {
                <Card>
                    <CardContent>
                        <Body1>{if running { "No output yet." } else { "No output captured." }}</Body1>
                    </CardContent>
                </Card>
            })}
        </Flex>
    }
}
