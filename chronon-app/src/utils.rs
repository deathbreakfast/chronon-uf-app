/// Format an ISO 8601 `started_at` timestamp for display (HH:MM).
pub fn format_started_at(started_at: &str) -> String {
    if let Some(time_part) = started_at.split('T').nth(1) {
        if let Some(time) = time_part.split('.').next() {
            let parts: Vec<&str> = time.split(':').collect();
            if parts.len() >= 2 {
                return format!("{}:{}", parts[0], parts[1]);
            }
        }
    }
    started_at.to_string()
}

/// Loads data on the client only and keeps the latest result in a signal.
///
/// `fetch` runs inside an Effect, so it reruns when a signal it reads (before its `async`
/// block) changes. Effects never run on the server, so server rendering and hydration both
/// see `None` and the page never waits on this data.
pub fn client_fetch<T, Fut>(
    fetch: impl Fn() -> Fut + 'static,
) -> leptos::prelude::RwSignal<Option<T>>
where
    T: Send + Sync + 'static,
    Fut: std::future::Future<Output = Option<T>> + 'static,
{
    use leptos::prelude::{Effect, RwSignal, Set};
    let out = RwSignal::new(None);
    Effect::new(move |_| {
        let fut = fetch();
        leptos::task::spawn_local_scoped(async move {
            if let Some(value) = fut.await {
                out.set(Some(value));
            }
        });
    });
    out
}
