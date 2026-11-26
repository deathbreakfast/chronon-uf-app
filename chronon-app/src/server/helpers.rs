//! Auth / persistence helpers for Chronon server functions.
//!
//! Pure DTO mappers and schedule builders live in [`chronon_backend`].

#[cfg(feature = "ssr")]
use leptos::prelude::ServerFnError;

#[cfg(feature = "ssr")]
pub(super) use chronon_backend::{
    apply_update_payload_to_job, backend_job_to_job, build_create_job_model, parse_script_params,
};

/// Validates that no existing job already uses the requested name.
#[cfg(feature = "ssr")]
pub(super) async fn ensure_job_name_available(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
    job_name: &str,
) -> Result<(), ServerFnError> {
    if backend.get_job_by_name(job_name).await.is_some() {
        return Err(ServerFnError::new(format!(
            "A job named '{}' already exists",
            job_name
        )));
    }

    Ok(())
}

/// Loads existing job data (cache-first) for update operations.
#[cfg(feature = "ssr")]
pub(super) async fn load_existing_job_for_update(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
    actual_job_id: &str,
) -> Result<chronon_coordinator::models::Job, ServerFnError> {
    backend
        .get_job(actual_job_id)
        .await
        .ok_or_else(|| ServerFnError::new(format!("Job {} not found", actual_job_id)))
}

/// Persists updated job config using cache-aware persistence paths.
#[cfg(feature = "ssr")]
pub(super) async fn persist_updated_job_config(
    backend: &dyn chronon_coordinator::ChrononCoordinatorBackend,
    v: &valence::Valence,
    actual_job_id: &str,
    updated_job: chronon_coordinator::models::Job,
) -> Result<(), ServerFnError> {
    backend
        .update_job_config_with_valence(v, actual_job_id, updated_job)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update job: {}", e)))?;
    Ok(())
}

/// Display names for the `User` actors in `actors`, keyed by
/// [`chronon_backend::actor_user_id`], read from Lepton profiles as the session actor.
///
/// A user with no profile, a blank name, or a profile the viewer can't read is left out,
/// so [`chronon_backend::actor_label_named`] falls back to the user id.
#[cfg(feature = "ssr")]
pub(super) async fn actor_display_names<'a>(
    v: &valence::Valence,
    actors: impl IntoIterator<Item = &'a serde_json::Value>,
) -> std::collections::HashMap<String, String> {
    use lepton::generated::UserProfile;
    use valence::{RecordId, RecordPredicate};

    let ids: std::collections::BTreeSet<&str> = actors
        .into_iter()
        .filter_map(chronon_backend::actor_user_id)
        .collect();
    let mut names = std::collections::HashMap::new();
    for id in ids {
        let profile = UserProfile::query(
            v,
            valence::use_!(r"In **Chronon job details**, we **read the display name on a person's profile** so the paused banner and the Edit schedule conflict message say who paused or last saved the job by name instead of a user id. Only the display name is used, and only where the viewer may already read the profile."),
        )
        .where_user(RecordPredicate::Equals(RecordId::new("user", id)))
        .first()
        .await;
        match profile {
            Ok(Some(profile)) => {
                let name = profile.display_name().trim();
                if !name.is_empty() {
                    names.insert(id.to_string(), name.to_string());
                }
            }
            Ok(None) => {}
            Err(e) => leptos::logging::warn!("chronon: profile name read failed for {id}: {e}"),
        }
    }
    names
}
