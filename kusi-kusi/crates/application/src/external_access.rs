//! Operator-only provisioning. An authenticated operator must verify external
//! identity and permission out of band; this is not a remote identity verifier.
use async_trait::async_trait;
use kusi_domain::external_access::valid_reference;
use serde::Deserialize;
use uuid::Uuid;

use crate::integrations::ExecutionError;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Provision {
    Protect,
    Link {
        subject: String,
        ttl_seconds: u32,
        confirmed: bool,
    },
    Grant {
        capability: String,
        version: String,
        resource_ref: String,
        ttl_seconds: u32,
        confirmed: bool,
    },
    RevokeLink,
    RevokeGrant {
        capability: String,
        version: String,
        resource_ref: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionRequest {
    pub organization: Uuid,
    pub principal_hash: String,
    pub adapter: String,
    pub action: Provision,
}

impl ProvisionRequest {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        let ttl_valid = |ttl: u32| (1..=2_592_000).contains(&ttl);
        let scope_valid = |cap: &str, version: &str, resource: &str| {
            !cap.is_empty()
                && cap.len() <= 120
                && !version.is_empty()
                && version.len() <= 32
                && valid_reference(resource)
        };
        let valid = match &self.action {
            Provision::Protect | Provision::RevokeLink => true,
            Provision::Link {
                subject,
                ttl_seconds,
                confirmed,
            } => *confirmed && valid_reference(subject) && ttl_valid(*ttl_seconds),
            Provision::Grant {
                capability,
                version,
                resource_ref,
                ttl_seconds,
                confirmed,
            } => {
                *confirmed
                    && ttl_valid(*ttl_seconds)
                    && scope_valid(capability, version, resource_ref)
            }
            Provision::RevokeGrant {
                capability,
                version,
                resource_ref,
            } => scope_valid(capability, version, resource_ref),
        };
        if self.organization.is_nil()
            || !valid
            || self.adapter.is_empty()
            || self.adapter.len() > 120
            || self.principal_hash.len() != 64
            || !self
                .principal_hash
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(ExecutionError::InvalidData);
        }
        Ok(())
    }
}

#[async_trait]
pub trait IdentityAccessRepository: Send + Sync {
    async fn apply(&self, request: &ProvisionRequest) -> Result<(), ExecutionError>;
}

pub async fn provision<R: IdentityAccessRepository>(
    repository: &R,
    request: &ProvisionRequest,
) -> Result<(), ExecutionError> {
    request.validate()?;
    repository.apply(request).await
}
