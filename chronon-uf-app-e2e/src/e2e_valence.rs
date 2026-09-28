//! Process-wide Valence + Higgs + in-memory Chronon for Playwright.
#![allow(dead_code)]

use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use chrono::Utc;
use chronon_coordinator::{
    snapshot_job_actor_from_valence, validate_external_job_actor_json, ChrononCoordinatorBackend,
    Job, JobRevision, Result as ChrononResult, Run, ScheduleKind, ScriptRegistry,
};
use chronon_core::{Result as CoreResult, RunStatus, ScriptContext};
use chronon_executor::ScriptDescriptor;
use gauge::manifest_sync::{
    sync_permission_manifests, PermissionDomainInput, PermissionInput, PermissionManifestInput,
};
use gauge::service;
use gauge::super_user::SUPER_USER_GROUP_NAME;
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter,
    InMemoryBackend, Model, RegisterBackendLogicalNamesOptions, RouterValenceFactory,
    RouterValenceFactoryConfig, Valence, ValenceFactory, MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    chronon_backend: Arc<dyn ChrononCoordinatorBackend>,
    registry: Arc<ScriptRegistry>,
    default_backend_key: String,
    fixtures: Mutex<FixtureIds>,
}

/// Stable fixture ids exposed to seed JSON / Playwright.
#[derive(Clone, Debug, Default)]
pub struct FixtureIds {
    pub script_name: String,
    pub job_id: String,
    pub job_name: String,
    pub run_id: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

/// Lab script name registered into the in-process registry.
pub const E2E_SCRIPT_NAME: &str = "e2e_echo";
pub const E2E_JOB_NAME: &str = "e2e-nightly";

fn e2e_echo_invoke(
    _ctx: Box<dyn ScriptContext>,
    _params: serde_json::Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = CoreResult<()>> + Send + 'static>> {
    Box::pin(async { Ok(()) })
}

fn e2e_script_registry() -> ScriptRegistry {
    let mut registry = ScriptRegistry::new();
    let desc: &'static ScriptDescriptor = Box::leak(Box::new(ScriptDescriptor::new(
        E2E_SCRIPT_NAME,
        e2e_echo_invoke,
    )));
    registry.register(desc);
    registry
}

#[derive(Default)]
struct LocalBackend {
    jobs: Mutex<Vec<Job>>,
    runs: Mutex<Vec<Run>>,
    revisions: Mutex<Vec<(String, JobRevision)>>,
}

impl LocalBackend {
    fn store_job(&self, job: Job) {
        let mut jobs = self.jobs.lock().expect("local backend lock");
        if let Some(existing) = jobs
            .iter_mut()
            .find(|existing| existing.job_id == job.job_id)
        {
            *existing = job;
        } else {
            jobs.push(job);
        }
    }

    fn store_run(&self, run: Run) {
        let mut runs = self.runs.lock().expect("runs lock");
        if let Some(existing) = runs.iter_mut().find(|r| r.run_id == run.run_id) {
            *existing = run;
        } else {
            runs.push(run);
        }
    }
}

#[async_trait]
impl ChrononCoordinatorBackend for LocalBackend {
    async fn load_jobs_from_db(&self) -> ChrononResult<()> {
        Ok(())
    }

    async fn upsert_job(&self, job: Job) -> ChrononResult<()> {
        validate_external_job_actor_json(&job.actor_json)?;
        self.store_job(job);
        Ok(())
    }

    async fn upsert_job_with_valence(&self, valence: &Valence, mut job: Job) -> ChrononResult<()> {
        snapshot_job_actor_from_valence(&mut job, valence)?;
        self.store_job(job);
        Ok(())
    }

    async fn get_job(&self, job_id: &str) -> Option<Job> {
        self.jobs
            .lock()
            .expect("local backend lock")
            .iter()
            .find(|job| job.job_id == job_id)
            .cloned()
    }

    async fn get_job_by_name(&self, job_name: &str) -> Option<Job> {
        self.jobs
            .lock()
            .expect("local backend lock")
            .iter()
            .find(|job| job.job_name == job_name)
            .cloned()
    }

    async fn list_jobs(&self) -> Vec<Job> {
        self.jobs.lock().expect("local backend lock").clone()
    }

    async fn list_runs(
        &self,
        job_id: Option<&str>,
        status: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> ChrononResult<Vec<Run>> {
        let mut runs = self.runs.lock().expect("runs lock").clone();
        if let Some(jid) = job_id {
            runs.retain(|r| r.job_id.as_deref() == Some(jid));
        }
        if let Some(st) = status {
            runs.retain(|r| r.status.to_string() == st);
        }
        Ok(runs.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_run(&self, run_id: &str) -> ChrononResult<Option<Run>> {
        Ok(self
            .runs
            .lock()
            .expect("runs lock")
            .iter()
            .find(|r| r.run_id == run_id)
            .cloned())
    }

    async fn pause_job(&self, job_id: &str) -> ChrononResult<()> {
        let mut jobs = self.jobs.lock().expect("local backend lock");
        if let Some(job) = jobs.iter_mut().find(|j| j.job_id == job_id) {
            job.enabled = false;
        }
        Ok(())
    }

    async fn resume_job(&self, job_id: &str) -> ChrononResult<()> {
        let mut jobs = self.jobs.lock().expect("local backend lock");
        if let Some(job) = jobs.iter_mut().find(|j| j.job_id == job_id) {
            job.enabled = true;
        }
        Ok(())
    }

    async fn list_revisions(&self, job_id_or_name: &str) -> ChrononResult<Vec<JobRevision>> {
        let job_id = self
            .get_job(job_id_or_name)
            .await
            .or(self.get_job_by_name(job_id_or_name).await)
            .map(|j| j.job_id);
        let Some(job_id) = job_id else {
            return Ok(Vec::new());
        };
        Ok(self
            .revisions
            .lock()
            .expect("revisions lock")
            .iter()
            .filter(|(id, _)| id == &job_id)
            .map(|(_, rev)| rev.clone())
            .collect())
    }

    async fn update_job_config(&self, _job_id: &str, updated: Job) -> ChrononResult<()> {
        self.upsert_job(updated).await
    }

    async fn update_job_config_with_valence(
        &self,
        valence: &Valence,
        job_id: &str,
        updated: Job,
    ) -> ChrononResult<()> {
        let _ = job_id;
        self.upsert_job_with_valence(valence, updated).await
    }

    async fn run_now(&self, job_id: &str) -> ChrononResult<String> {
        self.run_now_with_params(job_id, None).await
    }

    async fn run_now_with_params(
        &self,
        job_id: &str,
        params_override: Option<serde_json::Value>,
    ) -> ChrononResult<String> {
        let job = self
            .get_job(job_id)
            .await
            .ok_or_else(|| chronon_core::ChrononError::JobNotFound(job_id.into()))?;
        let now = Utc::now();
        let mut run = Run::new(job.script_name.clone(), now);
        run.job_id = Some(job.job_id.clone());
        run.status = RunStatus::Success;
        run.started_at = Some(now);
        run.finished_at = Some(now);
        run.duration_ms = Some(15);
        run.actor_json = job.actor_json.clone();
        run.params_json = params_override.unwrap_or(job.params_json);
        let run_id = run.run_id.clone();
        self.store_run(run);
        Ok(run_id)
    }
}

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

async fn seed_user(id: &str, email_verified: bool, valence: &Valence) {
    let now = Utc::now();
    let confirmed_at = email_verified.then_some(now);
    let user = lepton::generated::User::new(
        Some(lepton::generated::UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(lepton::generated::UserStatus::Active),
        None,
        None,
        confirmed_at,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    lepton::generated::User::upsert(id, user, valence, valence::use_!(r#"**Test:** Fixture **User** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("upsert user");
}

async fn seed_super_user_with_member(system: &Valence, member_user_id: &str) {
    let super_group = gauge::generated::PermissionGroup::new(
        SUPER_USER_GROUP_NAME.to_string(),
        Some("super users".to_string()),
        Utc::now(),
        Utc::now(),
    )
    .expect("build super user group");
    let created =
        gauge::generated::PermissionGroup::upsert("super_user_group", super_group, system, valence::use_!(r#"**Test:** Fixture **Permission Group** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
            .await
            .expect("upsert super user group");

    let member = lepton::generated::User::get(member_user_id, system, valence::use_!(r#"**Test:** Fixture **User** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("query member")
        .expect("member exists");
    let principal = gauge::generated::PermissionUserPrincipal::upsert(
        &format!("user:{member_user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            member.id().expect("member id").clone(),
            member_user_id.to_string(),
        )
        .expect("new principal"),
        system,
        valence::use_!(r#"**Test:** Fixture **Permission User Principal** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#),
    )
    .await
    .expect("upsert principal");
    created
        .relate_to_owner_record(
            principal.id().expect("principal id"),
            system,
            valence::use_!(r#"**Test:** Fixture **owner edge** write in `e2e_valence` so the suite can arrange Gauge super-user membership. CI and developers running the suite only."#),
        )
        .await
        .expect("relate super owner");
    created
        .relate_to_member_record(
            principal.id().expect("principal id"),
            system,
            valence::use_!(r#"**Test:** Fixture **member edge** write in `e2e_valence` so the suite can arrange Gauge super-user membership. CI and developers running the suite only."#),
        )
        .await
        .expect("relate super member");
}

async fn demote_admin_from_super_user(system: &Valence) {
    let Some(super_group) = gauge::generated::PermissionGroup::get("super_user_group", system, valence::use_!(r#"**Test:** Fixture **Permission Group** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("get super user group")
    else {
        return;
    };
    let Some(principal) = gauge::generated::PermissionUserPrincipal::get("user:admin", system, valence::use_!(r#"**Test:** Fixture **Permission User Principal** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("get admin principal")
    else {
        return;
    };
    let pid = principal.id().expect("principal id").clone();
    let _ = super_group
        .unrelate_from_member_record(
            &pid,
            system,
            valence::use_!(r#"**Test:** Fixture **member edge** remove in `e2e_valence` so the suite can demote a user from super-user after grants. CI and developers running the suite only."#),
        )
        .await;
    let _ = super_group
        .unrelate_from_owner_record(
            &pid,
            system,
            valence::use_!(r#"**Test:** Fixture **owner edge** remove in `e2e_valence` so the suite can demote a user from super-user after grants. CI and developers running the suite only."#),
        )
        .await;
}

fn chronon_admin_manifest() -> PermissionManifestInput {
    PermissionManifestInput {
        app_id: "chronon".into(),
        domains: vec![PermissionDomainInput {
            key: "chronon".into(),
            name: "Chronon".into(),
            description: "Chronon job scheduling administration".into(),
            permissions: vec![PermissionInput {
                name: "ChrononAdmin".into(),
                description: "Administer Chronon jobs, run-now, and schedule edits".into(),
            }],
        }],
    }
}

async fn grant_chronon_admin(admin_ctx: &Valence, user_id: &str) {
    let perms = service::list_permissions(admin_ctx, None)
        .await
        .expect("list permissions");
    let chronon_admin = perms
        .into_iter()
        .find(|p| p.name == "ChrononAdmin")
        .expect("ChrononAdmin after sync");
    service::grant_permission_to_user(&chronon_admin.id, user_id, admin_ctx)
        .await
        .expect("grant ChrononAdmin");
}

async fn bootstrap_chronon_fixtures(
    coordinator: &dyn ChrononCoordinatorBackend,
    valence: &Valence,
) -> anyhow::Result<FixtureIds> {
    let mut job = Job::new(E2E_JOB_NAME, E2E_SCRIPT_NAME);
    job.schedule_kind = ScheduleKind::Cron;
    job.cron_expr = Some("0 * * * *".into());
    job.enabled = true;
    job.script_sig_hash = "e2e-sig".into();
    let job_id = job.job_id.clone();
    coordinator
        .upsert_job_with_valence(valence, job)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let run_id = coordinator
        .run_now(&job_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(FixtureIds {
        script_name: E2E_SCRIPT_NAME.into(),
        job_id,
        job_name: E2E_JOB_NAME.into(),
        run_id,
    })
}

/// Build shared Valence/Higgs/Chronon once and seed baseline fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();

    let backend: Arc<dyn DatabaseBackend> = Arc::new(InMemoryBackend::new());
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        gauge::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions {
            register_alias_engine_id: Some(SQLITE_ENGINE_ID),
        },
    );
    router.register(
        router_key(gauge::embedded_surreal::LOGICAL_NAME, SQLITE_ENGINE_ID),
        Arc::clone(&backend),
    );
    let router = Arc::new(router);
    let default_key = router_key(gauge::embedded_surreal::LOGICAL_NAME, MEM_ENGINE_ID);

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_chronon_host".into(),
        })
        .build()
        .expect("e2e Valence");

    seed_user("admin", true, &system).await;
    seed_user("outsider", true, &system).await;
    seed_user("unverified", false, &system).await;
    seed_super_user_with_member(&system, "admin").await;

    sync_permission_manifests(&system, &[chronon_admin_manifest()])
        .await
        .expect("sync ChrononAdmin manifest");

    let admin_ctx = system.with_actor(Actor::User {
        user_id: "admin".to_string(),
    });
    grant_chronon_admin(&admin_ctx, "admin").await;
    grant_chronon_admin(&admin_ctx, "unverified").await;
    demote_admin_from_super_user(&system).await;

    let local = Arc::new(LocalBackend::default());
    let chronon_backend: Arc<dyn ChrononCoordinatorBackend> = Arc::clone(&local) as _;
    let registry = Arc::new(e2e_script_registry());

    let fixtures = bootstrap_chronon_fixtures(chronon_backend.as_ref(), &admin_ctx)
        .await
        .expect("bootstrap chronon fixtures");

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    // Lab host provides Chronon via Leptos `provide_context` (see main.rs).
    // Skip HiggsConfig::chronon to avoid version skew with higgs/chronon.
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        chronon_backend,
        registry,
        default_backend_key: default_key,
        fixtures: Mutex::new(fixtures),
    });
    let _ = E2E_STATE.set(state);
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_chronon_backend() -> Arc<dyn ChrononCoordinatorBackend> {
    Arc::clone(&state().chronon_backend)
}

pub fn e2e_registry() -> Arc<ScriptRegistry> {
    Arc::clone(&state().registry)
}

pub fn e2e_fixtures() -> FixtureIds {
    state().fixtures.lock().expect("fixtures").clone()
}

pub fn store_fixtures(fixtures: FixtureIds) {
    *state().fixtures.lock().expect("fixtures") = fixtures;
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}

pub fn e2e_admin_valence() -> Valence {
    e2e_system_valence().with_actor(Actor::User {
        user_id: "admin".into(),
    })
}
