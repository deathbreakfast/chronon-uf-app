# chronon-uf-app verification

Re-run after code or doc changes. This workspace is the Chronon operations app
(`chronon-app` Leptos UI + `chronon-backend` pure server contracts +
`chronon-uf-app-e2e` lab host). Layer 1 unit + integration tests cover
job/run/script/schedule/dashboard helpers backing the `#[server]` surface, plus
sibling-source UI surface contracts for `chronon-app`. Layer 2 is Playwright
against a dedicated lab Leptos host that mounts eager `ChrononRoutes` with mem
Valence and an in-process Chronon coordinator. Chronon coordinator / core
IsolatedLab contracts still own persistence and execution matrix correctness.

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-chronon-uf-app
```

## Teaching host

Axum oneshot under [`examples/protected-chronon-host`](../examples/protected-chronon-host/).
Copy table + product mount sketches live in that host README.

```bash
cargo check -p protected-chronon-host
cargo run -p protected-chronon-host
```

Success line: `protected_chronon_host: OK — /chronon deny/allow + dashboard KPIs`.
Hydrate/browser is out of gate for the oneshot (`cargo-leptos` + `wasm32` +
Orbital / `uf-product` belong to a composite product host or `chronon-uf-app-e2e`).

## Layer 1 — Unit + integration (CI)

GitHub Actions (`.github/workflows/ci.yml`) covers this Layer 1 subset plus the
teaching host, chronon-backend rustdoc gate, chronon-app SSR tests/clippy, and
the chronon-uf-app-e2e boundary check below. Layer 2 Playwright is a separate
`e2e` job in the same workflow.

Sibling-source UI contracts (no Orbital / `chronon-app` compile):

```bash
cargo test -p chronon-backend --test workspace_members --test product_surface
```

Backend contracts (preferred path; no UI graph):

```bash
cargo fmt -p chronon-backend -p chronon-app -p protected-chronon-host -p chronon-uf-app-e2e -- --check
cargo clippy -p chronon-backend --all-targets -- -D warnings
cargo clippy -p protected-chronon-host --all-targets -- -D warnings
cargo test -p chronon-backend
```

Host-aligned SSR surface (on CI when the UI graph compiles):

```bash
cargo clippy -p chronon-app --features ssr --all-targets -- -D warnings
cargo test -p chronon-app --features ssr
cargo check -p chronon-app --features ssr
cargo check -p chronon-uf-app-e2e --features ssr
```

Coordinator boundary (lab LocalBackend + Valence upsert / run-now):

```bash
cargo test -p chronon-uf-app-e2e --features ssr --test boundary_contract
```

`cargo fmt --all` can fail when a sibling checkout sits outside this workspace;
package-scoped fmt is the honest local gate.

Full workspace clippy/test may still fail when the sibling `uf-product` /
`uf-integrations` UI graph does not compile — that is a host-product UI issue,
not a Chronon backend contract gap. Surface needles for routes, nav testids,
`RequireAuthenticated`, and `ChrononAdmin` live in `product_surface` (structural
secondary; Layer 2 is primary for operator UI).

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### leptos-lints (local; hydrate UI)

Needs `cargo-dylint` / `dylint-link` 6.0.1 and toolchain `nightly-2025-05-14`
(see `leptos-lints@v0.1.2`). Workspace `[workspace.metadata.dylint]` pins the
library; rustc deny names are declared under `[workspace.lints.rust]`.

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview

export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-chronon-uf-app
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"

cargo dylint --all -p chronon-app --no-deps -- --features hydrate
```

Hard CI job deferred: `chronon-app` hydrate still depends on the Orbital / host
graph (same pin risk as UI compile in Layer 1). Run locally when that graph is
green.

## Layer 2 — E2E (lab host + Playwright, CI)

Primary operator-UI gate. Runs on pull requests and pushes to `main`/`master`
via the `e2e` job in `.github/workflows/ci.yml`. Dedicated lab host mounts eager
`ChrononRoutes` pages (same components as production Lazy routes), mem Valence,
Higgs session injection, and an in-process Chronon coordinator. Port
`127.0.0.1:3180`.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-chronon-uf-app
# From the chronon-uf-app workspace root. Builds SSR + hydrate, then Playwright.
cargo leptos end-to-end --project chronon-uf-app-e2e
```

Do not interrupt the end-to-end run. It stops when Playwright finishes.

Scenario IDs (validating happy + sad):

- `pw-chronon-auth-gate-happy-admin` / `pw-chronon-auth-gate-sad-anonymous`
- `pw-chronon-dashboard-happy-kpis` / `pw-chronon-dashboard-sad-empty-trend-not-crash`
- `pw-chronon-jobs-happy-list-detail` / `pw-chronon-jobs-sad-unknown-job` / `pw-chronon-jobs-sad-unverified-create`
- `pw-chronon-run-now-happy-admin` / `pw-chronon-run-now-sad-non-admin`
- `pw-chronon-runs-happy-list-detail` / `pw-chronon-runs-sad-unknown-run`
- `pw-chronon-scripts-happy-list` / `pw-chronon-scripts-sad-empty-search-not-crash`
- `help-spotlight-skips-when-seeded` / `help-spotlight-skips-auth-gate` / `help-spotlight-*-green` (all seven routes)

Help-tour security properties covered by those IDs: anonymous sessions never get a
spotlight footer even when `help_tour: true`; seeded visits stay quiet for non-tour
specs; tour copy and DOM `id`s carry no secrets.

See [`chronon-uf-app-e2e/README.md`](../chronon-uf-app-e2e/README.md).

## Layer 3 — Cloud + performance

**Waived.** This application workspace; no cloud resources or Criterion benches.
Correctness is in-process against Chronon UF app DTO/mapping contracts and the
lab e2e host. L0 `chronon-e2e` / cloud lab campaigns remain separate for
scheduler correctness.

## L5 host Playwright

**Deferred.** Live embedded/fleet `/chronon` Playwright with full product host
wiring and Chronon runtimes is out of scope for this workspace gate. Product
ops-UI correctness is covered by `chronon-uf-app-e2e` (Layer 2). Add L5 specs
in a later host workstream after the lab suite is green.

## Rustdoc policy

Preferred deny gate (no UI graph):

```bash
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p chronon-backend --no-deps
```

Workspace `rustdoc::broken_intra_doc_links` is `allow` in `Cargo.toml` because
sibling/cfg-gated links often fail under `--no-deps`. Prefer the
`RUSTDOCFLAGS` deny form above for the backend contract crate. `chronon-app`
rustdoc with deny flags is pin-dependent on Orbital / host graphs.
`chronon-app` still uses `#![allow(missing_docs)]` on macro-heavy UI surfaces.

## Notes

- Prefer `cargo test -p chronon-backend` for backend contract CI when the UI
  dependency graph (`uf-product` via `uf-integrations` / `lepton-shell`) fails to
  compile — report that separately from Chronon contract results.
- Tests may `unwrap`/`expect`; production server fns map failures to `ServerFnError`
  (no ordinary-path unwrap).
- Sad-path assertions check message content or `None` / empty — stronger than
  `is_err()` alone.
- Happy-path tests are named `*_happy_path` so audits detect them.
- Chronon routes data loaders call the `#[server]` fns; those fns are thin Higgs
  wrappers over the helpers covered by Layer 1 contract tests, with Layer 2
  exercising the real server-fn + UI path.
