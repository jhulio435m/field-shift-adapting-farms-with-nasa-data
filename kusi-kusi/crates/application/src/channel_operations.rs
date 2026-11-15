//! Channel administration grants no business resource authority.
use crate::{local::LocalError, managed_integrations::owner};
use async_trait::async_trait;
use kusi_domain::local::LocalUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Serialize)]
pub struct BindingSummary {
    pub id: Uuid,
    pub user_id: Uuid,
    pub version: i64,
    pub expires_at: i64,
    pub revoked: bool,
}
#[derive(Serialize)]
pub struct ChannelSummary {
    pub mode: &'static str,
    pub runtime_active: bool,
    pub bindings: Vec<BindingSummary>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingRevision {
    pub expected_version: i64,
}
#[async_trait]
pub trait ChannelOperations: Send + Sync {
    async fn telegram_bindings(
        &self,
        actor: &LocalUser,
        cursor: Option<Uuid>,
    ) -> Result<ChannelSummary, LocalError>;
    async fn revoke_binding(
        &self,
        actor: &LocalUser,
        id: Uuid,
        version: i64,
    ) -> Result<(), LocalError>;
}
pub async fn revoke<R: ChannelOperations>(
    repo: &R,
    actor: &LocalUser,
    id: Uuid,
    input: &BindingRevision,
) -> Result<(), LocalError> {
    owner(actor)?;
    if input.expected_version < 1 {
        return Err(LocalError::Conflict);
    }
    repo.revoke_binding(actor, id, input.expected_version).await
}
