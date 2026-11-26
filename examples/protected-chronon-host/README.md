# protected-chronon-host

Axum oneshot host under **`/chronon`**: deny without session, allow with
`X-Demo-User`, return the in-memory dashboard KPI shape `chronon-backend` builds
for the UI.

Production Leptos hosts mount `ChrononRoutes` at **`/chronon`** and gate
each job's actions on that job's own access (creating jobs needs `ChrononAdmin`). This example proves the same path + auth +
dashboard contract without the SSR/WASM / Orbital graph. The oneshot path
`/chronon` matches the Orbital app id/path (`chronon` / `/chronon`).

| | |
|---|---|
| **When to use** | First smoke of Chronon UF app host wiring (auth gate + dashboard API) |
| **Command** | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-chronon-uf-app cargo run -p protected-chronon-host` |
| **Success** | Stdout: `protected_chronon_host: OK — /chronon deny/allow + dashboard KPIs` |
| **Look next** | Mount [`ChrononRoutes`](../../chronon-app/) ; wire Chronon coordinator |

**Open first:** [`src/main.rs`](src/main.rs)

## Copy into your host

| File | What to take |
|------|----------------|
| This [`Cargo.toml`](Cargo.toml) | Axum oneshot shape + `chronon-backend` (dashboard KPI smoke) |
| Product mount `Cargo.toml` (below) | `chronon-app` + `chronon-backend` with `ssr` / `hydrate` features |
| [`src/main.rs`](src/main.rs) | Session gate on `/chronon`, dashboard JSON, inventory contract names |
| Leptos sketch (below) | `<ChrononRoutes />` under `/chronon` |

### Product mount dependencies

```toml
[dependencies]
chronon-app = { git = "https://github.com/unified-field-dev/chronon-uf-app", package = "chronon-app", rev = "REPLACE_WITH_PIN", default-features = false }
chronon-backend = { git = "https://github.com/unified-field-dev/chronon-uf-app", package = "chronon-backend", rev = "REPLACE_WITH_PIN" }
uf-product = { /* your pin */, default-features = false }
uf-integrations = { /* your pin */, default-features = false }

[features]
ssr = [
    "chronon-app/ssr",
    "uf-product/ssr",
    "uf-integrations/ssr",
]
hydrate = [
    "chronon-app/hydrate",
    "uf-product/hydrate",
    "uf-integrations/hydrate",
]
```

### Leptos mount sketch

```rust,ignore
use chronon_app::ChrononRoutes;
use leptos_router::components::Routes;

view! {
    <Routes fallback=|| "not found">
        <ChrononRoutes />
    </Routes>
}
```

Dashboard helpers (Leptos-free):

```rust,ignore
use chronon_backend::dashboard_stats_from_jobs_and_runs;

let stats = dashboard_stats_from_jobs_and_runs(&jobs, &runs, today_start);
```

Inventory names match `chronon` / `/chronon`. Layout uses `RequireAuthenticated`;
creating a job needs `ChrononAdmin` (manifest `permissions::ChrononPermission`), and
viewing, running and editing a job need View, Run and Edit on that job. Job create/edit also require a verified email.
Wire Chronon coordinator + session extractors in host bootstrap before mounting
the routes.

For shell chrome (layout, fonts, Axum + Leptos boot), copy
[`shell-chrome-host`](https://github.com/unified-field-dev/unified-field-product/tree/main/examples/shell-chrome-host)
from unified-field-product, then mount `ChrononRoutes`.

## Run (documented gate)

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-chronon-uf-app
cargo check -p protected-chronon-host
cargo run -p protected-chronon-host
```

**Success:** stdout prints `protected_chronon_host: OK — /chronon deny/allow + dashboard KPIs`.

## Hydrate / browser

Out of gate for this host. Full ops UI needs a product binary with
`cargo-leptos`, `wasm32`, session chrome, Chronon coordinator, and a working
Orbital / `uf-product` graph. Prefer the oneshot above.
