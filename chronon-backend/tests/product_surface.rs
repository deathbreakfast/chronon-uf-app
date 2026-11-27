//! Product surface contracts for chronon-app (sibling crate).
//!
//! Lives under `chronon-backend` so CI can gate route/testid/auth/admin needles
//! without compiling Orbital/turf UI when host pins churn. Pattern matches
//! photon-uf-app / boson-uf-app `*-backend/tests/product_surface.rs`, gauge
//! `gauge/tests/product_surface.rs`, and lepton-uf-app
//! `lepton-shell/tests/product_surface.rs`.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read_app(rel: &str) -> String {
    let path = workspace_root().join("chronon-app").join("src").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn chronon_routes_mount_happy_path() {
    let lib = read_app("lib.rs");
    for needle in [
        r#"path!("chronon")"#,
        r#"path!("")"#,
        r#"path!("jobs")"#,
        r#"path!("jobs/new")"#,
        r#"path!("jobs/:job_id")"#,
        r#"path!("runs")"#,
        r#"path!("runs/:run_id")"#,
        r#"path!("scripts")"#,
        "ChrononLayoutRouteView",
        "id: \"chronon\"",
        "route_path: \"/chronon\"",
        "permission_manifest: permissions::ChrononPermission",
    ] {
        assert!(
            lib.contains(needle),
            "ChrononRoutes / uf_app missing `{needle}`"
        );
    }
}

#[test]
fn chronon_routes_drop_leaf_sad_path() {
    let lib = read_app("lib.rs");
    for needle in [
        r#"path!("jobs/new")"#,
        r#"path!("jobs/:job_id")"#,
        r#"path!("runs/:run_id")"#,
        r#"path!("scripts")"#,
    ] {
        assert!(
            lib.contains(needle),
            "removing `{needle}` drops a Chronon ops funnel entry"
        );
    }
    assert!(
        !lib.contains("unimplemented!"),
        "ChrononRoutes must not ship unimplemented placeholders"
    );
}

#[test]
fn uf_app_wrong_id_sad_path() {
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"chronon\""),
        "wrong uf_app id breaks Orbital host registration"
    );
    assert!(
        !lib.contains("id: \"chronon-app\""),
        "uf_app id must stay `chronon` (product route id), not crate name chronon-app"
    );
}

#[test]
fn layout_auth_gate_and_nav_happy_path() {
    let layout = read_app("layout.rs");
    for needle in [
        "chronon-app-root",
        "uf_product::routes::RequireAuthenticated",
        "RequireAuthenticated",
        "Outlet",
        "nav-dashboard",
        "nav-jobs",
        "nav-runs",
        "nav-scripts",
        "AppBarUserMenu",
        "UnifiedFieldShellLayout",
    ] {
        assert!(
            layout.contains(needle),
            "ChrononLayout missing contract `{needle}`"
        );
    }
}

#[test]
fn layout_drop_auth_guard_sad_path() {
    let layout = read_app("layout.rs");
    assert!(
        layout.contains("uf_product::routes::RequireAuthenticated")
            || (layout.contains("use uf_product::routes::RequireAuthenticated")
                && layout.contains("<RequireAuthenticated>")),
        "layout must use product RequireAuthenticated so AccessGate suppresses Help tours for anonymous sessions"
    );
    assert!(
        layout.contains("<Outlet />"),
        "RequireAuthenticated must wrap the route Outlet"
    );
}

#[test]
fn layout_missing_nav_sad_path() {
    let layout = read_app("layout.rs");
    for id in ["nav-dashboard", "nav-jobs", "nav-runs", "nav-scripts"] {
        assert!(
            layout.contains(id),
            "dropping `{id}` breaks operator left-nav contract"
        );
    }
}

/// Body of `pub async fn {name}` up to the next server fn attribute.
fn fn_body<'a>(src: &'a str, name: &str) -> &'a str {
    let start = src
        .find(&format!("pub async fn {name}("))
        .unwrap_or_else(|| panic!("server missing `{name}`"));
    let rest = &src[start..];
    let end = rest[1..]
        .find("#[uf_product_macros::server")
        .map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

#[test]
fn job_actions_check_the_jobs_own_access_happy_path() {
    let jobs = read_app("server/jobs.rs");
    let runs = read_app("server/runs.rs");
    let access = read_app("server/access.rs");
    assert!(
        access.contains("uf_product::access::require("),
        "per-job checks go through uf_product::access::require as the session actor"
    );
    assert!(
        fn_body(&runs, "run_job_now").contains(r#"require_job(&job, "Run")"#),
        "Run now and Run again need Run on the job"
    );
    assert!(
        jobs.contains(r#"require_job(&existing_job, "Edit")"#),
        "edits, pause and resume need Edit on the job (save_job_update)"
    );
    for name in ["update_job", "set_job_paused"] {
        assert!(
            fn_body(&jobs, name).contains("save_job_update("),
            "`{name}` must save through save_job_update so the Edit check runs"
        );
    }
    assert!(
        fn_body(&jobs, "get_job_revisions").contains(r#"require_job(&job, "View")"#),
        "revisions need View on the job"
    );
    let create_attr = r#"#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn create_job("#;
    assert!(
        jobs.contains(create_attr),
        "creating jobs stays a ChrononAdmin action"
    );
}

#[test]
fn job_actions_drop_per_job_check_sad_path() {
    let jobs = read_app("server/jobs.rs");
    let runs = read_app("server/runs.rs");
    let combined = format!("{jobs}\n{runs}");
    assert!(
        !fn_body(&runs, "run_job_now").contains("You can't run this job"),
        "run_job_now must refuse through the per-job denial, not a coarse admin message"
    );
    assert!(
        !combined.contains(r#"permission = "GaugeAdmin""#)
            && !combined.contains(r#"permission = "BosonAdmin""#)
            && !combined.contains(r#"permission = "PhotonAdmin""#),
        "Chronon server fns must not gate on GaugeAdmin, BosonAdmin, or PhotonAdmin"
    );
    assert!(
        !fn_body(&runs, "run_job_now").contains(r#"permission = "ChrononAdmin""#)
            && !runs.contains(
                r#"#[uf_product_macros::server(permission = "ChrononAdmin")]
pub async fn run_job_now("#
            ),
        "run_job_now is gated per job, not by ChrononAdmin"
    );
}

#[test]
fn server_require_session_happy_path() {
    let ssr = read_app("server/ssr_utils.rs");
    assert!(
        ssr.contains("fn require_session")
            && ssr.contains("Authentication is required")
            && ssr.contains("session_user_id()"),
        "ssr_utils must fail closed without a session"
    );
    assert!(
        ssr.contains("Chronon backend not in request context"),
        "missing Chronon backend context must surface a typed ServerFnError message"
    );
    assert!(
        ssr.contains("fn require_email_verified") && ssr.contains("Email verification is required"),
        "ssr_utils must fail closed without a verified email for job CRUD"
    );

    let dashboard = read_app("server/dashboard.rs");
    let jobs = read_app("server/jobs.rs");
    for (src, call_site) in [
        (dashboard.as_str(), "get_dashboard_stats"),
        (jobs.as_str(), "get_jobs"),
        (jobs.as_str(), "create_job"),
    ] {
        assert!(src.contains(call_site), "server missing `{call_site}`");
    }
}

#[test]
fn server_drop_require_session_on_get_jobs_sad_path() {
    let jobs = read_app("server/jobs.rs");
    let start = jobs.find("pub async fn get_jobs").expect("get_jobs");
    let body = &jobs[start..start + 350.min(jobs.len() - start)];
    assert!(
        body.contains("require_session(&ctx)?"),
        "get_jobs must call require_session before Chronon IO"
    );

    let start = jobs.find("pub async fn create_job").expect("create_job");
    let body = &jobs[start..start + 450.min(jobs.len() - start)];
    assert!(
        body.contains("require_session(&ctx)?"),
        "create_job must call require_session before Chronon IO"
    );
}

#[test]
fn job_crud_email_gate_happy_path() {
    let lazy = read_app("lazy_routes.rs");
    assert!(
        lazy.contains("requires_email_verification=true")
            && lazy.contains("ChrononJobCreatePage")
            && lazy.contains("ChrononJobDetailPage"),
        "job create/detail lazy routes must require email verification"
    );

    let jobs = read_app("server/jobs.rs");
    let start = jobs.find("pub async fn create_job").expect("create_job");
    let body = &jobs[start..start + 500.min(jobs.len() - start)];
    assert!(
        body.contains("require_email_verified().await?"),
        "create_job must mirror the UI email-verification gate"
    );
}

#[test]
fn job_crud_drop_email_gate_sad_path() {
    let lazy = read_app("lazy_routes.rs");
    let email_gates = lazy.matches("requires_email_verification=true").count();
    assert!(
        email_gates >= 2,
        "dropping email verification on ChrononVerifiedJob* routes opens job CRUD to unverified sessions"
    );
    let jobs = read_app("server/jobs.rs");
    let start = jobs.find("pub async fn update_job").expect("update_job");
    let body = &jobs[start..start + 550.min(jobs.len() - start)];
    assert!(
        body.contains("require_email_verified().await?"),
        "update_job must keep require_email_verified"
    );
}

#[test]
fn index_pages_testid_and_list_bindings_happy_path() {
    let dashboard = read_app("pages/dashboard/mod.rs");
    for needle in [
        "chronon-dashboard",
        "get_dashboard_stats",
        "get_recent_runs",
    ] {
        assert!(
            dashboard.contains(needle),
            "ChrononDashboardPage missing `{needle}`"
        );
    }

    let jobs = read_app("pages/jobs/mod.rs");
    assert!(
        jobs.contains("chronon-jobs-page"),
        "ChrononJobsPage missing chronon-jobs-page testid"
    );

    let runs = read_app("pages/runs/mod.rs");
    assert!(
        runs.contains("chronon-runs-page"),
        "ChrononRunsPage missing chronon-runs-page testid"
    );

    let scripts = read_app("pages/scripts/mod.rs");
    assert!(
        scripts.contains("chronon-scripts-page"),
        "ChrononScriptsPage missing chronon-scripts-page testid"
    );
}

#[test]
fn index_drop_dashboard_testid_sad_path() {
    let dashboard = read_app("pages/dashboard/mod.rs");
    assert!(
        dashboard.contains("data_testid=\"chronon-dashboard\""),
        "dropping chronon-dashboard breaks host / future Playwright parity"
    );
    let jobs = read_app("pages/jobs/mod.rs");
    assert!(
        jobs.contains("data_testid=\"chronon-jobs-page\""),
        "dropping chronon-jobs-page breaks host / future Playwright parity"
    );
    let runs = read_app("pages/runs/mod.rs");
    assert!(
        runs.contains("data_testid=\"chronon-runs-page\""),
        "dropping chronon-runs-page breaks host / future Playwright parity"
    );
    let scripts = read_app("pages/scripts/mod.rs");
    assert!(
        scripts.contains("data_testid=\"chronon-scripts-page\""),
        "dropping chronon-scripts-page breaks host / future Playwright parity"
    );
}

#[test]
fn detail_pages_testid_and_bindings_happy_path() {
    let create = read_app("pages/job_create/mod.rs");
    for needle in ["chronon-job-create-page", "create_job", "get_scripts"] {
        assert!(
            create.contains(needle),
            "ChrononJobCreatePage missing `{needle}`"
        );
    }

    let job = read_app("pages/job_detail/mod.rs");
    // Revisions moved off the job page header (History tab); the page loads the job
    // and the scripts catalog.
    for needle in ["chronon-job-detail", "get_job", "get_scripts"] {
        assert!(
            job.contains(needle),
            "ChrononJobDetailPage missing `{needle}`"
        );
    }

    let run = read_app("pages/run_detail/mod.rs");
    for needle in ["chronon-run-detail", "get_run"] {
        assert!(
            run.contains(needle),
            "ChrononRunDetailPage missing `{needle}`"
        );
    }
}

#[test]
fn detail_pages_missing_bindings_sad_path() {
    let create = read_app("pages/job_create/mod.rs");
    assert!(
        create.contains("create_job"),
        "job create must bind create_job"
    );
    let job = read_app("pages/job_detail/mod.rs");
    assert!(job.contains("get_job"), "job detail must bind get_job");
    let run = read_app("pages/run_detail/mod.rs");
    assert!(run.contains("get_run"), "run detail must bind get_run");
    assert!(
        !create.contains("unimplemented!")
            && !job.contains("unimplemented!")
            && !run.contains("unimplemented!"),
        "detail pages must not ship unimplemented placeholders"
    );
}

#[test]
fn permission_manifest_chronon_admin_happy_path() {
    let perms = read_app("permissions.rs");
    for needle in [
        "domain_key = \"chronon\"",
        "ChrononAdmin",
        "UfPermissionManifest",
    ] {
        assert!(
            perms.contains(needle),
            "ChrononPermission manifest missing `{needle}`"
        );
    }
}

#[test]
fn protected_chronon_host_matches_uf_app_happy_path() {
    let host =
        fs::read_to_string(workspace_root().join("examples/protected-chronon-host/src/main.rs"))
            .expect("protected-chronon-host main.rs");
    for needle in [
        "\"app_id\": \"chronon\"",
        "\"route_path\": \"/chronon\"",
        "\"auth_gate\": \"RequireAuthenticated\"",
        "\"admin_permission\": \"ChrononAdmin\"",
        "dashboard_stats_from_jobs_and_runs",
    ] {
        assert!(
            host.contains(needle),
            "protected-chronon-host missing contract `{needle}`"
        );
    }
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"chronon\"") && lib.contains("route_path: \"/chronon\""),
        "host inventory must stay aligned with uf_app!"
    );
    let layout = read_app("layout.rs");
    assert!(
        layout.contains("RequireAuthenticated"),
        "host auth_gate must stay aligned with ChrononLayout guard"
    );
    let perms = read_app("permissions.rs");
    assert!(
        perms.contains("ChrononAdmin"),
        "host admin_permission must stay aligned with ChrononPermission"
    );
}

#[test]
fn lazy_routes_wire_pages_happy_path() {
    let lazy = read_app("lazy_routes.rs");
    for needle in [
        "ChrononDashboardPage",
        "ChrononJobsPage",
        "ChrononJobCreatePage",
        "ChrononJobDetailPage",
        "ChrononRunsPage",
        "ChrononRunDetailPage",
        "ChrononScriptsPage",
        "ChrononLayout",
    ] {
        assert!(
            lazy.contains(needle),
            "lazy_routes missing page wire `{needle}`"
        );
    }
}

#[test]
fn ops_path_helpers_encode_segments_happy_path() {
    let recent = read_app("pages/dashboard/recent_runs_table.rs");
    let jobs_table = read_app("components/jobs_data_table/mod.rs");
    let runs_table = read_app("components/runs_data_table/mod.rs");
    let run_detail = read_app("pages/run_detail/content.rs");
    for (label, src) in [
        ("recent_runs_table", recent.as_str()),
        ("jobs_data_table", jobs_table.as_str()),
        ("runs_data_table", runs_table.as_str()),
        ("run_detail_content", run_detail.as_str()),
    ] {
        assert!(
            src.contains("chronon_backend::chronon_")
                || src.contains("chronon_job_path")
                || src.contains("chronon_run_path"),
            "{label} must build detail hrefs via chronon_backend path helpers"
        );
        assert!(
            !src.contains("crate::paths::job(") && !src.contains("crate::paths::run("),
            "{label} must not interpolate raw ids into orbital paths::*"
        );
    }
}

#[test]
fn ops_path_helpers_drop_encoding_sad_path() {
    let recent = read_app("pages/dashboard/recent_runs_table.rs");
    assert!(
        recent.contains("chronon_backend::chronon_run_path"),
        "dropping chronon_run_path reopens path-segment smuggling via run ids"
    );
    let jobs_table = read_app("components/jobs_data_table/mod.rs");
    assert!(
        jobs_table.contains("chronon_backend::chronon_job_path"),
        "dropping chronon_job_path reopens path-segment smuggling via job ids"
    );
    let run_detail = read_app("pages/run_detail/content.rs");
    assert!(
        run_detail.contains("chronon_backend::chronon_job_path"),
        "dropping chronon_job_path on run detail reopens path-segment smuggling via job ids"
    );
}
