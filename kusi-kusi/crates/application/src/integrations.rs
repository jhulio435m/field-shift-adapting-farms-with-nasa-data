use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("operation forbidden")]
    Forbidden,
    #[error("invalid arguments or result")]
    InvalidData,
    #[error("connector unavailable")]
    Unavailable,
}

/// Implemented by transport adapters. Destinations must come from trusted configuration.
#[async_trait]
pub trait ConnectorExecutor: Send + Sync {
    async fn execute(
        &self,
        adapter: &str,
        input: &Value,
        timeout_ms: u32,
    ) -> Result<Value, ExecutionError>;
}

#[async_trait]
pub trait AuthorizationService: Send + Sync {
    async fn permits(&self, organization: uuid::Uuid, principal: &str, permission: &str) -> bool;
    /// None is reserved for explicitly unprotected service capabilities.
    /// Implementations must fail closed if policy storage is unavailable.
    async fn authorize_resource(
        &self,
        organization: uuid::Uuid,
        principal: &str,
        capability: &kusi_domain::Capability,
        input: &Value,
    ) -> Result<Option<kusi_domain::external_access::ResourceRead>, ExecutionError>;
}

pub trait SchemaValidator: Send + Sync {
    fn validate(&self, schema: &Value, value: &Value) -> bool;
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn record(
        &self,
        event: uuid::Uuid,
        organization: uuid::Uuid,
        capability: &str,
        version: &str,
        outcome: &str,
    ) -> Result<(), ExecutionError>;
}

pub struct ExecuteCapability<R, A, V, C, D> {
    pub registry: R,
    pub authorization: A,
    pub validator: V,
    pub connector: C,
    pub audit: D,
}

impl<
    R: crate::CapabilityRegistry,
    A: AuthorizationService,
    V: SchemaValidator,
    C: ConnectorExecutor,
    D: AuditRepository,
> ExecuteCapability<R, A, V, C, D>
{
    pub async fn execute(
        &self,
        organization: uuid::Uuid,
        principal: &str,
        identifier: &str,
        version: &str,
        input: &Value,
    ) -> Result<Value, ExecutionError> {
        let event = uuid::Uuid::new_v4();
        self.audit
            .record(event, organization, identifier, version, "started")
            .await?;
        let result = self
            .execute_inner(organization, principal, identifier, version, input)
            .await;
        let outcome = match &result {
            Ok(_) => "succeeded",
            Err(ExecutionError::Forbidden) => "forbidden",
            Err(ExecutionError::InvalidData) => "invalid_data",
            Err(ExecutionError::Unavailable) => "unavailable",
        };
        self.audit
            .record(event, organization, identifier, version, outcome)
            .await?;
        result
    }

    async fn execute_inner(
        &self,
        organization: uuid::Uuid,
        principal: &str,
        identifier: &str,
        version: &str,
        input: &Value,
    ) -> Result<Value, ExecutionError> {
        let capabilities = self
            .registry
            .list(organization)
            .await
            .map_err(|_| ExecutionError::Unavailable)?;
        let capability = capabilities
            .into_iter()
            .find(|c| c.identifier == identifier && c.version == version && c.enabled)
            .ok_or(ExecutionError::Forbidden)?;
        if !self
            .authorization
            .permits(organization, principal, &capability.required_permission)
            .await
        {
            return Err(ExecutionError::Forbidden);
        }
        if !self.validator.validate(&capability.input_schema, input) {
            return Err(ExecutionError::InvalidData);
        }
        let resource = self
            .authorization
            .authorize_resource(organization, principal, &capability, input)
            .await?;
        let output = self
            .connector
            .execute(&capability.adapter, input, capability.timeout_ms)
            .await?;
        if !self.validator.validate(&capability.output_schema, &output)
            || resource.is_some_and(|scope| !scope.accepts(&output))
        {
            return Err(ExecutionError::InvalidData);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApplicationError, CapabilityRegistry};
    use kusi_domain::{Capability, Operation};
    use std::sync::Mutex;

    struct Registry;
    #[async_trait]
    impl CapabilityRegistry for Registry {
        async fn register(&self, _: uuid::Uuid, _: &Capability) -> Result<(), ApplicationError> {
            Ok(())
        }
        async fn list(&self, _: uuid::Uuid) -> Result<Vec<Capability>, ApplicationError> {
            Ok(vec![Capability {
                operation: Operation::Read,
                identifier: "student.summary".into(),
                version: "1".into(),
                description: "Generic read capability".into(),
                adapter: "selected".into(),
                required_permission: "student.read".into(),
                enabled: true,
                timeout_ms: 1000,
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
            }])
        }
    }

    struct Allow;
    #[async_trait]
    impl AuthorizationService for Allow {
        async fn authorize_resource(
            &self,
            _: uuid::Uuid,
            _: &str,
            _: &Capability,
            _: &Value,
        ) -> Result<Option<kusi_domain::external_access::ResourceRead>, ExecutionError> {
            Ok(None)
        }
        async fn permits(&self, _: uuid::Uuid, _: &str, _: &str) -> bool {
            true
        }
    }

    struct Valid;
    impl SchemaValidator for Valid {
        fn validate(&self, _: &Value, _: &Value) -> bool {
            true
        }
    }

    struct Adapter(&'static str, Mutex<Vec<String>>);
    #[async_trait]
    impl ConnectorExecutor for Adapter {
        async fn execute(&self, adapter: &str, _: &Value, _: u32) -> Result<Value, ExecutionError> {
            self.1
                .lock()
                .map_err(|_| ExecutionError::Unavailable)?
                .push(adapter.to_owned());
            Ok(serde_json::json!({ "source": self.0 }))
        }
    }

    struct Audit;
    #[async_trait]
    impl AuditRepository for Audit {
        async fn record(
            &self,
            _: uuid::Uuid,
            _: uuid::Uuid,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), ExecutionError> {
            Ok(())
        }
    }

    struct Deny;
    #[async_trait]
    impl AuthorizationService for Deny {
        async fn authorize_resource(
            &self,
            _: uuid::Uuid,
            _: &str,
            _: &Capability,
            _: &Value,
        ) -> Result<Option<kusi_domain::external_access::ResourceRead>, ExecutionError> {
            Err(ExecutionError::Forbidden)
        }
        async fn permits(&self, _: uuid::Uuid, _: &str, _: &str) -> bool {
            false
        }
    }

    struct Reject;
    impl SchemaValidator for Reject {
        fn validate(&self, _: &Value, _: &Value) -> bool {
            false
        }
    }

    struct UnavailableAudit;
    #[async_trait]
    impl AuditRepository for UnavailableAudit {
        async fn record(
            &self,
            _: uuid::Uuid,
            _: uuid::Uuid,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), ExecutionError> {
            Err(ExecutionError::Unavailable)
        }
    }

    #[tokio::test]
    async fn denied_permission_never_invokes_connector() {
        let executor = ExecuteCapability {
            registry: Registry,
            authorization: Deny,
            validator: Valid,
            connector: Adapter("live-target", Mutex::new(Vec::new())),
            audit: Audit,
        };
        let result = executor
            .execute(
                uuid::Uuid::new_v4(),
                "principal",
                "student.summary",
                "1",
                &serde_json::json!({}),
            )
            .await;
        assert!(matches!(result, Err(ExecutionError::Forbidden)));
        assert!(executor.connector.1.lock().expect("test lock").is_empty());
    }

    #[tokio::test]
    async fn invalid_input_never_invokes_connector() {
        let executor = ExecuteCapability {
            registry: Registry,
            authorization: Allow,
            validator: Reject,
            connector: Adapter("live-target", Mutex::new(Vec::new())),
            audit: Audit,
        };
        let result = executor
            .execute(
                uuid::Uuid::new_v4(),
                "principal",
                "student.summary",
                "1",
                &serde_json::json!({"untrusted":"value"}),
            )
            .await;
        assert!(matches!(result, Err(ExecutionError::InvalidData)));
        assert!(executor.connector.1.lock().expect("test lock").is_empty());
    }

    #[tokio::test]
    async fn unavailable_start_audit_never_invokes_connector() {
        let executor = ExecuteCapability {
            registry: Registry,
            authorization: Allow,
            validator: Valid,
            connector: Adapter("live-target", Mutex::new(Vec::new())),
            audit: UnavailableAudit,
        };
        let result = executor
            .execute(
                uuid::Uuid::new_v4(),
                "principal",
                "student.summary",
                "1",
                &serde_json::json!({}),
            )
            .await;
        assert!(matches!(result, Err(ExecutionError::Unavailable)));
        assert!(executor.connector.1.lock().expect("test lock").is_empty());
    }

    #[tokio::test]
    async fn capability_use_case_accepts_independent_connector_adapters() {
        let first = Adapter("system-a", Mutex::new(Vec::new()));
        let second = Adapter("system-b", Mutex::new(Vec::new()));
        let execute_a = ExecuteCapability {
            registry: Registry,
            authorization: Allow,
            validator: Valid,
            connector: first,
            audit: Audit,
        };
        let execute_b = ExecuteCapability {
            registry: Registry,
            authorization: Allow,
            validator: Valid,
            connector: second,
            audit: Audit,
        };
        let organization = uuid::Uuid::new_v4();
        let result_a = execute_a
            .execute(
                organization,
                "principal",
                "student.summary",
                "1",
                &serde_json::json!({}),
            )
            .await;
        let result_b = execute_b
            .execute(
                organization,
                "principal",
                "student.summary",
                "1",
                &serde_json::json!({}),
            )
            .await;
        assert_eq!(
            result_a.ok(),
            Some(serde_json::json!({ "source": "system-a" }))
        );
        assert_eq!(
            result_b.ok(),
            Some(serde_json::json!({ "source": "system-b" }))
        );
    }
}
