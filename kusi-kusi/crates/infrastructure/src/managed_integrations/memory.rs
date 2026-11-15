use crate::local::LocalStore;
use kusi_application::{
    authorized_memory::{
        AuthorizedMemoryRepository, MemoryPreference, MemorySummary, PolicyDecision, PolicyInput,
    },
    hash_api_key,
    local::LocalError,
    managed_integrations::owner,
    resource_access::DemoResource,
};
use kusi_domain::local::LocalUser;
use uuid::Uuid;
fn aad(org: Uuid, actor: Uuid, id: Uuid) -> String {
    format!("authorized-memory:{org}:{actor}:{id}")
}
#[async_trait::async_trait]
impl AuthorizedMemoryRepository for LocalStore {
    async fn memory_preference(&self, actor: &LocalUser) -> Result<MemoryPreference, LocalError> {
        let row:Option<(bool,i64)>=sqlx::query_as("SELECT consent,ttl_seconds FROM authorized_memory_preferences WHERE organization_id=$1 AND actor_id=$2").bind(actor.organization_id).bind(actor.id).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        let (consent, ttl_seconds) = row.unwrap_or((false, 604800));
        Ok(MemoryPreference {
            consent,
            ttl_seconds,
        })
    }
    async fn set_memory_preference(
        &self,
        actor: &LocalUser,
        input: &MemoryPreference,
    ) -> Result<(), LocalError> {
        input.validate()?;
        if actor.disabled {
            return Err(LocalError::Forbidden);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        sqlx::query("INSERT INTO authorized_memory_preferences(organization_id,actor_id,consent,ttl_seconds) VALUES($1,$2,$3,$4) ON CONFLICT(organization_id,actor_id) DO UPDATE SET consent=EXCLUDED.consent,ttl_seconds=EXCLUDED.ttl_seconds").bind(actor.organization_id).bind(actor.id).bind(input.consent).bind(input.ttl_seconds).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        // Opting out erases payload immediately. Reducing TTL cannot lengthen old memories.
        sqlx::query("UPDATE authorized_memories SET ciphertext=CASE WHEN $3 THEN ciphertext ELSE NULL END,deleted_at=CASE WHEN $3 THEN deleted_at ELSE now() END,expires_at=LEAST(expires_at,created_at+make_interval(secs=>$4::double precision)) WHERE organization_id=$1 AND actor_id=$2").bind(actor.organization_id).bind(actor.id).bind(input.consent).bind(input.ttl_seconds as f64).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)
    }
    async fn memory_list(&self, actor: &LocalUser) -> Result<Vec<MemorySummary>, LocalError> {
        let rows=sqlx::query_as::<_,(Uuid,String,i64)>("SELECT m.id,m.source_ref,extract(epoch FROM m.expires_at)::bigint FROM authorized_memories m JOIN authorized_memory_preferences p ON p.organization_id=m.organization_id AND p.actor_id=m.actor_id JOIN local_resource_grants g ON g.organization_id=m.organization_id AND g.user_id=m.actor_id AND g.resource_ref=m.source_ref JOIN local_users u ON u.id=m.actor_id AND u.organization_id=m.organization_id WHERE m.organization_id=$1 AND m.actor_id=$2 AND u.role='viewer' AND NOT u.disabled AND p.consent AND m.deleted_at IS NULL AND m.ciphertext IS NOT NULL AND m.expires_at>now() ORDER BY m.id LIMIT 50").bind(actor.organization_id).bind(actor.id).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|(id, source_ref, expires_at)| MemorySummary {
                id,
                source_ref,
                expires_at,
            })
            .collect())
    }
    async fn forget_memory(&self, actor: &LocalUser, id: Uuid) -> Result<(), LocalError> {
        let changed=sqlx::query("UPDATE authorized_memories SET deleted_at=COALESCE(deleted_at,now()),ciphertext=NULL WHERE organization_id=$1 AND actor_id=$2 AND id=$3").bind(actor.organization_id).bind(actor.id).bind(id).execute(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        if changed.rows_affected() == 0 {
            return Err(LocalError::NotFound);
        }
        Ok(())
    }
    async fn remember_resource(
        &self,
        actor: &LocalUser,
        result: &DemoResource,
    ) -> Result<(), LocalError> {
        // Serialize consent withdrawal and grant revocation with memory creation.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let permitted:Option<i64>=sqlx::query_scalar("SELECT p.ttl_seconds FROM authorized_memory_preferences p JOIN local_resource_grants g ON g.organization_id=p.organization_id AND g.user_id=p.actor_id JOIN local_users u ON u.id=p.actor_id AND u.organization_id=p.organization_id WHERE p.organization_id=$1 AND p.actor_id=$2 AND g.resource_ref=$3 AND g.capability='demo.resource.read' AND p.consent AND u.role='viewer' AND NOT u.disabled FOR SHARE OF p,g,u").bind(actor.organization_id).bind(actor.id).bind(&result.resource_ref).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        let Some(ttl_seconds) = permitted else {
            return Ok(());
        };
        let id = Uuid::new_v4();
        let raw = serde_json::to_vec(result).map_err(|_| LocalError::Unavailable)?;
        let version = hash_api_key(&String::from_utf8_lossy(&raw));
        let ciphertext = self
            .connection_vault
            .encrypt(&aad(actor.organization_id, actor.id, id), &raw)
            .map_err(|_| LocalError::Unavailable)?;
        // Do not upsert: explicit deletion/expiration remains effective until the actor opts in again.
        sqlx::query("INSERT INTO authorized_memories(id,organization_id,actor_id,source_ref,source_version,ciphertext,expires_at) SELECT $1,$2,$3,$4,$5,$6,now()+make_interval(secs=>$7::double precision) FROM local_resource_grants g JOIN local_users u ON u.id=g.user_id AND u.organization_id=g.organization_id WHERE g.organization_id=$2 AND g.user_id=$3 AND g.resource_ref=$4 AND u.role='viewer' AND NOT u.disabled AND EXISTS(SELECT 1 FROM authorized_memory_preferences p WHERE p.organization_id=$2 AND p.actor_id=$3 AND p.consent) ON CONFLICT(organization_id,actor_id,source_ref) DO NOTHING").bind(id).bind(actor.organization_id).bind(actor.id).bind(&result.resource_ref).bind(version).bind(ciphertext).bind(ttl_seconds as f64).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        tx.commit().await.map_err(|_| LocalError::Unavailable)?;
        Ok(())
    }
    async fn recalled_resource(
        &self,
        actor: &LocalUser,
        current: &DemoResource,
    ) -> Result<Option<DemoResource>, LocalError> {
        let row:Option<(Uuid,String,String)>=sqlx::query_as("SELECT m.id,m.source_version,m.ciphertext FROM authorized_memories m JOIN authorized_memory_preferences p ON p.organization_id=m.organization_id AND p.actor_id=m.actor_id JOIN local_resource_grants g ON g.organization_id=m.organization_id AND g.user_id=m.actor_id AND g.resource_ref=m.source_ref JOIN local_users u ON u.id=m.actor_id AND u.organization_id=m.organization_id WHERE m.organization_id=$1 AND m.actor_id=$2 AND m.source_ref=$3 AND u.role='viewer' AND NOT u.disabled AND p.consent AND m.deleted_at IS NULL AND m.ciphertext IS NOT NULL AND m.expires_at>now()").bind(actor.organization_id).bind(actor.id).bind(&current.resource_ref).fetch_optional(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        let Some((id, version, ciphertext)) = row else {
            return Ok(None);
        };
        let raw = serde_json::to_vec(current).map_err(|_| LocalError::Unavailable)?;
        if hash_api_key(&String::from_utf8_lossy(&raw)) != version {
            self.forget_memory(actor, id).await?;
            return Ok(None);
        }
        let bytes = self
            .connection_vault
            .decrypt(&aad(actor.organization_id, actor.id, id), &ciphertext)
            .map_err(|_| LocalError::Unavailable)?;
        let remembered: DemoResource =
            serde_json::from_slice(&bytes).map_err(|_| LocalError::Unavailable)?;
        // Integrity must bind payload to the current source, not just to a row ID.
        if remembered != *current {
            return Err(LocalError::Unavailable);
        }
        Ok(Some(remembered))
    }
    async fn simulate_policy(
        &self,
        actor: &LocalUser,
        input: &PolicyInput,
    ) -> Result<PolicyDecision, LocalError> {
        owner(actor)?;
        if input.operation != "demo.resource.read" || input.resource_reference.len() > 255 {
            return Err(LocalError::Invalid);
        }
        let subject = input.subject_user_id.unwrap_or(actor.id);
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM local_resource_grants g JOIN local_users u ON u.id=g.user_id AND u.organization_id=g.organization_id WHERE g.organization_id=$1 AND g.user_id=$2 AND g.resource_ref=$3 AND g.capability='demo.resource.read' AND u.role='viewer' AND NOT u.disabled)").bind(actor.organization_id).bind(subject).bind(&input.resource_reference).fetch_one(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(PolicyDecision {
            allowed,
            reason: if allowed { "allowed" } else { "not_authorized" },
            simulation: true,
        })
    }
}
