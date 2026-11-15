use super::LocalStore;
use kusi_application::authorized_memory::AuthorizedMemoryRepository;
use kusi_application::resource_access::{
    DemoResource, RESOURCE_REFS, ResourceAccessAudit, ResourceAccessError, ResourceGrantRepository,
    ResourceReader,
};
use kusi_domain::local::LocalUser;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceGrantRow {
    pub user_id: Uuid,
    pub user_name: String,
    pub resource_ref: String,
    pub resource_label: String,
    pub created_at: i64,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccessibleResource {
    pub resource_ref: String,
    pub display_label: String,
    pub granted: bool,
}
#[derive(FromRow)]
struct GrantDbRow {
    user_id: Uuid,
    user_name: String,
    resource_ref: String,
    created_at: i64,
}

fn fixture(resource: &str) -> Option<DemoResource> {
    let (display_label, summary) = match resource {
        "demo-resource-001" => (
            "Recurso de demostración A",
            "Resumen ficticio para probar permisos por recurso.",
        ),
        "demo-resource-002" => (
            "Recurso de demostración B",
            "Segundo resumen ficticio, independiente del recurso A.",
        ),
        _ => return None,
    };
    Some(DemoResource {
        resource_ref: resource.to_owned(),
        display_label: display_label.to_owned(),
        summary: summary.to_owned(),
    })
}

#[derive(Clone)]
struct PgGrants(sqlx::PgPool);
#[async_trait::async_trait]
impl ResourceGrantRepository for PgGrants {
    async fn allowed(
        &self,
        org: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<bool, ResourceAccessError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_resource_grants g JOIN local_users u ON u.organization_id=g.organization_id AND u.id=g.user_id WHERE g.organization_id=$1 AND g.user_id=$2 AND g.capability='demo.resource.read' AND g.resource_ref=$3 AND u.role='viewer' AND NOT u.disabled)")
            .bind(org).bind(user).bind(resource).fetch_one(&self.0).await.map_err(|_| ResourceAccessError::Unavailable)
    }
    async fn grant(
        &self,
        org: Uuid,
        actor: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<(), ResourceAccessError> {
        sqlx::query("INSERT INTO local_resource_grants(organization_id,user_id,capability,resource_ref,created_by) SELECT $1,u.id,'demo.resource.read',$3,$2 FROM local_users u WHERE u.organization_id=$1 AND u.id=$4 AND u.role='viewer' AND NOT u.disabled ON CONFLICT(organization_id,user_id,capability,resource_ref) DO UPDATE SET created_by=local_resource_grants.created_by")
            .bind(org).bind(actor).bind(resource).bind(user).execute(&self.0).await.map_err(|_| ResourceAccessError::Unavailable).and_then(|result| if result.rows_affected()==0 { Err(ResourceAccessError::NotFound) } else { Ok(()) })
    }
    async fn revoke(
        &self,
        org: Uuid,
        actor: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<(), ResourceAccessError> {
        let result=sqlx::query("DELETE FROM local_resource_grants WHERE organization_id=$1 AND user_id=$2 AND capability='demo.resource.read' AND resource_ref=$3")
            .bind(org).bind(user).bind(resource).execute(&self.0).await.map_err(|_| ResourceAccessError::Unavailable)?;
        if result.rows_affected() == 0 {
            return Err(ResourceAccessError::NotFound);
        }
        sqlx::query("INSERT INTO local_audit(organization_id,user_id,action,target) VALUES($1,$2,'resource_access_revoked',$3)")
            .bind(org).bind(actor).bind(hex::encode(Sha256::digest(resource.as_bytes()))).execute(&self.0).await.map_err(|_| ResourceAccessError::Unavailable)?;
        Ok(())
    }
}
#[derive(Clone)]
struct FixtureReader;
#[async_trait::async_trait]
impl ResourceReader for FixtureReader {
    async fn read(&self, resource: &str) -> Result<Option<DemoResource>, ResourceAccessError> {
        Ok(fixture(resource))
    }
}
#[derive(Clone)]
struct PgAudit(sqlx::PgPool);
#[async_trait::async_trait]
impl ResourceAccessAudit for PgAudit {
    async fn record(
        &self,
        org: Uuid,
        actor: Uuid,
        resource: &str,
        allowed: bool,
        invoked: bool,
    ) -> Result<(), ResourceAccessError> {
        let digest = hex::encode(Sha256::digest(resource.as_bytes()));
        sqlx::query("INSERT INTO local_resource_access_audit(organization_id,actor_id,capability,resource_digest,allowed,adapter_invoked) VALUES($1,$2,'demo.resource.read',$3,$4,$5)")
            .bind(org).bind(actor).bind(digest).bind(allowed).bind(invoked).execute(&self.0).await.map_err(|_| ResourceAccessError::Unavailable)?;
        Ok(())
    }
}

fn map_error(error: ResourceAccessError) -> kusi_application::local::LocalError {
    use kusi_application::local::LocalError;
    match error {
        ResourceAccessError::Forbidden => LocalError::Forbidden,
        ResourceAccessError::NotFound => LocalError::NotFound,
        ResourceAccessError::Invalid => LocalError::Invalid,
        ResourceAccessError::Unavailable => LocalError::Unavailable,
    }
}
impl LocalStore {
    pub async fn resource_catalog(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<AccessibleResource>, kusi_application::local::LocalError> {
        let granted: Vec<String> = sqlx::query_scalar("SELECT resource_ref FROM local_resource_grants WHERE organization_id=$1 AND user_id=$2 AND capability='demo.resource.read'")
            .bind(actor.organization_id).bind(actor.id).fetch_all(&self.pool).await.map_err(|_| kusi_application::local::LocalError::Unavailable)?;
        Ok(RESOURCE_REFS
            .iter()
            .filter(|resource| {
                actor.role.is_owner()
                    || (actor.role.name() == "viewer" && granted.iter().any(|g| g == **resource))
            })
            .filter_map(|resource| {
                fixture(resource).map(|f| AccessibleResource {
                    resource_ref: f.resource_ref,
                    display_label: f.display_label,
                    granted: granted.iter().any(|g| g == *resource),
                })
            })
            .collect())
    }
    pub async fn resource_grants(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<ResourceGrantRow>, kusi_application::local::LocalError> {
        if !actor.role.is_owner() {
            return Err(kusi_application::local::LocalError::Forbidden);
        }
        let rows=sqlx::query_as::<_,GrantDbRow>("SELECT g.user_id,u.display_name AS user_name,g.resource_ref,extract(epoch FROM g.created_at)::bigint AS created_at FROM local_resource_grants g JOIN local_users u ON u.organization_id=g.organization_id AND u.id=g.user_id WHERE g.organization_id=$1 AND u.role='viewer' AND NOT u.disabled ORDER BY u.display_name,g.resource_ref")
            .bind(actor.organization_id).fetch_all(&self.pool).await.map_err(|_| kusi_application::local::LocalError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|r| ResourceGrantRow {
                user_id: r.user_id,
                user_name: r.user_name,
                resource_label: fixture(&r.resource_ref)
                    .map(|f| f.display_label)
                    .unwrap_or("Recurso de demostración".into()),
                resource_ref: r.resource_ref,
                created_at: r.created_at,
            })
            .collect())
    }
    pub async fn grant_resource(
        &self,
        actor: &LocalUser,
        user: Uuid,
        resource: &str,
        verified: bool,
    ) -> Result<(), kusi_application::local::LocalError> {
        if !actor.role.is_owner() {
            return Err(kusi_application::local::LocalError::Forbidden);
        }
        if !verified || fixture(resource).is_none() {
            return Err(kusi_application::local::LocalError::Invalid);
        }
        ResourceGrantRepository::grant(
            &PgGrants(self.pool.clone()),
            actor.organization_id,
            actor.id,
            user,
            resource,
        )
        .await
        .map_err(map_error)?;
        sqlx::query("INSERT INTO local_audit(organization_id,user_id,action,target) VALUES($1,$2,'resource_access_granted',$3)")
            .bind(actor.organization_id).bind(actor.id).bind(hex::encode(Sha256::digest(resource.as_bytes()))).execute(&self.pool).await.map_err(|_|kusi_application::local::LocalError::Unavailable)?;
        Ok(())
    }
    pub async fn revoke_resource(
        &self,
        actor: &LocalUser,
        user: Uuid,
        resource: &str,
    ) -> Result<(), kusi_application::local::LocalError> {
        if !actor.role.is_owner() {
            return Err(kusi_application::local::LocalError::Forbidden);
        }
        if fixture(resource).is_none() {
            return Err(kusi_application::local::LocalError::Invalid);
        }
        ResourceGrantRepository::revoke(
            &PgGrants(self.pool.clone()),
            actor.organization_id,
            actor.id,
            user,
            resource,
        )
        .await
        .map_err(map_error)
    }
    pub async fn execute_resource(
        &self,
        actor: &LocalUser,
        resource: &str,
    ) -> Result<DemoResource, kusi_application::local::LocalError> {
        let result = kusi_application::resource_access::execute_read(
            &PgGrants(self.pool.clone()),
            &FixtureReader,
            &PgAudit(self.pool.clone()),
            actor.organization_id,
            actor.id,
            resource,
        )
        .await
        .map_err(map_error)?;
        let recalled = self.recalled_resource(actor, &result).await?;
        self.remember_resource(actor, &result).await?;
        // Check current grants again after memory I/O before returning any evidence.
        if !ResourceGrantRepository::allowed(
            &PgGrants(self.pool.clone()),
            actor.organization_id,
            actor.id,
            resource,
        )
        .await
        .map_err(map_error)?
        {
            return Err(kusi_application::local::LocalError::Forbidden);
        }
        Ok(recalled.unwrap_or(result))
    }
    pub async fn resource_audit(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<serde_json::Value>, kusi_application::local::LocalError> {
        if !actor.role.is_owner() {
            return Err(kusi_application::local::LocalError::Forbidden);
        }
        sqlx::query("SELECT actor_id,capability,resource_digest,allowed,adapter_invoked,extract(epoch FROM created_at)::bigint AS created_at FROM local_resource_access_audit WHERE organization_id=$1 ORDER BY created_at DESC LIMIT 100")
            .bind(actor.organization_id).fetch_all(&self.pool).await.map_err(|_|kusi_application::local::LocalError::Unavailable)?
            .into_iter().map(|row| Ok(serde_json::json!({"actor_id":row.try_get::<Uuid,_>("actor_id").map_err(|_|kusi_application::local::LocalError::Unavailable)?,"capability":row.try_get::<String,_>("capability").map_err(|_|kusi_application::local::LocalError::Unavailable)?,"resource_digest":row.try_get::<String,_>("resource_digest").map_err(|_|kusi_application::local::LocalError::Unavailable)?,"allowed":row.try_get::<bool,_>("allowed").map_err(|_|kusi_application::local::LocalError::Unavailable)?,"adapter_invoked":row.try_get::<bool,_>("adapter_invoked").map_err(|_|kusi_application::local::LocalError::Unavailable)?,"created_at":row.try_get::<i64,_>("created_at").map_err(|_|kusi_application::local::LocalError::Unavailable)?}))).collect()
    }
}
