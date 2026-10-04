//! Shared SSR helpers for Chronon server functions.

use leptos::prelude::*;
#[cfg(feature = "ssr")]
use std::collections::HashMap;

#[cfg(feature = "ssr")]
pub(super) use chronon_backend::{chronon_run_status_to_ui, model_run_to_app_run};

#[cfg(feature = "ssr")]
pub(super) fn script_registry(
) -> Result<std::sync::Arc<chronon_coordinator::ScriptRegistry>, ServerFnError> {
    if let Some(cfg) = leptos::context::use_context::<std::sync::Arc<higgs::HiggsConfig>>() {
        if let Ok(registry) = cfg.script_registry_arc() {
            return Ok(registry);
        }
    }
    leptos::context::use_context::<std::sync::Arc<chronon_coordinator::ScriptRegistry>>()
        .ok_or_else(|| ServerFnError::new("Script registry not in request context"))
}

#[cfg(feature = "ssr")]
pub(super) fn chronon_backend(
) -> Result<std::sync::Arc<dyn chronon_coordinator::ChrononCoordinatorBackend>, ServerFnError> {
    if let Some(cfg) = leptos::context::use_context::<std::sync::Arc<higgs::HiggsConfig>>() {
        if let Ok(backend) = cfg.chronon_backend_arc() {
            return Ok(backend);
        }
    }
    leptos::context::use_context::<std::sync::Arc<dyn chronon_coordinator::ChrononCoordinatorBackend>>()
        .ok_or_else(|| ServerFnError::new("Chronon backend not in request context"))
}

/// Pools the host offers: the context [`chronon_backend::ChrononPoolProvider`],
/// or [`chronon_backend::default_chronon_pool_rows`] when the host supplies none.
#[cfg(feature = "ssr")]
pub(super) fn offered_pools() -> Vec<chronon_backend::ChrononPoolPickRow> {
    leptos::context::use_context::<std::sync::Arc<dyn chronon_backend::ChrononPoolProvider>>()
        .map_or_else(chronon_backend::default_chronon_pool_rows, |p| p.pools())
}

/// Require an authenticated session (`SessionSnapshot` / `session_user_id`).
///
/// `SessionSnapshot` does not carry `email_verified`; use
/// [`require_email_verified`] for the job CRUD UI gate.
#[cfg(feature = "ssr")]
pub(super) fn require_session(ctx: &higgs::Higgs) -> Result<(), ServerFnError> {
    if ctx.session_user_id().is_some() {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "Authentication is required for this action",
        ))
    }
}

/// Mirror the job CRUD UI `requires_email_verification` gate server-side.
///
/// Uses axum-login's auth user (via lepton-auth) because `SessionSnapshot`
/// only stores `user_id` + `auth_hash`. With the `e2e-lab` Cargo feature, lab
/// hosts may force the outcome via `e2e_lab::set_email_verified_override`
/// without a lepton-auth Backend.
#[cfg(feature = "ssr")]
pub(super) async fn require_email_verified() -> Result<(), ServerFnError> {
    #[cfg(feature = "e2e-lab")]
    if let Some(verified) = crate::e2e_lab::email_verified_override() {
        return if verified {
            Ok(())
        } else {
            Err(ServerFnError::new(
                "Email verification is required for this action",
            ))
        };
    }
    let user = lepton_auth::extract_auth_user().await?;
    if user.email_verified {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "Email verification is required for this action",
        ))
    }
}

#[cfg(feature = "ssr")]
pub(super) async fn resolve_job_id(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
    job_id_or_name: &str,
) -> Result<Option<String>, ServerFnError> {
    if backend.get_job(job_id_or_name).await.is_some() {
        return Ok(Some(job_id_or_name.to_string()));
    }

    if let Some(j) = backend.get_job_by_name(job_id_or_name).await {
        return Ok(Some(j.job_id));
    }

    Ok(None)
}

#[cfg(feature = "ssr")]
pub(super) async fn resolve_job_names_for_model_runs(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
    runs: &[chronon_coordinator::models::Run],
) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for run in runs {
        let jid = run.job_id.clone().unwrap_or_default();
        if jid.is_empty() || names.contains_key(&jid) {
            continue;
        }
        let name = if let Some(job) = backend.get_job(&jid).await {
            job.job_name
        } else {
            run.script_name.clone()
        };
        names.insert(jid, name);
    }
    names
}
