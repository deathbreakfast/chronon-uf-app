use leptos::prelude::*;
use orbital::components::{Caption1, FormHint};
use orbital::primitives::{Label, MessageBar, MessageBarIntent, Select};

use crate::components::ChrononHelpSectionHeader;
use crate::server::get_owned_by_options;

use super::FormSectionCard;

/// "Owned by": the group that will own the new job alongside its creator.
///
/// Lists the creator's teams, the Chronon admin group and "Only me" (an empty value). It
/// starts on the creator's team when they're on exactly one, otherwise the admin group.
#[component]
pub fn OwnedBySection(
    /// Two-way bound group id; empty means only the creator.
    owned_by: RwSignal<String>,
) -> impl IntoView {
    let options_res = Resource::new(|| (), |()| async { get_owned_by_options().await });
    // Start on the server's default once, without overwriting a later pick.
    let seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if let Some(Ok(owned)) = options_res.get() {
            if !seeded.get_value() {
                seeded.set_value(true);
                owned_by.set(owned.default_group_id.unwrap_or_default());
            }
        }
    });

    view! {
        <FormSectionCard>
            <ChrononHelpSectionHeader
                title="Owned by"
                info=view! {
                    <Caption1>
                        "Owners decide who can view, run and change this job. You stay an owner either way."
                    </Caption1>
                }.into_any()
            />
            <Transition fallback=|| ()>
                {move || options_res.get().map(|res| match res {
                    Ok(owned) => {
                        view! {
                            <Label>"Owner group"</Label>
                            <div data-testid="job-owned-by">
                                <Select bind=owned_by>
                                    {owned.options.into_iter().map(|o| view! {
                                        <option value=o.group_id.unwrap_or_default()>{o.label}</option>
                                    }).collect_view()}
                                </Select>
                            </div>
                            <FormHint>"You can change who has access later on the job's Access tab."</FormHint>
                        }.into_any()
                    }
                    Err(err) => view! {
                        <MessageBar intent=MessageBarIntent::Error>
                            "Failed to load owner choices: " {err.to_string()}
                        </MessageBar>
                    }.into_any(),
                })}
            </Transition>
        </FormSectionCard>
    }
}
