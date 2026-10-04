use leptos::prelude::*;
use orbital::components::FormHint;
use orbital::primitives::{MessageBar, MessageBarIntent, Select};

use crate::server::{list_chronon_job_pools, ChrononPoolPickRow};

/// Worker pool `Select` over the pools the host offers.
///
/// A bound value the host no longer offers stays selectable with a note, so
/// opening an old job doesn't silently move it.
#[component]
pub fn JobPoolSelect(
    /// Two-way bound pool id.
    pool: RwSignal<String>,
    /// `data-testid` on the select wrapper.
    testid: &'static str,
) -> impl IntoView {
    let pools_res = Resource::new(|| (), |()| async { list_chronon_job_pools().await });

    view! {
        <Transition fallback=|| ()>
            {move || pools_res.get().map(|res| match res {
                Ok(pools) => {
                    let pools = with_current_pool(pools, &pool.get_untracked());
                    let hints = pools.clone();
                    view! {
                        <div data-testid=testid>
                            <Select bind=pool>
                                {pools.into_iter().map(|p| view! {
                                    <option value=p.id>{p.label}</option>
                                }).collect_view()}
                            </Select>
                        </div>
                        <FormHint>{move || {
                            let id = pool.get();
                            hints.iter().find(|p| p.id == id).map(|p| p.detail.clone()).unwrap_or_default()
                        }}</FormHint>
                    }.into_any()
                }
                Err(err) => view! {
                    <MessageBar intent=MessageBarIntent::Error>
                        "Failed to load worker pools: " {err.to_string()}
                    </MessageBar>
                }.into_any(),
            })}
        </Transition>
    }
}

fn with_current_pool(mut pools: Vec<ChrononPoolPickRow>, current: &str) -> Vec<ChrononPoolPickRow> {
    if !current.is_empty() && !pools.iter().any(|p| p.id == current) {
        pools.push(ChrononPoolPickRow {
            id: current.to_string(),
            label: current.to_string(),
            detail: "No longer offered by this host; pick another pool to move the job."
                .to_string(),
        });
    }
    pools
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str) -> ChrononPoolPickRow {
        ChrononPoolPickRow {
            id: id.into(),
            label: id.into(),
            detail: String::new(),
        }
    }

    #[test]
    fn with_current_pool_keeps_offered_list_happy_path() {
        let pools = with_current_pool(vec![row("general"), row("a")], "a");
        assert_eq!(pools.len(), 2);
    }

    #[test]
    fn with_current_pool_appends_retired_pool_sad() {
        let pools = with_current_pool(vec![row("general")], "retired");
        assert_eq!(pools.last().map(|p| p.id.as_str()), Some("retired"));
        assert!(pools[1].detail.contains("No longer offered"));
    }
}
