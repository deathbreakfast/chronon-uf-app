# chronon-uf-app-e2e

Leptos lab host + Playwright for [`chronon-app`](../chronon-app/) `ChrononRoutes`.

Mounts the same pages a product host would under `/chronon`, with lab-only mem
Valence, session injection, and an in-process Chronon coordinator backend.
**Do not copy this boot into a production host.**

## Run

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-chronon-uf-app
# From the chronon-uf-app workspace root.
cargo leptos end-to-end --project chronon-uf-app-e2e
```

Do not interrupt the end-to-end run. It stops on its own when Playwright finishes.

Site: `http://127.0.0.1:3180` · seed: `POST /api/test/seed-data`

Boundary integration (no browser):

```bash
cargo test -p chronon-uf-app-e2e --features ssr --test boundary_contract
```

## Scenarios

| ID | Asserts |
|----|---------|
| `pw-chronon-auth-gate-*` | Anon gated; admin sees dashboard |
| `pw-chronon-dashboard-*` | KPI / seeded job visible; empty trend does not crash |
| `pw-chronon-jobs-*` | List→detail; unknown job; unverified create blocked; host pool provider rows in the create and edit pickers, edited pool persists |
| `pw-chronon-run-now-*` | Admin run-now; non-admin denied / hidden |
| `pw-chronon-runs-*` | List→detail; unknown run |
| `pw-chronon-scripts-*` | Script catalog lists lab script |

## L5 host Playwright

Deferred. Product e2e in this crate is the CI correctness gate for ops UI.
Live L5 `/chronon` Playwright (embedded/fleet full host + runtimes) is tracked
separately and is not required to land this lab host.
