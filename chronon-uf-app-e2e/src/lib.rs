//! Chronon ops UI Playwright host.
#![allow(missing_docs)]
// Lab bootstrap panics on setup failure — same posture as boson-uf-app-e2e.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod app;
mod chronon_routes_eager;
#[cfg(feature = "ssr")]
mod e2e_valence;
mod gate_demos;
mod harness_auth_menu;
#[cfg(feature = "ssr")]
pub mod seed;

pub use app::{shell, wire_gauge_permissions_bridge, App};
#[cfg(feature = "ssr")]
pub use e2e_valence::{
    e2e_admin_valence, e2e_chronon_backend, e2e_fixtures, e2e_higgs_config, e2e_pool_provider,
    e2e_registry, e2e_router, e2e_system_valence, init_e2e_valence, E2E_POOL,
};
#[cfg(feature = "ssr")]
pub use gate_demos::inject_e2e_session_snapshot;
