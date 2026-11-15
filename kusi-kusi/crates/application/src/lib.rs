pub mod academic;
pub mod adaptive;
pub mod admin_identity;
pub mod audio;
pub mod bureaucratic_cases;
pub mod connection_settings;
pub mod creative;
pub mod external_access;
pub mod llm;
pub mod local;
pub mod oauth;
pub mod resource_access;
pub mod session_management;
pub mod student_payments;
#[async_trait]
pub trait ChannelInbox: Send + Sync {
    async fn accept(
        &self,
        organization: Uuid,
        channel: Uuid,
        events: &[kusi_domain::ChannelEvent],
    ) -> Result<(), ApplicationError>;
}

pub mod integrations;
use kusi_domain::Capability;
use uuid::Uuid;

#[async_trait]
pub trait CapabilityRegistry: Send + Sync {
    async fn register(
        &self,
        organization: Uuid,
        capability: &Capability,
    ) -> Result<(), ApplicationError>;
    async fn list(&self, organization: Uuid) -> Result<Vec<Capability>, ApplicationError>;
}

use async_trait::async_trait;
use kusi_domain::Organization;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] kusi_domain::OrganizationError),
    #[error("organization slug already exists")]
    Conflict,
    #[error("organization not found")]
    NotFound,
    #[error("storage operation failed")]
    Storage,
}

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn create(
        &self,
        organization: &Organization,
        key_hash: &str,
    ) -> Result<(), ApplicationError>;
    async fn find_by_api_key_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<Organization>, ApplicationError>;
}

pub fn hash_api_key(api_key: &str) -> String {
    hex::encode(Sha256::digest(api_key.as_bytes()))
}

pub struct CreateOrganization<R> {
    repository: R,
}

impl<R: OrganizationRepository> CreateOrganization<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        name: String,
        slug: String,
        api_key: &str,
    ) -> Result<Organization, ApplicationError> {
        let organization = Organization::new(name, slug)?;
        self.repository
            .create(&organization, &hash_api_key(api_key))
            .await?;
        Ok(organization)
    }
}

pub struct GetOrganizationByKey<R> {
    repository: R,
}

impl<R: OrganizationRepository> GetOrganizationByKey<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, api_key: &str) -> Result<Organization, ApplicationError> {
        self.repository
            .find_by_api_key_hash(&hash_api_key(api_key))
            .await?
            .ok_or(ApplicationError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    #[derive(Clone, Default)]
    struct MemoryOrganizations(Arc<Mutex<HashMap<String, Organization>>>);

    #[async_trait]
    impl OrganizationRepository for MemoryOrganizations {
        async fn create(
            &self,
            organization: &Organization,
            key_hash: &str,
        ) -> Result<(), ApplicationError> {
            self.0
                .lock()
                .map_err(|_| ApplicationError::Storage)?
                .insert(key_hash.to_owned(), organization.clone());
            Ok(())
        }

        async fn find_by_api_key_hash(
            &self,
            key_hash: &str,
        ) -> Result<Option<Organization>, ApplicationError> {
            Ok(self
                .0
                .lock()
                .map_err(|_| ApplicationError::Storage)?
                .get(key_hash)
                .cloned())
        }
    }

    #[tokio::test]
    async fn tenant_key_resolves_only_its_organization() {
        let repository = MemoryOrganizations::default();
        let expected = CreateOrganization::new(repository.clone())
            .execute("Demo Org".into(), "demo-org".into(), "tenant-secret-a")
            .await
            .expect("valid organization should be created");
        let resolved = GetOrganizationByKey::new(repository.clone())
            .execute("tenant-secret-a")
            .await
            .expect("key should resolve");
        assert_eq!(resolved.id, expected.id);
        assert!(matches!(
            GetOrganizationByKey::new(repository)
                .execute("tenant-secret-b")
                .await,
            Err(ApplicationError::NotFound)
        ));
    }

    #[test]
    fn stored_api_key_representation_is_a_digest() {
        assert_ne!(hash_api_key("secret-value"), "secret-value");
    }
}

pub mod managed_integrations;

pub mod connector_manifest;

pub mod channel_operations;

pub mod execution_review;

pub mod evaluation;

pub mod authorized_memory;
