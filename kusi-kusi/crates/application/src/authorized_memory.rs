//! Memory never supplies identity or permissions. It is explicit and bounded.
use crate::local::LocalError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryPreference {
    pub consent: bool,
    pub ttl_seconds: i64,
}
impl MemoryPreference {
    pub fn validate(&self) -> Result<(), LocalError> {
        if (1..=604800).contains(&self.ttl_seconds) {
            Ok(())
        } else {
            Err(LocalError::Invalid)
        }
    }
}
#[derive(Serialize)]
pub struct MemorySummary {
    pub id: Uuid,
    pub source_ref: String,
    pub expires_at: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInput {
    pub operation: String,
    pub resource_reference: String,
    pub subject_user_id: Option<Uuid>,
}
#[derive(Serialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: &'static str,
    pub simulation: bool,
}

#[async_trait::async_trait]
pub trait AuthorizedMemoryRepository: Send + Sync {
    async fn memory_preference(
        &self,
        actor: &kusi_domain::local::LocalUser,
    ) -> Result<MemoryPreference, LocalError>;
    async fn set_memory_preference(
        &self,
        actor: &kusi_domain::local::LocalUser,
        input: &MemoryPreference,
    ) -> Result<(), LocalError>;
    async fn memory_list(
        &self,
        actor: &kusi_domain::local::LocalUser,
    ) -> Result<Vec<MemorySummary>, LocalError>;
    async fn forget_memory(
        &self,
        actor: &kusi_domain::local::LocalUser,
        id: Uuid,
    ) -> Result<(), LocalError>;
    async fn remember_resource(
        &self,
        actor: &kusi_domain::local::LocalUser,
        resource: &crate::resource_access::DemoResource,
    ) -> Result<(), LocalError>;
    async fn recalled_resource(
        &self,
        actor: &kusi_domain::local::LocalUser,
        current: &crate::resource_access::DemoResource,
    ) -> Result<Option<crate::resource_access::DemoResource>, LocalError>;
    async fn simulate_policy(
        &self,
        actor: &kusi_domain::local::LocalUser,
        input: &PolicyInput,
    ) -> Result<PolicyDecision, LocalError>;
}
pub async fn set_preference<R: AuthorizedMemoryRepository>(
    repository: &R,
    actor: &kusi_domain::local::LocalUser,
    input: &MemoryPreference,
) -> Result<(), LocalError> {
    if actor.disabled {
        return Err(LocalError::Forbidden);
    }
    input.validate()?;
    repository.set_memory_preference(actor, input).await
}
pub async fn simulate<R: AuthorizedMemoryRepository>(
    repository: &R,
    actor: &kusi_domain::local::LocalUser,
    input: &PolicyInput,
) -> Result<PolicyDecision, LocalError> {
    crate::managed_integrations::owner(actor)?;
    if input.operation != "demo.resource.read" || input.resource_reference.len() > 255 {
        return Err(LocalError::Invalid);
    }
    repository.simulate_policy(actor, input).await
}
