//! Server-side connection use cases. Local calls never acquire provider authority.
use crate::{connection_settings::ConnectionKind, local::LocalError};
use async_trait::async_trait;
use kusi_domain::{local::LocalUser, managed_integrations::VerificationMode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyInput {
    pub expected_revision: i64,
    pub mode: VerificationMode,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleInput {
    pub expected_revision: i64,
    pub verification_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct VerificationSummary {
    pub id: Option<Uuid>,
    pub profile_revision: i64,
    pub adapter_version: String,
    pub mode: Option<VerificationMode>,
    pub status: String,
    pub checked_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub scope_summary: String,
    pub error_code: Option<String>,
    pub runtime_active: bool,
}
#[async_trait]
pub trait ManagedConnections: Send + Sync {
    async fn verify_fixture(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<VerificationSummary, LocalError>;
    async fn connection_status(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
    ) -> Result<VerificationSummary, LocalError>;
    async fn disable_connection(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<VerificationSummary, LocalError>;
}
pub fn owner(actor: &LocalUser) -> Result<(), LocalError> {
    if actor.disabled || !actor.role.is_owner() {
        Err(LocalError::Forbidden)
    } else {
        Ok(())
    }
}
pub async fn verify<R: ManagedConnections>(
    repository: &R,
    actor: &LocalUser,
    kind: ConnectionKind,
    input: &VerifyInput,
) -> Result<VerificationSummary, LocalError> {
    owner(actor)?;
    if input.expected_revision < 1 {
        return Err(LocalError::Conflict);
    }
    if input.mode == VerificationMode::Provider {
        return Err(LocalError::Forbidden);
    }
    repository
        .verify_fixture(actor, kind, input.expected_revision)
        .await
}
pub async fn disable<R: ManagedConnections>(
    repository: &R,
    actor: &LocalUser,
    kind: ConnectionKind,
    input: &LifecycleInput,
) -> Result<VerificationSummary, LocalError> {
    owner(actor)?;
    if input.expected_revision < 1 {
        return Err(LocalError::Conflict);
    }
    repository
        .disable_connection(actor, kind, input.expected_revision)
        .await
}
pub fn enable_local(actor: &LocalUser, input: &LifecycleInput) -> Result<(), LocalError> {
    owner(actor)?;
    if input.expected_revision < 1 {
        return Err(LocalError::Conflict);
    }
    Err(LocalError::Forbidden)
}

/// A fixture connection is deliberately a distinct authority from a provider connection.
#[async_trait]
pub trait FixtureConnectionResolver: Send + Sync {
    async fn fixture_revision(
        &self,
        actor: &crate::adaptive::Actor,
        kind: ConnectionKind,
    ) -> Result<i64, kusi_domain::adaptive::Error>;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedQueryInput {
    pub kind: ConnectionKind,
    pub version: String,
    pub operation: String,
    pub input: serde_json::Value,
    pub expected_revision: i64,
}
