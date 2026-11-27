use crate::RunStatus;
use leptos::prelude::*;
use orbital::primitives::{Badge, BadgeAppearance, BadgeColor};

/// Badge component for displaying run status
#[component]
pub fn RunStatusBadge(
    /// Current status value.
    #[prop(into)]
    status: RunStatus,
) -> impl IntoView {
    let (label, appearance) = match status {
        RunStatus::Pending => ("Pending", BadgeAppearance::Outline),
        RunStatus::Running => ("Running", BadgeAppearance::Tint),
        RunStatus::Completed => ("Completed", BadgeAppearance::Filled),
        RunStatus::Failed => ("Failed", BadgeAppearance::Filled),
        RunStatus::Cancelled => ("Cancelled", BadgeAppearance::Outline),
        RunStatus::Refused => ("Refused", BadgeAppearance::Tint),
    };

    let color = match status {
        RunStatus::Pending => BadgeColor::Informative,
        RunStatus::Running => BadgeColor::Brand,
        RunStatus::Completed => BadgeColor::Success,
        RunStatus::Failed => BadgeColor::Danger,
        RunStatus::Cancelled | RunStatus::Refused => BadgeColor::Warning,
    };

    view! {
        <Badge appearance=appearance color=color>{label}</Badge>
    }
}
