use std::collections::BTreeMap;

use leptos::prelude::*;
use orbital_charts::{AxisDef, ChartType, LineChart, ScaleType, SeriesDef};

use crate::server::DashboardChartSeries;

fn bucket_label(ts: chrono::DateTime<chrono::Utc>, use_daily: bool) -> String {
    if use_daily {
        ts.format("%m/%d").to_string()
    } else {
        format!("{}:{}", ts.format("%H"), ts.format("%M"))
    }
}

pub fn line_chart_from_series(
    series: &[DashboardChartSeries],
    height: f64,
    use_daily_labels: bool,
) -> impl IntoView {
    let mut ts_labels: BTreeMap<chrono::DateTime<chrono::Utc>, String> = BTreeMap::new();
    for s in series {
        for p in &s.points {
            ts_labels
                .entry(p.ts)
                .or_insert_with(|| bucket_label(p.ts, use_daily_labels));
        }
    }
    let ts_order: Vec<_> = ts_labels.keys().copied().collect();
    let categories: Vec<String> = ts_labels.values().cloned().collect();

    let chart_series: Vec<SeriesDef> = series
        .iter()
        .map(|s| {
            let map: BTreeMap<_, _> = s.points.iter().map(|p| (p.ts, p.value)).collect();
            let data: Vec<f64> = ts_order
                .iter()
                .map(|ts| map.get(ts).copied().unwrap_or(f64::NAN))
                .collect();
            SeriesDef {
                id: s.id.clone(),
                label: Some(s.label.clone()),
                data: Some(data),
                chart_type: Some(ChartType::Line),
                connect_nulls: Some(true),
                show_markers: Some(false),
                ..Default::default()
            }
        })
        .collect();

    let x_axis = vec![AxisDef {
        id: "x".into(),
        scale_type: ScaleType::Band,
        data: Some(categories),
        ..Default::default()
    }];

    view! {
        <LineChart series=chart_series x_axis=x_axis height=height />
    }
}
