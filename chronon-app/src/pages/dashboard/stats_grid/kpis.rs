use leptos::prelude::*;
use orbital::components::StatCardVariant;

use crate::server::DashboardStats;

/// Static definition for one dashboard KPI card.
pub struct DashboardKpiDef {
    pub label: &'static str,
    pub icon: Option<icondata_core::Icon>,
    pub variant: StatCardVariant,
    pub test_id: &'static str,
    pub select: fn(&DashboardStats) -> String,
}

/// `96.6%` from tenths of a percent, or a dash when no run finished.
fn worked_share(permille: Option<u32>) -> String {
    permille.map_or_else(
        || "\u{2014}".to_string(),
        |p| format!("{}.{}%", p / 10, p % 10),
    )
}

pub const KPI_DEFS: &[DashboardKpiDef] = &[
    DashboardKpiDef {
        label: "Failing jobs",
        icon: Some(icondata::AiWarningOutlined),
        variant: StatCardVariant::Danger,
        test_id: "chronon-stat-failing-jobs",
        select: |s| s.failing_jobs.to_string(),
    },
    DashboardKpiDef {
        label: "Runs today",
        icon: Some(icondata::AiClockCircleOutlined),
        variant: StatCardVariant::Default,
        test_id: "chronon-stat-runs-today",
        select: |s| s.total_runs_today.to_string(),
    },
    DashboardKpiDef {
        label: "Worked (24h)",
        icon: Some(icondata::AiCheckCircleOutlined),
        variant: StatCardVariant::Success,
        test_id: "chronon-stat-worked-24h",
        select: |s| worked_share(s.worked_24h_permille),
    },
    DashboardKpiDef {
        label: "Running now",
        icon: Some(icondata::AiReloadOutlined),
        variant: StatCardVariant::Default,
        test_id: "chronon-stat-running-now",
        select: |s| s.running_now.to_string(),
    },
    DashboardKpiDef {
        label: "Paused",
        icon: None,
        variant: StatCardVariant::Warning,
        test_id: "chronon-stat-paused",
        select: |s| s.paused_jobs.to_string(),
    },
];

/// Memoized string values for each KPI, updating in place on resource refetch.
pub fn dashboard_stat_memos(
    stats_res: Resource<Result<DashboardStats, ServerFnError>>,
) -> Vec<Memo<String>> {
    KPI_DEFS
        .iter()
        .map(|def| {
            let select = def.select;
            Memo::new(move |_| {
                stats_res
                    .get()
                    .and_then(Result::ok)
                    .map(|s| select(&s))
                    .unwrap_or_default()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::worked_share;

    #[test]
    fn worked_share_one_decimal() {
        assert_eq!(worked_share(Some(966)), "96.6%");
        assert_eq!(worked_share(Some(1000)), "100.0%");
        assert_eq!(worked_share(Some(5)), "0.5%");
        assert_eq!(worked_share(None), "\u{2014}");
    }
}
