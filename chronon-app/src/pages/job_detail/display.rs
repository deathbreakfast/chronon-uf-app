//! Display helpers for the job page: messages, configuration text, edit seeds.

use std::collections::HashMap;

use leptos::prelude::*;

use crate::components::format_run_time;
use crate::server::{Job, ScriptParam};

/// Provides Chronon default timezone label for empty values.
pub fn display_timezone(timezone: &str) -> String {
    if timezone.is_empty() {
        "UTC".to_string()
    } else {
        timezone.to_string()
    }
}

/// Message shown on the job page right after the create form saves it.
pub fn created_message(job: &Job) -> String {
    job.next_run_at.as_deref().map_or_else(
        || {
            format!(
                "{} is saved. It runs only when someone uses Run now.",
                job.name
            )
        },
        |next| format!("{} first runs {}.", job.name, format_run_time(next)),
    )
}

/// A stored parameter value as the text a field shows.
pub fn param_value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// One field signal per script parameter, seeded from the job's stored values.
pub fn param_signals_for_edit(
    params: &[ScriptParam],
    current: &serde_json::Value,
) -> HashMap<String, RwSignal<String>> {
    params
        .iter()
        .map(|p| {
            let value = current
                .get(&p.name)
                .map(param_value_text)
                .or_else(|| p.default.clone())
                .unwrap_or_default();
            (p.name.clone(), RwSignal::new(value))
        })
        .collect()
}

/// `Up to 3, 30 s apart`, or `None` when retries are off.
pub fn retries_text(max_attempts: u32, base_delay_ms: u64) -> String {
    if max_attempts == 0 {
        return "None".to_string();
    }
    let secs = base_delay_ms / 1000;
    if secs == 0 {
        format!("Up to {max_attempts}")
    } else {
        format!("Up to {max_attempts}, {secs} s apart")
    }
}

/// `5 min`, `45 s`, or `No limit`.
pub fn timeout_text(timeout_ms: Option<u64>) -> String {
    match timeout_ms.map(|ms| ms / 1000) {
        None | Some(0) => "No limit".to_string(),
        Some(s) if s % 60 == 0 => format!("{} min", s / 60),
        Some(s) => format!("{s} s"),
    }
}

#[cfg(test)]
mod tests {
    use super::{display_timezone, param_value_text, retries_text, timeout_text};

    #[test]
    fn display_timezone_defaults_to_utc() {
        assert_eq!(display_timezone(""), "UTC");
        assert_eq!(
            display_timezone("America/Los_Angeles"),
            "America/Los_Angeles"
        );
    }

    #[test]
    fn configuration_text() {
        assert_eq!(retries_text(3, 30_000), "Up to 3, 30 s apart");
        assert_eq!(retries_text(0, 1_000), "None");
        assert_eq!(timeout_text(Some(300_000)), "5 min");
        assert_eq!(timeout_text(Some(45_000)), "45 s");
        assert_eq!(timeout_text(None), "No limit");
        assert_eq!(param_value_text(&serde_json::json!("prod")), "prod");
        assert_eq!(param_value_text(&serde_json::json!(800)), "800");
    }
}
