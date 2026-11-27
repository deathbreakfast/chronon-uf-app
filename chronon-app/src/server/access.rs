//! Per-job access checks shared by Chronon server functions.
//!
//! Every check runs as the session actor through the host's
//! `uf_product::access::ResourceAccessBackend` (fail-closed when none is wired). Nothing
//! elevates.

use std::collections::HashSet;
use std::sync::Arc;

use leptos::prelude::ServerFnError;
use uf_product::access::{ResourceAccessBackend, ResourceRef};

use crate::permissions::{access_label, job_ref};

/// The host's Chronon admin group: default owner of code-declared jobs and an "Owned by"
/// choice on the create form. Hosts provide it with `provide_context` next to the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChrononAdminGroup {
    /// Gauge group id (the group that holds `ChrononAdmin`).
    pub id: String,
    /// Display name, such as "Chronon admins".
    pub label: String,
}

/// A job as the access views name it, from the stored job.
pub(super) fn core_job_ref(job: &chronon_coordinator::models::Job) -> ResourceRef {
    job_ref(&chronon_backend::core_job_resource_id(job), &job.job_name)
}

/// Require `action` on `job` as the session actor.
///
/// # Errors
///
/// `permission_denied::chronon_job.{id}.{action}` when the reader lacks it.
pub(super) async fn require_job(
    job: &chronon_coordinator::models::Job,
    action: &str,
) -> Result<(), ServerFnError> {
    uf_product::access::require(&core_job_ref(job), action).await
}

/// What the reader holds on one job.
#[allow(clippy::struct_excessive_bools)] // one flag per Gauge action
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Held {
    pub view: bool,
    pub run: bool,
    pub edit: bool,
    pub maintain: bool,
}

impl Held {
    /// The role label: Owner, Can edit, Can run, Can view or No access.
    pub(super) fn label(self) -> &'static str {
        let mut held = Vec::new();
        for (has, action) in [
            (self.view, "View"),
            (self.run, "Run"),
            (self.edit, "Edit"),
            (self.maintain, "Maintain"),
        ] {
            if has {
                held.push(action);
            }
        }
        access_label(&held)
    }
}

/// Whether the reader holds `action`; a missing backend or a failed check reads as no.
async fn reader_can(
    backend: Option<&Arc<dyn ResourceAccessBackend>>,
    resource: &ResourceRef,
    action: &str,
) -> bool {
    let Some(backend) = backend else {
        return false;
    };
    match backend.can(resource, action).await {
        Ok(can) => can,
        Err(e) => {
            leptos::logging::warn!(
                "chronon access check failed id={} action={action}: {e}",
                resource.id
            );
            false
        }
    }
}

/// Every action the reader holds on `job`.
pub(super) async fn held_on(job: &chronon_coordinator::models::Job) -> Held {
    let backend = uf_product::access::use_resource_access_backend();
    let backend = backend.as_ref();
    let resource = core_job_ref(job);
    Held {
        view: reader_can(backend, &resource, "View").await,
        run: reader_can(backend, &resource, "Run").await,
        edit: reader_can(backend, &resource, "Edit").await,
        maintain: reader_can(backend, &resource, "Maintain").await,
    }
}

/// Whether the reader holds `action` on `job`.
pub(super) async fn can_on(job: &chronon_coordinator::models::Job, action: &str) -> bool {
    let backend = uf_product::access::use_resource_access_backend();
    reader_can(backend.as_ref(), &core_job_ref(job), action).await
}

/// Ids of the jobs the reader can view, out of `jobs`.
pub(super) async fn viewable_job_ids(jobs: &[chronon_coordinator::models::Job]) -> HashSet<String> {
    let mut ids = HashSet::new();
    for job in jobs {
        if can_on(job, "View").await {
            ids.insert(job.job_id.clone());
        }
    }
    ids
}

/// Keep runs of jobs the reader can view; runs with no job stay hidden.
pub(super) fn keep_viewable_runs(
    runs: &mut Vec<chronon_coordinator::models::Run>,
    viewable: &HashSet<String>,
) {
    runs.retain(|r| r.job_id.as_deref().is_some_and(|id| viewable.contains(id)));
}

/// The host's admin group, when it provided one.
pub(super) fn admin_group() -> Option<ChrononAdminGroup> {
    leptos::context::use_context::<ChrononAdminGroup>()
}
