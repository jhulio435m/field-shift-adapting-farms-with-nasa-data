use crate::PgOrganizationRepository;
use kusi_application::{
    external_access::{IdentityAccessRepository, Provision, ProvisionRequest},
    integrations::ExecutionError,
};
use kusi_domain::{Capability, external_access::ResourceRead};
use serde_json::Value;
use uuid::Uuid;

impl PgOrganizationRepository {
    pub(crate) async fn authorize_external_resource(
        &self,
        organization: Uuid,
        principal: &str,
        capability: &Capability,
        input: &Value,
    ) -> Result<Option<ResourceRead>, ExecutionError> {
        let protected: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM external_resource_policies WHERE organization_id=$1 AND adapter=$2)",
        ).bind(organization).bind(&capability.adapter).fetch_one(&self.pool).await.map_err(|_| ExecutionError::Unavailable)?;
        if !protected {
            return Ok(None);
        }
        let scope: ResourceRead =
            serde_json::from_value(input.clone()).map_err(|_| ExecutionError::InvalidData)?;
        if !scope.is_valid() {
            return Err(ExecutionError::InvalidData);
        }
        let allowed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM external_identity_links l \
             JOIN organization_api_keys k ON k.organization_id=l.organization_id AND k.key_hash=l.principal_hash \
             JOIN external_resource_grants g ON g.link_id=l.id \
             WHERE l.organization_id=$1 AND l.principal_hash=$2 AND l.adapter=$3 \
             AND l.revoked_at IS NULL AND l.expires_at > now() \
             AND k.revoked_at IS NULL AND (k.expires_at IS NULL OR k.expires_at > now()) \
             AND g.capability=$4 AND g.version=$5 AND g.resource_ref=$6 \
             AND g.revoked_at IS NULL AND g.expires_at > now())",
        ).bind(organization).bind(principal).bind(&capability.adapter)
            .bind(&capability.identifier).bind(&capability.version).bind(&scope.resource_ref)
            .fetch_one(&self.pool).await.map_err(|_| ExecutionError::Unavailable)?;
        if !allowed {
            return Err(ExecutionError::Forbidden);
        }
        Ok(Some(scope))
    }
}

#[async_trait::async_trait]
impl IdentityAccessRepository for PgOrganizationRepository {
    async fn apply(&self, request: &ProvisionRequest) -> Result<(), ExecutionError> {
        request.validate()?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ExecutionError::Unavailable)?;
        // Operator provisioning cannot manufacture a tenant or bind another tenant's key.
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM organization_api_keys WHERE organization_id=$1 AND key_hash=$2 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>now()))")
            .bind(request.organization).bind(&request.principal_hash).fetch_one(&mut *tx).await.map_err(|_| ExecutionError::Unavailable)?;
        if !active
            && !matches!(
                request.action,
                Provision::RevokeLink | Provision::RevokeGrant { .. }
            )
        {
            return Err(ExecutionError::Forbidden);
        }
        let changed = match &request.action {
            Provision::Protect => {
                sqlx::query("INSERT INTO external_resource_policies(organization_id,adapter) VALUES($1,$2) ON CONFLICT DO NOTHING")
                    .bind(request.organization).bind(&request.adapter).execute(&mut *tx).await
            }
            Provision::Link { subject, ttl_seconds, .. } => {
                // No upsert: a revoked identity must never silently inherit old grants.
                sqlx::query("INSERT INTO external_identity_links(organization_id,principal_hash,adapter,external_subject,expires_at) VALUES($1,$2,$3,$4,now()+make_interval(secs=>$5::double precision)) ON CONFLICT DO NOTHING")
                    .bind(request.organization).bind(&request.principal_hash).bind(&request.adapter).bind(subject).bind(f64::from(*ttl_seconds)).execute(&mut *tx).await
            }
            Provision::Grant { capability, version, resource_ref, ttl_seconds, .. } => {
                sqlx::query("INSERT INTO external_resource_grants(link_id,capability,version,resource_ref,expires_at) SELECT l.id,$4,$5,$6,LEAST(l.expires_at,now()+make_interval(secs=>$7::double precision)) FROM external_identity_links l JOIN capabilities c ON c.organization_id=l.organization_id AND c.adapter=l.adapter AND c.identifier=$4 AND c.version=$5 AND c.enabled WHERE l.organization_id=$1 AND l.principal_hash=$2 AND l.adapter=$3 AND l.revoked_at IS NULL AND l.expires_at>now() ON CONFLICT DO NOTHING")
                    .bind(request.organization).bind(&request.principal_hash).bind(&request.adapter).bind(capability).bind(version).bind(resource_ref).bind(f64::from(*ttl_seconds)).execute(&mut *tx).await
            }
            Provision::RevokeLink => {
                sqlx::query("UPDATE external_identity_links SET revoked_at=COALESCE(revoked_at,now()) WHERE organization_id=$1 AND principal_hash=$2 AND adapter=$3")
                    .bind(request.organization).bind(&request.principal_hash).bind(&request.adapter).execute(&mut *tx).await
            }
            Provision::RevokeGrant { capability, version, resource_ref } => {
                sqlx::query("UPDATE external_resource_grants g SET revoked_at=COALESCE(g.revoked_at,now()) FROM external_identity_links l WHERE g.link_id=l.id AND l.organization_id=$1 AND l.principal_hash=$2 AND l.adapter=$3 AND g.capability=$4 AND g.version=$5 AND g.resource_ref=$6")
                    .bind(request.organization).bind(&request.principal_hash).bind(&request.adapter).bind(capability).bind(version).bind(resource_ref).execute(&mut *tx).await
            }
        }.map_err(|_| ExecutionError::Unavailable)?;
        if changed.rows_affected() == 0 && !matches!(request.action, Provision::Protect) {
            return Err(ExecutionError::Forbidden);
        }
        // Existing audit contains only random event, tenant, static operation and result.
        sqlx::query("INSERT INTO execution_audit(event_id,organization_id,capability,version,outcome) VALUES($1,$2,$3,'1','succeeded')")
            .bind(Uuid::new_v4()).bind(request.organization).bind(match request.action {
                Provision::Protect => "external_access.policy_protected", Provision::Link { .. } => "external_access.identity_linked",
                Provision::Grant { .. } => "external_access.resource_granted", Provision::RevokeLink => "external_access.identity_revoked",
                Provision::RevokeGrant { .. } => "external_access.resource_revoked",
            }).execute(&mut *tx).await.map_err(|_| ExecutionError::Unavailable)?;
        tx.commit().await.map_err(|_| ExecutionError::Unavailable)
    }
}
