use leptos::prelude::*;
use orbital::components::{
    AutoGrid, Caption1, Skeleton, SkeletonItem, SkeletonItemSize, SpacingSize,
};
use orbital::primitives::{Card, Flex, FlexAlign, Icon};

use super::kpis::KPI_DEFS;

/// Stat card shell with a skeleton placeholder for the value only.
#[component]
pub fn ChrononStatCardSkeleton(
    /// Label text.
    label: &'static str,
    /// Icon to display.
    icon: Option<icondata_core::Icon>,
) -> impl IntoView {
    let value_skeleton = Signal::from(SkeletonItemSize::S32);
    let (style_sheet, class_names) = turf::inline_style_sheet_values! {
        .Label { color: var(--orb-color-text-tertiary); }
        .ValueSkeleton { width: 4rem; }
    };

    view! {
        <style>{style_sheet}</style>
        <Card>
            <Flex vertical=true gap=SpacingSize::Size80.flex_gap() padding=SpacingSize::Size160.inset()>
                <Flex align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                    {icon.map(|icon| view! { <Icon icon=icon /> })}
                    <Caption1 class=class_names.label>{label}</Caption1>
                </Flex>
                <Skeleton>
                    <SkeletonItem class=class_names.value_skeleton size=value_skeleton />
                </Skeleton>
            </Flex>
        </Card>
    }
}

/// Skeleton grid matching the dashboard KPI cards.
#[component]
pub fn ChrononStatsSkeleton() -> impl IntoView {
    view! {
        <AutoGrid min=super::STAT_CARD_MIN>
            {KPI_DEFS.iter().map(|def| {
                view! {
                    <div data-testid=def.test_id>
                        <ChrononStatCardSkeleton label=def.label icon=def.icon />
                    </div>
                }
            }).collect_view()}
        </AutoGrid>
    }
}
