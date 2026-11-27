//! Per-resource access over Gauge for the Playwright host.
//!
//! Implements `uf_product::access::ResourceAccessBackend` for Chronon jobs. Every call runs
//! as the session actor (`Higgs::from_request`); nothing elevates to System. Gauge's own
//! implementation of this backend will replace it once it ships, so this one covers what the
//! Chronon pages and specs use: checks, the Access card view, role changes, team and group
//! search, and access requests.

use std::collections::BTreeMap;

use chronon_app::permissions::{CHRONON_JOB, CHRONON_JOB_KIND};
use gauge::resource_permissions::{
    owners_group_id, permission_name, permission_record_id, remove_principal, roles,
    set_principal_role, ResourceAction, ResourceKindDescriptor, ResourcePermissionError,
};
use gauge::types::{
    PermissionRequestCreateInput, PermissionRequestTargetKind, PrincipalKind as GaugeKind,
    PrincipalRefDto,
};
use uf_product::access::{
    AccessEntry, AccessError, AccessView, PrincipalKind, PrincipalRef, PrincipalResults, RequestId,
    ResourceAccessBackend, ResourceRef, Role,
};
use valence::Valence;

/// Gauge-backed access for Chronon jobs; `admin_group_id` reads as Default owner.
pub struct GaugeResourceAccess {
    admin_group_id: &'static str,
}

impl GaugeResourceAccess {
    pub const fn new(admin_group_id: &'static str) -> Self {
        Self { admin_group_id }
    }
}

fn backend_err(op: &str, e: impl std::fmt::Display) -> AccessError {
    AccessError::Backend {
        op: op.to_string(),
        message: e.to_string(),
    }
}

fn descriptor(resource: &ResourceRef) -> Result<ResourceKindDescriptor, AccessError> {
    match resource.kind.as_str() {
        CHRONON_JOB_KIND => Ok(CHRONON_JOB),
        other => Err(backend_err(
            "resolve resource kind",
            format!("unknown kind {other}"),
        )),
    }
}

fn action_of(name: &str) -> Option<ResourceAction> {
    match name {
        "View" => Some(ResourceAction::View),
        "Run" => Some(ResourceAction::Run),
        "Edit" => Some(ResourceAction::Edit),
        "Delete" => Some(ResourceAction::Delete),
        "Maintain" => Some(ResourceAction::Maintain),
        _ => None,
    }
}

fn to_gauge_role(role: &Role) -> Option<roles::Role> {
    match role {
        Role::View => Some(roles::Role::View),
        Role::Run => Some(roles::Role::Run),
        Role::ReadUnmasked => Some(roles::Role::ReadUnmasked),
        Role::Edit => Some(roles::Role::Edit),
        Role::Owner => Some(roles::Role::Owner),
        Role::Custom(_) => None,
    }
}

fn from_gauge_role(role: roles::Role) -> Role {
    match role {
        roles::Role::View => Role::View,
        roles::Role::Run => Role::Run,
        roles::Role::ReadUnmasked => Role::ReadUnmasked,
        roles::Role::Edit => Role::Edit,
        roles::Role::Owner => Role::Owner,
        roles::Role::Custom(actions) => Role::Custom(actions),
    }
}

fn role_from_grants(kind: ResourceKindDescriptor, grants: &[ResourceAction]) -> Option<Role> {
    if grants.is_empty() {
        return None;
    }
    Some(roles::role_of(kind, grants).map_or_else(
        || Role::Custom(grants.iter().map(|a| a.as_str().to_string()).collect()),
        from_gauge_role,
    ))
}

fn principal_from_dto(dto: &PrincipalRefDto) -> PrincipalRef {
    let kind = match dto.kind {
        GaugeKind::Group => PrincipalKind::Group,
        GaugeKind::ServiceUser => PrincipalKind::ServiceUser {
            owner: dto.owner_label.clone(),
        },
        _ => PrincipalKind::User,
    };
    PrincipalRef {
        kind,
        id: dto.id.clone(),
        label: dto.label.clone(),
    }
}

fn gauge_kind(principal: &PrincipalRef) -> GaugeKind {
    match principal.kind {
        PrincipalKind::User => GaugeKind::User,
        PrincipalKind::Group | PrincipalKind::Team { .. } => GaugeKind::Group,
        PrincipalKind::ServiceUser { .. } => GaugeKind::ServiceUser,
    }
}

fn map_role_err(resource: &ResourceRef, op: &str, e: ResourcePermissionError) -> AccessError {
    match e {
        ResourcePermissionError::LastOwner { .. } => AccessError::LastOwner {
            resource: resource.label_with_id(),
        },
        other => backend_err(op, other),
    }
}

async fn session() -> Result<Valence, AccessError> {
    let ctx = higgs::Higgs::from_request()
        .await
        .map_err(|e| backend_err("read the session", e))?;
    ctx.valence().map_err(|e| backend_err("build Valence", e))
}

async fn actor_holds(
    v: &Valence,
    kind: ResourceKindDescriptor,
    resource: &ResourceRef,
    action: ResourceAction,
) -> Result<bool, AccessError> {
    let _caller = gauge::instrumentation::PermissionCheckCallerGuard::new("resource_access");
    gauge::service::actor_can(v, &permission_name(kind, &resource.id, action))
        .await
        .map_err(|e| backend_err("check access", e))
}

#[async_trait::async_trait]
impl ResourceAccessBackend for GaugeResourceAccess {
    async fn access(&self, resource: &ResourceRef) -> Result<AccessView, AccessError> {
        let kind = descriptor(resource)?;
        let v = session().await?;

        let mut held = Vec::new();
        for action in kind.actions {
            if actor_holds(&v, kind, resource, *action).await? {
                held.push(*action);
            }
        }

        // Grants per principal. Gauge fills the allow lists only for maintainers, so other
        // readers see their own role and an empty table.
        let owners = owners_group_id(kind, &resource.id);
        let mut grants: BTreeMap<String, (PrincipalRef, Vec<ResourceAction>, bool)> =
            BTreeMap::new();
        for action in kind.actions {
            let record = permission_record_id(kind, &resource.id, *action);
            let detail = gauge::service::get_permission_detail(&record, &v)
                .await
                .map_err(|e| backend_err("load access", e))?;
            for dto in detail.map(|d| d.allow_list).unwrap_or_default() {
                if dto.kind == GaugeKind::Group && dto.id == owners {
                    continue;
                }
                let principal = principal_from_dto(&dto);
                grants
                    .entry(principal.key())
                    .or_insert_with(|| (principal, Vec::new(), false))
                    .1
                    .push(*action);
            }
        }
        if let Some(group) = gauge::service::get_group_detail(&owners, &v)
            .await
            .map_err(|e| backend_err("load owners", e))?
        {
            for dto in group.owner_users.iter().chain(&group.members) {
                let principal = principal_from_dto(dto);
                grants
                    .entry(principal.key())
                    .or_insert_with(|| (principal, Vec::new(), false))
                    .2 = true;
            }
        }

        let entries = grants
            .into_values()
            .filter_map(|(principal, actions, owner)| {
                let role = if owner {
                    Role::Owner
                } else {
                    role_from_grants(kind, &actions)?
                };
                let default_owner =
                    principal.kind == PrincipalKind::Group && principal.id == self.admin_group_id;
                Some(AccessEntry {
                    principal,
                    role,
                    default_owner,
                    umbrella: false,
                })
            })
            .collect();

        Ok(AccessView {
            entries,
            roles: roles::Role::offered(kind)
                .into_iter()
                .map(from_gauge_role)
                .collect(),
            viewer_role: role_from_grants(kind, &held),
            can_maintain: held.contains(&ResourceAction::Maintain),
        })
    }

    async fn set_role(
        &self,
        resource: &ResourceRef,
        principal: &PrincipalRef,
        role: Role,
    ) -> Result<(), AccessError> {
        let kind = descriptor(resource)?;
        let role = to_gauge_role(&role)
            .ok_or_else(|| backend_err("change access", "custom grants are changed in Gauge"))?;
        let v = session().await?;
        set_principal_role(
            &v,
            kind,
            &resource.id,
            (gauge_kind(principal), &principal.id),
            role,
        )
        .await
        .map_err(|e| map_role_err(resource, "change access", e))
    }

    async fn remove(
        &self,
        resource: &ResourceRef,
        principal: &PrincipalRef,
    ) -> Result<(), AccessError> {
        let kind = descriptor(resource)?;
        let v = session().await?;
        remove_principal(
            &v,
            kind,
            &resource.id,
            (gauge_kind(principal), &principal.id),
        )
        .await
        .map_err(|e| map_role_err(resource, "remove access", e))
    }

    async fn search_principals(&self, query: &str) -> Result<PrincipalResults, AccessError> {
        use valence::Model as _;
        let v = session().await?;
        let found = gauge::service::list_groups(&v, Some(query.to_string()))
            .await
            .map_err(|e| backend_err("search groups", e))?;
        let mut teams = Vec::new();
        let mut groups = Vec::new();
        for g in found {
            // Groups another product manages from a team roster show under Teams.
            let managed = gauge::generated::PermissionGroup::get(&g.id, &v, valence::use_!(r"**Test:** When the **Chronon e2e host** searches for people to give access to a job, we **load each matching Gauge group** to tell teams from plain groups. CI and developers running the suite only."))
                .await
                .ok()
                .flatten()
                .is_some_and(|row| row.managed_by().is_some());
            if managed {
                teams.push(PrincipalRef {
                    kind: PrincipalKind::Team {
                        org: None,
                        member_count: u32::try_from(g.members.len()).unwrap_or(u32::MAX),
                    },
                    id: g.id,
                    label: g.name,
                });
            } else {
                groups.push(PrincipalRef {
                    kind: PrincipalKind::Group,
                    id: g.id,
                    label: g.name,
                });
            }
        }
        Ok(PrincipalResults {
            teams,
            groups,
            people: Vec::new(),
        })
    }

    async fn request(
        &self,
        resource: &ResourceRef,
        role: Role,
        reason: &str,
    ) -> Result<RequestId, AccessError> {
        let kind = descriptor(resource)?;
        let v = session().await?;
        let targets: Vec<(PermissionRequestTargetKind, String)> = if role == Role::Owner {
            vec![(
                PermissionRequestTargetKind::Group,
                owners_group_id(kind, &resource.id),
            )]
        } else {
            let wanted = to_gauge_role(&role)
                .and_then(|r| roles::grants_for(kind, &r))
                .ok_or_else(|| {
                    backend_err("request access", "this resource doesn't offer that role")
                })?;
            let mut targets = Vec::new();
            for action in wanted {
                if !actor_holds(&v, kind, resource, action).await? {
                    targets.push((
                        PermissionRequestTargetKind::Permission,
                        permission_record_id(kind, &resource.id, action),
                    ));
                }
            }
            targets
        };

        let mut first = None;
        for (target_kind, target_id) in targets {
            let row = gauge::service::create_permission_request(
                PermissionRequestCreateInput {
                    target_kind,
                    target_id,
                    reason: reason.to_string(),
                    on_behalf_of: None,
                },
                &v,
            )
            .await
            .map_err(|e| backend_err("send the request", e))?;
            first.get_or_insert(row.id);
        }
        first
            .map(RequestId)
            .ok_or_else(|| backend_err("send the request", "you already have this access"))
    }

    async fn can(&self, resource: &ResourceRef, action: &str) -> Result<bool, AccessError> {
        let kind = descriptor(resource)?;
        let Some(action) = action_of(action) else {
            return Ok(false);
        };
        if !kind.actions.contains(&action) {
            return Ok(false);
        }
        let v = session().await?;
        actor_holds(&v, kind, resource, action).await
    }
}
