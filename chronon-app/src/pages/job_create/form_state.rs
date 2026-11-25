use std::collections::HashMap;

use leptos::prelude::*;

use crate::server::ScriptParam;

/// Builds the JSON params object for job creation from form string values.
#[cfg(test)]
pub fn build_create_params(
    params: &[ScriptParam],
    values: &HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    let mut object = serde_json::Map::new();

    for param in params {
        let raw = values.get(&param.name).map_or("", String::as_str);
        let trimmed = raw.trim();

        if param.required && trimmed.is_empty() {
            return Err(format!("Parameter '{}' is required.", param.name));
        }

        if trimmed.is_empty() {
            continue;
        }

        object.insert(
            param.name.clone(),
            coerce_param_value(&param.param_type, trimmed)?,
        );
    }

    Ok(serde_json::Value::Object(object))
}

fn coerce_param_value(param_type: &str, raw: &str) -> Result<serde_json::Value, String> {
    match param_type {
        "bool" => raw
            .parse::<bool>()
            .map(serde_json::Value::Bool)
            .map_err(|_| format!("Choose yes or no. You entered {raw}.")),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => raw
            .parse::<i64>()
            .map(serde_json::Value::from)
            .map_err(|_| format!("Enter a whole number, such as 800. You entered {raw}.")),
        "f32" | "f64" => raw
            .parse::<f64>()
            .map(serde_json::Value::from)
            .map_err(|_| format!("Enter a number, such as 1.5. You entered {raw}.")),
        _ => Ok(serde_json::Value::String(raw.to_string())),
    }
}

/// A script parameter's type in words: "Whole number", "Number", "Yes or no", "Text".
#[must_use]
pub fn param_type_words(param_type: &str) -> &'static str {
    match param_type {
        "bool" => "Yes or no",
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            "Whole number"
        }
        "f32" | "f64" => "Number",
        _ => "Text",
    }
}

/// Like [`build_create_params`], but checks every parameter and returns each error
/// keyed by parameter name.
pub fn build_create_params_by_field(
    params: &[ScriptParam],
    values: &HashMap<String, String>,
) -> Result<serde_json::Value, HashMap<String, String>> {
    let mut object = serde_json::Map::new();
    let mut errors = HashMap::new();
    for param in params {
        let raw = values.get(&param.name).map_or("", String::as_str).trim();
        if raw.is_empty() {
            if param.required {
                errors.insert(
                    param.name.clone(),
                    format!("Enter a value for {}.", param.name),
                );
            }
            continue;
        }
        match coerce_param_value(&param.param_type, raw) {
            Ok(value) => {
                object.insert(param.name.clone(), value);
            }
            Err(message) => {
                errors.insert(param.name.clone(), message);
            }
        }
    }
    if errors.is_empty() {
        Ok(serde_json::Value::Object(object))
    } else {
        Err(errors)
    }
}

/// UTC instant for a run-once date (`YYYY-MM-DD`), time (`HH:MM`) and IANA timezone.
#[must_use]
pub fn run_once_instant(date: &str, time: &str, tz: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::TimeZone;
    let date = chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()?;
    let time = chrono::NaiveTime::parse_from_str(time.trim(), "%H:%M").ok()?;
    let tz: chrono_tz::Tz = if tz.trim().is_empty() {
        chrono_tz::UTC
    } else {
        tz.trim().parse().ok()?
    };
    tz.from_local_datetime(&date.and_time(time))
        .earliest()
        .map(|t| t.with_timezone(&chrono::Utc))
}

/// Every create-form problem, keyed by field (`job_name`, `script`, `cron`, `timezone`,
/// `run_once`, or `param:<name>`), in the order the form shows them.
pub type FieldErrors = Vec<(String, String)>;

/// Summary line for the top of the form: "1 field needs a look" / "2 fields need a look".
#[must_use]
pub fn errors_summary(errors: &FieldErrors) -> String {
    match errors.len() {
        0 => String::new(),
        1 => "1 field needs a look".to_string(),
        n => format!("{n} fields need a look"),
    }
}

/// Error for one field, if any.
#[must_use]
pub fn error_for(errors: &FieldErrors, key: &str) -> Option<String> {
    errors
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, m)| m.clone())
}

/// Builds per-param signals seeded from script defaults when the selected script changes.
pub fn seed_param_signals(params: &[ScriptParam]) -> HashMap<String, RwSignal<String>> {
    params
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                RwSignal::new(p.default.clone().unwrap_or_default()),
            )
        })
        .collect()
}

/// Collects current string values from param signals for submission.
pub fn collect_param_values(
    signals: &HashMap<String, RwSignal<String>>,
) -> HashMap<String, String> {
    signals
        .iter()
        .map(|(name, signal)| (name.clone(), signal.get()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        build_create_params, build_create_params_by_field, coerce_param_value, errors_summary,
    };

    #[test]
    fn run_once_instant_converts_to_utc() {
        let at = super::run_once_instant("2026-10-28", "02:00", "UTC").unwrap();
        assert_eq!(at.to_rfc3339(), "2026-10-28T02:00:00+00:00");
        let la = super::run_once_instant("2026-10-28", "02:00", "America/Los_Angeles").unwrap();
        assert_eq!(la.to_rfc3339(), "2026-10-28T09:00:00+00:00");
        assert!(super::run_once_instant("", "02:00", "UTC").is_none());
        assert!(super::run_once_instant("2026-10-28", "02:00", "Nowhere").is_none());
    }

    #[test]
    fn build_create_params_by_field_reports_every_error() {
        let defs = vec![
            param("bucket", "String", true, None),
            param("max_width", "i64", true, None),
        ];
        let mut values = std::collections::HashMap::new();
        values.insert("max_width".to_string(), "wide".to_string());
        let errors = build_create_params_by_field(&defs, &values).expect_err("two errors");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors["bucket"], "Enter a value for bucket.");
        assert_eq!(
            errors["max_width"],
            "Enter a whole number, such as 800. You entered wide."
        );
        assert_eq!(
            errors_summary(&vec![("a".into(), "x".into()), ("b".into(), "y".into())]),
            "2 fields need a look"
        );
    }
    use crate::server::ScriptParam;

    fn param(name: &str, param_type: &str, required: bool, default: Option<&str>) -> ScriptParam {
        ScriptParam {
            name: name.to_string(),
            param_type: param_type.to_string(),
            default: default.map(str::to_string),
            required,
        }
    }

    #[test]
    fn build_create_params_rejects_missing_required() {
        let defs = vec![param("limit", "i32", true, None)];
        let values = std::collections::HashMap::new();
        let err = build_create_params(&defs, &values).expect_err("required param missing");
        assert!(err.contains("limit"));
    }

    #[test]
    fn build_create_params_coerces_bool_and_number() {
        let defs = vec![
            param("force", "bool", false, None),
            param("retain_days", "i32", false, None),
        ];
        let mut values = std::collections::HashMap::new();
        values.insert("force".to_string(), "true".to_string());
        values.insert("retain_days".to_string(), "7".to_string());

        let json = build_create_params(&defs, &values).expect("valid params");
        assert_eq!(json, serde_json::json!({"force": true, "retain_days": 7}));
    }

    #[test]
    fn build_create_params_empty_script_returns_empty_object() {
        let json = build_create_params(&[], &std::collections::HashMap::new()).expect("empty");
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn coerce_param_value_parses_bool() {
        assert_eq!(
            coerce_param_value("bool", "false").unwrap(),
            serde_json::json!(false)
        );
    }

    #[test]
    fn coerce_param_value_rejects_invalid_bool_and_int() {
        assert!(coerce_param_value("bool", "yes").is_err());
        assert!(coerce_param_value("i32", "1.5").is_err());
        assert!(coerce_param_value("f64", "nope").is_err());
    }

    #[test]
    fn build_create_params_skips_empty_optional() {
        let defs = vec![
            param("limit", "i32", false, None),
            param("label", "String", false, None),
        ];
        let mut values = std::collections::HashMap::new();
        values.insert("limit".to_string(), "  ".to_string());
        values.insert("label".to_string(), "ok".to_string());
        let json = build_create_params(&defs, &values).expect("optional empty skipped");
        assert_eq!(json, serde_json::json!({"label": "ok"}));
    }
}
