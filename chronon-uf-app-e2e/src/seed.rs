//! Harness-only seed endpoint for Playwright.

use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::e2e_valence::e2e_fixtures;
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `admin` | `outsider` | `unverified`
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
        E2eAuthKind::Admin | E2eAuthKind::Outsider => {
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
        }
    })))
}
