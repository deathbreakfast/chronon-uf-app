//! Harness-only seed endpoint for Playwright.

use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::e2e_valence::e2e_fixtures;
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `admin` | `outsider` | `unverified` | `member`
    #[serde(default = "default_auth")]
    pub auth: String,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Lab seam: skip lepton-auth Backend for job CRUD server fns.
    match kind {
        E2eAuthKind::Anonymous => chronon_app::e2e_lab::set_email_verified_override(None),
        E2eAuthKind::Admin | E2eAuthKind::Outsider | E2eAuthKind::Member => {
            chronon_app::e2e_lab::set_email_verified_override(Some(true));
        }
        E2eAuthKind::Unverified => {
            chronon_app::e2e_lab::set_email_verified_override(Some(false));
        }
    }

    let fixtures = e2e_fixtures();
    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "fixtures": {
            "script_name": fixtures.script_name,
            "job_id": fixtures.job_id,
            "job_name": fixtures.job_name,
            "run_id": fixtures.run_id,
            "purge_job_id": fixtures.purge_job_id,
            "purge_job_name": fixtures.purge_job_name,
            "purge_run_id": fixtures.purge_run_id,
            "params_job_id": fixtures.params_job_id,
            "params_job_name": fixtures.params_job_name,
            "access_job_id": fixtures.access_job_id,
            "access_job_name": fixtures.access_job_name,
            "request_job_id": fixtures.request_job_id,
            "request_job_name": fixtures.request_job_name,
            "refused_job_id": fixtures.refused_job_id,
            "refused_job_name": fixtures.refused_job_name,
            "admins_group": crate::e2e_valence::CHRONON_ADMINS_LABEL,
            "team_name": crate::e2e_valence::E2E_TEAM_NAME,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct HoldRequest {
    pub hold: bool,
}

/// Lab only: keep new runs queued (`hold: true`) or start them as they come (`false`).
pub async fn hold_runs(Json(body): Json<HoldRequest>) -> Json<serde_json::Value> {
    crate::e2e_valence::set_hold_runs(body.hold);
    Json(serde_json::json!({ "ok": true, "hold": body.hold }))
}

/// Lab only: start every held run (each one re-checks Run as the person who started it).
pub async fn release_runs() -> Json<serde_json::Value> {
    crate::e2e_valence::set_hold_runs(false);
    let started = crate::e2e_valence::release_held_runs().await;
    Json(serde_json::json!({ "ok": true, "started": started }))
}

/// Lab only: approve, in Gauge, every pending access request the signed-in person may
/// review. Runs as that person's own Valence through Gauge's request flow; nothing elevates.
pub async fn approve_requests(
    session: tower_sessions::Session,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use gauge::types::{PermissionRequestDecision, PermissionRequestDecisionInput};

    let kind = crate::gate_demos::read_e2e_auth_kind(&session).await;
    let Some(user_id) = kind.session_user_id() else {
        return Err((StatusCode::UNAUTHORIZED, "sign in first".into()));
    };
    let reviewer = crate::e2e_valence::e2e_system_valence().with_actor(valence::Actor::User {
        user_id: user_id.trim_start_matches("user:").to_string(),
    });
    let pending = gauge::service::list_permission_requests_for_review(&reviewer)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut approved = Vec::new();
    for row in pending {
        gauge::service::decide_permission_request(
            PermissionRequestDecisionInput {
                request_id: row.id.clone(),
                decision: PermissionRequestDecision::Approve,
            },
            &reviewer,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        approved.push(row.target_label);
    }
    Ok(Json(
        serde_json::json!({ "ok": true, "approved": approved }),
    ))
}
