use crate::local::LocalStore;
use kusi_application::{
    channel_operations::{BindingSummary, ChannelOperations, ChannelSummary},
    local::LocalError,
    managed_integrations::owner,
};
use kusi_domain::local::LocalUser;
use uuid::Uuid;
#[async_trait::async_trait]
impl ChannelOperations for LocalStore {
    async fn telegram_bindings(
        &self,
        actor: &LocalUser,
        cursor: Option<Uuid>,
    ) -> Result<ChannelSummary, LocalError> {
        owner(actor)?;
        let rows=sqlx::query_as::<_,(Uuid,Uuid,i64,i64,bool)>("SELECT id,user_id,version,extract(epoch FROM expires_at)::bigint,revoked_at IS NOT NULL FROM local_channel_bindings WHERE organization_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT 50").bind(actor.organization_id).bind(cursor).fetch_all(&self.pool).await.map_err(|_|LocalError::Unavailable)?;
        Ok(ChannelSummary {
            mode: "local_no_delivery",
            runtime_active: false,
            bindings: rows
                .into_iter()
                .map(
                    |(id, user_id, version, expires_at, revoked)| BindingSummary {
                        id,
                        user_id,
                        version,
                        expires_at,
                        revoked,
                    },
                )
                .collect(),
        })
    }
    async fn revoke_binding(
        &self,
        actor: &LocalUser,
        id: Uuid,
        version: i64,
    ) -> Result<(), LocalError> {
        owner(actor)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| LocalError::Unavailable)?;
        let row:Option<(i64,bool)>=sqlx::query_as("SELECT version,revoked_at IS NOT NULL FROM local_channel_bindings WHERE organization_id=$1 AND id=$2 FOR UPDATE").bind(actor.organization_id).bind(id).fetch_optional(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        let (current, revoked) = row.ok_or(LocalError::NotFound)?;
        if !revoked && current != version {
            return Err(LocalError::Conflict);
        }
        if !revoked {
            sqlx::query("UPDATE local_channel_bindings SET revoked_at=now(),version=version+1 WHERE id=$1 AND organization_id=$2").bind(id).bind(actor.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
            // Drop retained conversational context for this binding. It must not be inherited after re-pairing.
            sqlx::query("UPDATE local_conversations c SET context='{}',context_expires_at=now() FROM local_channel_bindings b WHERE b.id=$1 AND b.organization_id=$2 AND c.organization_id=b.organization_id AND c.channel_id=b.channel_id AND c.sender=b.sender AND c.user_id=b.user_id").bind(id).bind(actor.organization_id).execute(&mut *tx).await.map_err(|_|LocalError::Unavailable)?;
        }
        tx.commit().await.map_err(|_| LocalError::Unavailable)
    }
}
