//! Permission manifest and per-job access for the Chronon operations app.
//!
//! Two layers of access:
//!
//! - [`ChrononPermission::ChrononAdmin`], the app-wide permission synced into the `chronon`
//!   domain. It gates creating jobs, and the host's admin group holds it.
//! - A permission bundle per job ([`CHRONON_JOB_KIND`]): View (definition, schedule, runs and
//!   output), Run (Run now and Run again), Edit (schedule and settings, pause and resume) and
//!   Maintain (who has access). Server functions check the job's action as the session actor
//!   through `uf_product::access::require`; the pages show the shared Access card and denied
//!   states.
//!
//! A job a person made has its own bundle, keyed by job id, made in the create call with the
//! creator as maintainer and the chosen "Owned by" group as owner. A job declared in code
//! (System job actor) uses its script's bundle, made at host boot by
//! `ensure_chronon_job_bundles` (SSR) with the host's admin group as default owner.

use uf_product::access::ResourceRef;
use uf_product_macros::UfPermissionManifest;

pub use chronon_backend::CHRONON_JOB_KIND;

/// App-wide admin permission for the Chronon ops UI.
///
/// Synced into the `chronon` domain; gates creating jobs. The host's admin group holds it,
/// and that group is the default owner of every job bundle made at boot.
#[allow(clippy::expl_impl_clone_on_copy)]
#[derive(UfPermissionManifest)]
#[permission_manifest(
    domain_key = "chronon",
    domain_name = "Chronon",
    domain_description = "Chronon job-scheduling administration"
)]
pub enum ChrononPermission {
    /// Create jobs; holders form the default owner group of every code-declared job.
    #[permission(description = "Administer Chronon jobs and immediate run triggers")]
    ChrononAdmin,
}

/// What the jobs list shows for a job the reader holds nothing on.
pub const NO_ACCESS: &str = "No access";

/// A job as the shared access views name it.
///
/// `resource_id` comes from [`chronon_backend::job_resource_id`]; `label` is the job name.
#[must_use]
pub fn job_ref(resource_id: &str, label: &str) -> ResourceRef {
    ResourceRef::new(CHRONON_JOB_KIND, resource_id)
        .label(label)
        .noun("job")
}

/// [`job_ref`] for a job DTO.
#[must_use]
pub fn job_ref_for(job: &crate::Job) -> ResourceRef {
    job_ref(
        &chronon_backend::job_resource_id(job.system_owned, &job.id, &job.script_name),
        &job.name,
    )
}

/// The role label for the actions the reader holds on one job, strongest first.
///
/// `Maintain` reads as Owner. A reader with nothing gets [`NO_ACCESS`].
#[must_use]
pub fn access_label(held: &[&str]) -> &'static str {
    [
        ("Maintain", "Owner"),
        ("Edit", "Can edit"),
        ("Run", "Can run"),
        ("View", "Can view"),
    ]
    .into_iter()
    .find(|(action, _)| held.contains(action))
    .map_or(NO_ACCESS, |(_, label)| label)
}

/// The job named in a permission-denied server error, with the missing action.
///
/// `uf_product::access::require` fails with `permission_denied::chronon_job.{id}.{action}`.
/// Returns `None` for any other error.
#[must_use]
pub fn denied_resource(error: &str) -> Option<(ResourceRef, String)> {
    const PREFIX: &str = "permission_denied::";
    let rest = &error[error.find(PREFIX)? + PREFIX.len()..];
    let rest = rest.split_whitespace().next()?;
    let (name, action) = rest.rsplit_once('.')?;
    let id = name.strip_prefix(CHRONON_JOB_KIND)?.strip_prefix('.')?;
    if id.is_empty() {
        return None;
    }
    Some((job_ref(id, id), action.to_string()))
}

#[cfg(feature = "ssr")]
pub use bundles::{
    ensure_chronon_job_bundles, ensure_created_job_bundle, BundleSetupError, EnsuredBundles,
    CHRONON_JOB,
};

#[cfg(feature = "ssr")]
mod bundles {
    use gauge::resource_permissions::{
        ensure_resource_permission_bundle, ensure_resource_permission_bundle_for_group,
        roles::Role, set_principal_role, ActorId, KindDefaultGroups, ResourceAction,
        ResourceKindDescriptor, ResourcePermissionError, ResourcePermissionSpec, UmbrellaPolicy,
    };
    use gauge::types::PrincipalKind;

    use super::CHRONON_JOB_KIND;

    /// Gauge kind for one job: View, Run, Edit, Maintain.
    ///
    /// Delete arrives with a delete action for run-once jobs; the app has none yet.
    pub const CHRONON_JOB: ResourceKindDescriptor = ResourceKindDescriptor {
        prefix: CHRONON_JOB_KIND,
        display_label: "Chronon job",
        actions: &[
            ResourceAction::View,
            ResourceAction::Run,
            ResourceAction::Edit,
            ResourceAction::Maintain,
        ],
        umbrella: UmbrellaPolicy::KindWide,
        groups: KindDefaultGroups {
            creators: "chronon.job.creators",
            viewers: "chronon.job.viewers",
            editors: "chronon.job.editors",
            operators: "chronon.job.operators",
        },
        create_permission: "ChrononAdmin",
    };

    /// How many bundles [`ensure_chronon_job_bundles`] ensured.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EnsuredBundles {
        /// Bundles for registered scripts (code-declared jobs).
        pub scripts: usize,
        /// Bundles for jobs people made before per-job access existed.
        pub jobs: usize,
    }

    /// Why a bundle couldn't be made.
    #[derive(Debug)]
    pub struct BundleSetupError {
        /// Resource id of the job.
        pub resource_id: String,
        /// Gauge's error.
        pub source: ResourcePermissionError,
    }

    impl std::fmt::Display for BundleSetupError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "chronon access setup: ensure job bundle for {}: {}",
                self.resource_id, self.source
            )
        }
    }

    impl std::error::Error for BundleSetupError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.source)
        }
    }

    /// Ensure a bundle for every registered script and every job a person already made.
    ///
    /// Call at host boot after manifest sync, with a System `Valence` (boot starts as System;
    /// never call this from a request). `admin_group_id` is the group that holds
    /// `ChrononAdmin`: it becomes each bundle's default owner and holds every action, so the
    /// people who administer Chronon today keep their access. Existing jobs people made get
    /// the same treatment, since they predate per-job access. Idempotent; a script that leaves
    /// the code keeps its bundle until someone removes it in Gauge.
    ///
    /// # Errors
    ///
    /// [`BundleSetupError`] when Gauge refuses a bundle: `system` is not System,
    /// `admin_group_id` doesn't exist, or a write fails.
    pub async fn ensure_chronon_job_bundles(
        system: &valence::Valence,
        script_names: &[String],
        jobs: &[chronon_coordinator::models::Job],
        admin_group_id: &str,
    ) -> Result<EnsuredBundles, BundleSetupError> {
        let mut ensured = EnsuredBundles {
            scripts: 0,
            jobs: 0,
        };
        for name in script_names {
            ensure_for_group(
                system,
                name,
                &format!("Chronon script {name}"),
                admin_group_id,
            )
            .await?;
            ensured.scripts += 1;
        }
        for job in jobs {
            if chronon_backend::is_system_actor(&job.actor_json) {
                continue;
            }
            ensure_for_group(
                system,
                &job.job_id,
                &format!("Chronon job {}", job.job_name),
                admin_group_id,
            )
            .await?;
            ensured.jobs += 1;
        }
        Ok(ensured)
    }

    async fn ensure_for_group(
        system: &valence::Valence,
        resource_id: &str,
        display_name: &str,
        admin_group_id: &str,
    ) -> Result<(), BundleSetupError> {
        ensure_resource_permission_bundle_for_group(
            system,
            CHRONON_JOB,
            resource_id,
            display_name,
            &[],
            admin_group_id,
        )
        .await
        .map(|_| ())
        .map_err(|source| BundleSetupError {
            resource_id: resource_id.to_string(),
            source,
        })
    }

    /// Make the bundle for a job a person just created, as that person.
    ///
    /// The session actor becomes the bundle's maintainer; `owned_by_group`, when given, is
    /// made an owner in the same request, as the same actor. Nothing elevates.
    ///
    /// # Errors
    ///
    /// [`BundleSetupError`] when the session isn't a user, or a Gauge write fails.
    pub async fn ensure_created_job_bundle(
        session: &valence::Valence,
        job_id: &str,
        job_name: &str,
        owned_by_group: Option<&str>,
    ) -> Result<(), BundleSetupError> {
        let err = |source| BundleSetupError {
            resource_id: job_id.to_string(),
            source,
        };
        let actor = ActorId::from_valence(session).ok_or_else(|| {
            err(ResourcePermissionError::MissingMaintainer {
                kind: CHRONON_JOB_KIND.to_string(),
                resource_id: job_id.to_string(),
            })
        })?;
        ensure_resource_permission_bundle(
            session,
            ResourcePermissionSpec {
                kind: CHRONON_JOB,
                resource_id: job_id.to_string(),
                display_name: format!("Chronon job {job_name}"),
                actions: Vec::new(),
                actor,
            },
        )
        .await
        .map_err(err)?;
        if let Some(group) = owned_by_group {
            set_principal_role(
                session,
                CHRONON_JOB,
                job_id,
                (PrincipalKind::Group, group),
                Role::Owner,
            )
            .await
            .map_err(err)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_label_picks_the_strongest_role() {
        assert_eq!(access_label(&["View", "Run", "Maintain"]), "Owner");
        assert_eq!(access_label(&["View", "Run", "Edit"]), "Can edit");
        assert_eq!(access_label(&["Run", "View"]), "Can run");
        assert_eq!(access_label(&["View"]), "Can view");
        assert_eq!(access_label(&[]), NO_ACCESS);
    }

    #[test]
    fn denied_resource_reads_job_ids() {
        let err = "error running server function: permission_denied::chronon_job.0b5c-11.Run";
        let (resource, action) = denied_resource(err).expect("denied job");
        assert_eq!(resource.kind, CHRONON_JOB_KIND);
        assert_eq!(resource.id, "0b5c-11");
        assert_eq!(action, "Run");
    }

    #[test]
    fn denied_resource_ignores_other_errors() {
        assert!(denied_resource("permission_denied::ChrononAdmin").is_none());
        assert!(denied_resource("Chronon IO failed").is_none());
        assert!(denied_resource("permission_denied::photon_topic.x.View").is_none());
    }

    #[test]
    fn job_ref_uses_the_job_name_as_label() {
        let r = job_ref("e2e_echo", "nightly");
        assert_eq!(r.id, "e2e_echo");
        assert_eq!(r.label, "nightly");
        assert_eq!(r.noun, "job");
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn descriptor_names_permissions_per_job() {
        use gauge::resource_permissions::{permission_name, ResourceAction};
        let name = permission_name(CHRONON_JOB, "billing-rollup", ResourceAction::Run);
        assert!(name.starts_with("chronon_job."), "{name}");
        assert!(name.ends_with(".Run"), "{name}");
        assert!(!CHRONON_JOB.actions.contains(&ResourceAction::Delete));
    }
}
