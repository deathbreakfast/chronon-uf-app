use leptos::prelude::*;
use orbital::components::{Body1, Caption2, Card, CardContent};
use orbital::primitives::{Grid, GridConfig, GridItem, Link};

use crate::server::RunDetail;

/// `2026-11-24 03:00:12 UTC` from an RFC 3339 string; the raw string when it doesn't parse.
fn time_text(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339).map_or_else(
        |_| rfc3339.to_string(),
        |t| {
            t.with_timezone(&chrono::Utc)
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
        },
    )
}

/// `older_than_days: 14 (stored: 30)` lines: the values this run used, and the stored value
/// where it differed.
fn parameter_lines(used: &serde_json::Value, stored: &serde_json::Value) -> Vec<String> {
    let Some(map) = used.as_object().filter(|m| !m.is_empty()) else {
        return vec!["None".to_string()];
    };
    map.iter()
        .map(|(name, value)| {
            let now = chronon_backend::param_field_text(Some(value));
            let kept = stored.get(name);
            match kept {
                Some(kept) if kept != value => format!(
                    "{name}: {now} (stored: {})",
                    chronon_backend::param_field_text(Some(kept))
                ),
                _ => format!("{name}: {now}"),
            }
        })
        .collect()
}

/// Facts grid for a run: job, who started it, timing, the parameters it used, its worker
/// pool, and the run id last.
#[component]
pub fn RunInfoGrid(
    /// The run to describe.
    detail: RunDetail,
) -> impl IntoView {
    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .Label { color: var(--colorNeutralForeground3); }
    };
    let run = detail.run;
    let job_href = chronon_backend::chronon_job_path(&run.job_id);
    let finished = run
        .finished_at
        .as_deref()
        .map_or_else(|| "Not finished yet".to_string(), time_text);
    let duration = run
        .duration_ms
        .map_or_else(|| "\u{2014}".to_string(), chronon_backend::duration_words);
    let params = parameter_lines(&detail.params, &detail.stored_params);

    view! {
        <style>{style_sheet}</style>
        <div data-testid="chronon-run-facts">
        <Card>
            <CardContent>
                <Grid config=GridConfig::with_gaps(2, 16, 8)>
                    <GridItem><Caption2 class=class_names.label>"Job"</Caption2></GridItem>
                    <GridItem>
                        <div data-testid="run-job-link"><Link href=job_href>{run.job_name}</Link></div>
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Started by"</Caption2></GridItem>
                    <GridItem><Body1 attr:data-testid="run-started-by">{detail.started_by_label}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Started"</Caption2></GridItem>
                    <GridItem><Body1>{time_text(&run.started_at)}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Finished"</Caption2></GridItem>
                    <GridItem><Body1>{finished}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Duration"</Caption2></GridItem>
                    <GridItem><Body1>{duration}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Parameters"</Caption2></GridItem>
                    <GridItem>
                        <div data-testid="run-parameters">
                            {params.into_iter().map(|line| view! { <Body1>{line}</Body1><br /> }).collect_view()}
                        </div>
                    </GridItem>

                    <GridItem><Caption2 class=class_names.label>"Worker pool"</Caption2></GridItem>
                    <GridItem><Body1>{detail.pool}</Body1></GridItem>

                    <GridItem><Caption2 class=class_names.label>"Run id"</Caption2></GridItem>
                    <GridItem><Body1>{run.id}</Body1></GridItem>
                </Grid>
            </CardContent>
        </Card>
        </div>
    }
}
