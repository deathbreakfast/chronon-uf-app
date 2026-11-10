//! Photon live-update subscriptions for Chronon run lifecycle events.

use leptos::prelude::*;
use serde_json::Value;

/// Reserved for the future per-job WS wiring (see `ChrononJobRunLiveSource`).
pub const CHRONON_JOB_RUN_WS_PREFIX: &str = "/ws/chronon-job";

/// Reactive handle to a per-job run-update WebSocket subscription.
#[derive(Clone, Copy)]
pub struct ChrononJobRunSubscription {
    pub trigger: RwSignal<u64>,
    pub latest_event: RwSignal<Option<Value>>,
}

/// Returns true when the event payload targets the given job (status or stdout).
pub fn chronon_run_event_matches_job(event: &Value, job_id: &str) -> bool {
    event
        .get("job_id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id == job_id)
}

/// Returns true when the event payload targets the given run.
pub fn chronon_run_event_matches_run(event: &Value, run_id: &str) -> bool {
    event
        .get("run_id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id == run_id)
}

/// Returns true for status transitions (not stdout streaming).
pub fn chronon_run_event_is_status(event: &Value) -> bool {
    event
        .get("kind")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| kind == "status")
}

/// Placeholder live source for the standalone export.
///
/// The template route used Photon Axum wiring; this export intentionally omits
/// that route and relies on the polling refresh path below.
#[component]
pub fn ChrononJobRunLiveSource(
    /// Reactive signal for the job ID.
    job_id: Signal<Option<String>>,
    /// Two-way signal holding the trigger element/state.
    trigger: RwSignal<u64>,
    /// Two-way signal holding the latest event.
    latest_event: RwSignal<Option<Value>>,
) -> impl IntoView {
    let _ = (job_id, trigger, latest_event);
}

/// Interval for dashboard and runs-list polling (no broadcast WS). Only read when the
/// `hydrate` feature is enabled.
#[cfg_attr(not(feature = "hydrate"), allow(dead_code))]
pub const CHRONON_POLL_INTERVAL_MS: u64 = 20_000;

/// Bump a tick on an interval for resource/table refresh (client only).
#[cfg(feature = "hydrate")]
pub fn use_chronon_poll_tick() -> RwSignal<u32> {
    let tick = RwSignal::new(0u32);
    leptos_use::use_interval_fn(
        move || {
            tick.update(|n| *n += 1);
        },
        CHRONON_POLL_INTERVAL_MS,
    );
    tick
}

/// SSR stub — polling runs in the browser bundle only.
#[cfg(not(feature = "hydrate"))]
pub fn use_chronon_poll_tick() -> RwSignal<u32> {
    RwSignal::new(0u32)
}

/// Shared subscription signals for a job-scoped live source.
pub fn chronon_job_run_subscription() -> ChrononJobRunSubscription {
    ChrononJobRunSubscription {
        trigger: RwSignal::new(0),
        latest_event: RwSignal::new(None),
    }
}
