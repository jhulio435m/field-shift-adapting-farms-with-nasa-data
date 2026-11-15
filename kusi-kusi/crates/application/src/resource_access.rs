use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const READ_CAPABILITY: &str = "demo.resource.read";
pub const RESOURCE_REFS: [&str; 2] = ["demo-resource-001", "demo-resource-002"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResourceAccessError {
    #[error("access denied")]
    Forbidden,
    #[error("resource not found")]
    NotFound,
    #[error("invalid request")]
    Invalid,
    #[error("storage or fixture unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemoResource {
    pub resource_ref: String,
    pub display_label: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceGrant {
    pub user_id: Uuid,
    pub user_name: String,
    pub resource_ref: String,
    pub resource_label: String,
    pub created_at: i64,
}

#[async_trait]
pub trait ResourceGrantRepository: Send + Sync {
    async fn allowed(
        &self,
        organization: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<bool, ResourceAccessError>;
    async fn grant(
        &self,
        organization: Uuid,
        actor: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<(), ResourceAccessError>;
    async fn revoke(
        &self,
        organization: Uuid,
        actor: Uuid,
        user: Uuid,
        resource: &str,
    ) -> Result<(), ResourceAccessError>;
}

#[async_trait]
pub trait ResourceReader: Send + Sync {
    async fn read(&self, resource: &str) -> Result<Option<DemoResource>, ResourceAccessError>;
}

#[async_trait]
pub trait ResourceAccessAudit: Send + Sync {
    async fn record(
        &self,
        organization: Uuid,
        actor: Uuid,
        resource: &str,
        allowed: bool,
        adapter_invoked: bool,
    ) -> Result<(), ResourceAccessError>;
}

pub async fn execute_read<G: ResourceGrantRepository, R: ResourceReader, A: ResourceAccessAudit>(
    grants: &G,
    reader: &R,
    audit: &A,
    organization: Uuid,
    actor: Uuid,
    resource: &str,
) -> Result<DemoResource, ResourceAccessError> {
    if !RESOURCE_REFS.contains(&resource) {
        audit
            .record(organization, actor, resource, false, false)
            .await?;
        return Err(ResourceAccessError::NotFound);
    }
    let allowed = grants.allowed(organization, actor, resource).await?;
    if !allowed {
        audit
            .record(organization, actor, resource, false, false)
            .await?;
        return Err(ResourceAccessError::Forbidden);
    }
    let result = reader
        .read(resource)
        .await?
        .ok_or(ResourceAccessError::Unavailable)?;
    audit
        .record(organization, actor, resource, true, true)
        .await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Grants(bool);
    #[async_trait]
    impl ResourceGrantRepository for Grants {
        async fn allowed(&self, _: Uuid, _: Uuid, _: &str) -> Result<bool, ResourceAccessError> {
            Ok(self.0)
        }
        async fn grant(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
            _: &str,
        ) -> Result<(), ResourceAccessError> {
            Ok(())
        }
        async fn revoke(
            &self,
            _: Uuid,
            _: Uuid,
            _: Uuid,
            _: &str,
        ) -> Result<(), ResourceAccessError> {
            Ok(())
        }
    }
    #[derive(Clone, Default)]
    struct Reader(Arc<Mutex<usize>>);
    #[async_trait]
    impl ResourceReader for Reader {
        async fn read(&self, resource: &str) -> Result<Option<DemoResource>, ResourceAccessError> {
            *self
                .0
                .lock()
                .map_err(|_| ResourceAccessError::Unavailable)? += 1;
            Ok(Some(DemoResource {
                resource_ref: resource.into(),
                display_label: "Fixture".into(),
                summary: "Simulated".into(),
            }))
        }
    }
    #[derive(Default)]
    struct Audit(Arc<Mutex<Vec<(bool, bool)>>>);
    #[async_trait]
    impl ResourceAccessAudit for Audit {
        async fn record(
            &self,
            _: Uuid,
            _: Uuid,
            _: &str,
            allowed: bool,
            invoked: bool,
        ) -> Result<(), ResourceAccessError> {
            self.0
                .lock()
                .map_err(|_| ResourceAccessError::Unavailable)?
                .push((allowed, invoked));
            Ok(())
        }
    }
    #[tokio::test]
    async fn denied_read_is_audited_before_reader_is_invoked() {
        let reader = Reader::default();
        let audit = Audit::default();
        let result = execute_read(
            &Grants(false),
            &reader,
            &audit,
            Uuid::new_v4(),
            Uuid::new_v4(),
            RESOURCE_REFS[0],
        )
        .await;
        assert_eq!(result, Err(ResourceAccessError::Forbidden));
        assert_eq!(*reader.0.lock().expect("test lock"), 0);
        assert_eq!(*audit.0.lock().expect("test lock"), [(false, false)]);
    }
    #[tokio::test]
    async fn granted_read_calls_reader_once_and_records_no_payload() {
        let reader = Reader::default();
        let audit = Audit::default();
        let result = execute_read(
            &Grants(true),
            &reader,
            &audit,
            Uuid::new_v4(),
            Uuid::new_v4(),
            RESOURCE_REFS[0],
        )
        .await;
        assert_eq!(result.expect("fixture").summary, "Simulated");
        assert_eq!(*reader.0.lock().expect("test lock"), 1);
        assert_eq!(*audit.0.lock().expect("test lock"), [(true, true)]);
    }
}
