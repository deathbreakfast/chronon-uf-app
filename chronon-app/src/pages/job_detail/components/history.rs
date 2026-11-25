//! The job page's History tab: every revision with who saved it and what changed, and a diff
//! of the selected revision against the one before.

use leptos::prelude::*;
use orbital::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, CardHeader, Skeleton, SkeletonItem,
    SkeletonItemSize, SpacingSize, Subtitle2,
};
use orbital::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, DiffLayout, DiffView, Flex, FlexAlign,
    MessageBar, MessageBarBody, MessageBarIntent, Table, TableBody, TableCell, TableCellLayout,
    TableHeader, TableHeaderCell, TableRow,
};

use super::super::loaded::JobPageCtx;
use crate::server::{get_job_revisions, JobRevision};

/// `Oct 27, 10:05` (UTC) for an RFC 3339 time.
fn saved_at_label(changed_at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(changed_at).map_or_else(
        |_| changed_at.to_string(),
        |t| {
            t.with_timezone(&chrono::Utc)
                .format("%b %-d, %H:%M")
                .to_string()
        },
    )
}

/// History tab body. Revisions load once per saved revision of the job; picking a row only
/// changes which revision the diff shows, so nothing reloads.
#[allow(clippy::too_many_lines)]
#[component]
pub fn JobHistory(
    /// Shared job page state (job id, current revision, access).
    ctx: JobPageCtx,
) -> impl IntoView {
    let revisions = RwSignal::new(None::<Result<Vec<JobRevision>, String>>);
    // `None` means the current (newest) revision.
    let selected = RwSignal::new(None::<u32>);

    // Reload when the job gets a new revision (a save on this page).
    let load_key = Memo::new(move |_| ctx.job.with(|j| (j.id.clone(), j.revision)));
    Effect::new(move |_| {
        let (id, _) = load_key.get();
        leptos::task::spawn_local_scoped(async move {
            let result = get_job_revisions(id).await.map_err(|e| e.to_string());
            selected.set(None);
            revisions.set(Some(result));
        });
    });

    let rows = Memo::new(move |_| revisions.get().and_then(Result::ok).unwrap_or_default());
    let current = Memo::new(move |_| rows.with(|r| r.first().map(|r| r.revision_number)));
    let shown = Memo::new(move |_| selected.get().or_else(|| current.get()));
    // The shown revision and the one before it.
    let pair = Memo::new(move |_| {
        let number = shown.get()?;
        rows.with(|r| {
            let at = r.iter().position(|rev| rev.revision_number == number)?;
            Some((r[at].clone(), r.get(at + 1).cloned()))
        })
    });

    let skeleton_size = Signal::from(SkeletonItemSize::S16);
    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .Row { cursor: pointer; }
        .Row:hover { background: var(--colorNeutralBackground1Hover); }
        .Selected { background: var(--colorNeutralBackground1Selected); }
    };
    let row_class = StoredValue::new(class_names.row.to_string());
    let selected_class = StoredValue::new(format!("{} {}", class_names.row, class_names.selected));

    view! {
        <style>{style_sheet}</style>
        <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
            <div data-testid="job-history-card">
            <Card>
                <CardHeader>
                    <Subtitle2>"History"</Subtitle2>
                </CardHeader>
                <CardContent>
                    <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                        {move || revisions.get().and_then(Result::err).map(|e| {
                            // Revisions need ChrononAdmin today; say so rather than show the raw refusal.
                            if ctx.access.with(|a| a.can_edit) {
                                view! {
                                    <MessageBar intent=MessageBarIntent::Error>
                                        <MessageBarBody>{format!("Couldn't load the revisions: {e}")}</MessageBarBody>
                                    </MessageBar>
                                }.into_any()
                            } else {
                                view! {
                                    <MessageBar intent=MessageBarIntent::Info>
                                        <MessageBarBody>
                                            "Only Chronon admins can see this job's revisions."
                                        </MessageBarBody>
                                    </MessageBar>
                                }.into_any()
                            }
                        })}
                        <div data-testid="job-history-table">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"Revision"</TableHeaderCell>
                                    <TableHeaderCell>"Saved by"</TableHeaderCell>
                                    <TableHeaderCell>"When"</TableHeaderCell>
                                    <TableHeaderCell>"What changed"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <Show
                                    when=move || revisions.with(Option::is_some)
                                    fallback=move || view! {
                                        <Skeleton>
                                            {(0..2).map(|_| view! {
                                                <TableRow>
                                                    {(0..4).map(|_| view! {
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                <SkeletonItem size=skeleton_size />
                                                            </TableCellLayout>
                                                        </TableCell>
                                                    }).collect_view()}
                                                </TableRow>
                                            }).collect_view()}
                                        </Skeleton>
                                    }
                                >
                                    <For
                                        each=move || rows.get()
                                        key=|rev| rev.revision_id.clone()
                                        let:rev
                                    >
                                        {
                                            let number = rev.revision_number;
                                            let is_shown = move || shown.get() == Some(number);
                                            let class = Signal::derive(move || {
                                                if is_shown() { selected_class.get_value() } else { row_class.get_value() }
                                            });
                                            let select = move || selected.set(Some(number));
                                            view! {
                                                <TableRow
                                                    class=class
                                                    attr:data-testid="job-history-row"
                                                    attr:data-revision=number.to_string()
                                                    attr:data-selected=move || is_shown().to_string()
                                                    attr:aria-selected=move || is_shown().to_string()
                                                    attr:tabindex="0"
                                                    on:click=move |_| select()
                                                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                                        if ev.key() == "Enter" || ev.key() == " " {
                                                            ev.prevent_default();
                                                            select();
                                                        }
                                                    }
                                                >
                                                    <TableCell>
                                                        <Flex align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                                                            <Body1Strong>{format!("Revision {number}")}</Body1Strong>
                                                            <Show when=move || current.get() == Some(number)>
                                                                <span data-testid="job-history-current">
                                                                    <Badge
                                                                        appearance=BadgeAppearance::Tint
                                                                        color=BadgeColor::Brand
                                                                        size=BadgeSize::Small
                                                                    >
                                                                        "Current"
                                                                    </Badge>
                                                                </span>
                                                            </Show>
                                                        </Flex>
                                                    </TableCell>
                                                    <TableCell><Body1>{rev.saved_by.clone()}</Body1></TableCell>
                                                    <TableCell><Body1>{saved_at_label(&rev.changed_at)}</Body1></TableCell>
                                                    <TableCell><Caption1>{rev.summary.clone()}</Caption1></TableCell>
                                                </TableRow>
                                            }
                                        }
                                    </For>
                                </Show>
                            </TableBody>
                        </Table>
                        </div>
                        <Caption1>"Every save makes a revision. Older revisions can't be edited."</Caption1>
                    </Flex>
                </CardContent>
            </Card>
            </div>

            {move || pair.get().map(|(rev, before)| view! { <RevisionDiffCard rev=rev before=before /> })}
        </Flex>
    }
}

/// The selected revision compared with the one before it (or with nothing, for the first).
#[component]
fn RevisionDiffCard(rev: JobRevision, before: Option<JobRevision>) -> impl IntoView {
    let number = rev.revision_number;
    let (title, from_label) = before.as_ref().map_or_else(
        || {
            (
                format!("Revision {number}, as the job was created"),
                "Empty".to_string(),
            )
        },
        |b| {
            (
                format!(
                    "Revision {number} compared with revision {}",
                    b.revision_number
                ),
                format!("Revision {}", b.revision_number),
            )
        },
    );
    let before_text = before.map(|b| b.settings_text).unwrap_or_default();
    view! {
        <div data-testid="job-history-diff-card">
        <Card>
            <CardHeader>
                <span data-testid="job-history-diff-title">
                    <Subtitle2>{title}</Subtitle2>
                </span>
            </CardHeader>
            <CardContent>
                <DiffView
                    before=before_text
                    after=rev.settings_text
                    layout=DiffLayout::Unified
                    from_label=from_label
                    to_label=format!("Revision {number}")
                    identical_message="This save didn't change any setting."
                />
            </CardContent>
        </Card>
        </div>
    }
}
