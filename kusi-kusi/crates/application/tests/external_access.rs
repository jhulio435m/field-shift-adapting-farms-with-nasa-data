use async_trait::async_trait;
use kusi_application::{
    ApplicationError, CapabilityRegistry, external_access::ProvisionRequest, integrations::*,
};
use kusi_domain::{Capability, Operation, external_access::ResourceRead};
use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

struct Registry;
#[async_trait]
impl CapabilityRegistry for Registry {
    async fn register(&self, _: Uuid, _: &Capability) -> Result<(), ApplicationError> {
        Ok(())
    }
    async fn list(&self, _: Uuid) -> Result<Vec<Capability>, ApplicationError> {
        Ok(vec![capability()])
    }
}
fn capability() -> Capability {
    Capability {
        operation: Operation::Read,
        identifier: "record.read".into(),
        version: "1".into(),
        description: "Generic record".into(),
        adapter: "system-a".into(),
        required_permission: "record.read".into(),
        enabled: true,
        timeout_ms: 1000,
        input_schema: json!({}),
        output_schema: json!({}),
    }
}
// Synthetic port: represents a revocable, exact external ACL response.
struct Access {
    org: Uuid,
    active: Mutex<bool>,
    unavailable: bool,
}
#[async_trait]
impl AuthorizationService for Access {
    async fn permits(&self, _: Uuid, _: &str, _: &str) -> bool {
        true
    }
    async fn authorize_resource(
        &self,
        org: Uuid,
        principal: &str,
        cap: &Capability,
        input: &Value,
    ) -> Result<Option<ResourceRead>, ExecutionError> {
        if self.unavailable {
            return Err(ExecutionError::Unavailable);
        }
        let read: ResourceRead =
            serde_json::from_value(input.clone()).map_err(|_| ExecutionError::InvalidData)?;
        if !read.is_valid() {
            return Err(ExecutionError::InvalidData);
        }
        if org != self.org
            || principal != "verified-principal"
            || cap.adapter != "system-a"
            || cap.identifier != "record.read"
            || cap.version != "1"
            || read.resource_ref != "record-a"
            || !*self
                .active
                .lock()
                .map_err(|_| ExecutionError::Unavailable)?
        {
            return Err(ExecutionError::Forbidden);
        }
        Ok(Some(read))
    }
}
struct Validator;
impl SchemaValidator for Validator {
    fn validate(&self, _: &Value, _: &Value) -> bool {
        true
    }
}
struct Connector {
    count: AtomicUsize,
    wrong_resource: bool,
}
#[async_trait]
impl ConnectorExecutor for Connector {
    async fn execute(&self, _: &str, _: &Value, _: u32) -> Result<Value, ExecutionError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(
            json!({"resource_ref":if self.wrong_resource {"record-b"} else {"record-a"},"data":{"synthetic":true}}),
        )
    }
}
#[derive(Default)]
struct Audit(Mutex<Vec<String>>);
#[async_trait]
impl AuditRepository for Audit {
    async fn record(
        &self,
        _: Uuid,
        _: Uuid,
        _: &str,
        _: &str,
        outcome: &str,
    ) -> Result<(), ExecutionError> {
        self.0
            .lock()
            .map_err(|_| ExecutionError::Unavailable)?
            .push(outcome.into());
        Ok(())
    }
}
type Executor = ExecuteCapability<Registry, Access, Validator, Connector, Audit>;
fn executor(org: Uuid, unavailable: bool, wrong_resource: bool) -> Executor {
    ExecuteCapability {
        registry: Registry,
        authorization: Access {
            org,
            active: Mutex::new(true),
            unavailable,
        },
        validator: Validator,
        connector: Connector {
            count: AtomicUsize::new(0),
            wrong_resource,
        },
        audit: Audit::default(),
    }
}

#[tokio::test]
async fn exact_acl_denials_never_reach_transport() {
    let org = Uuid::new_v4();
    let e = executor(org, false, false);
    for (tenant, principal, input) in [
        (
            Uuid::new_v4(),
            "verified-principal",
            json!({"resource_ref":"record-a"}),
        ),
        (
            org,
            "caller-chosen-subject",
            json!({"resource_ref":"record-a"}),
        ),
        (
            org,
            "verified-principal",
            json!({"resource_ref":"record-b"}),
        ),
        (
            org,
            "verified-principal",
            json!({"resource_ref":"record-a","tenant":"fake"}),
        ),
        (
            org,
            "verified-principal",
            json!({"resource_ref":"record-a","filters":[]}),
        ),
        (org, "verified-principal", json!({"resource_ref":" "})),
    ] {
        assert!(
            e.execute(tenant, principal, "record.read", "1", &input)
                .await
                .is_err()
        );
    }
    assert!(
        e.execute(
            org,
            "verified-principal",
            "record.read",
            "2",
            &json!({"resource_ref":"record-a"})
        )
        .await
        .is_err()
    );
    assert_eq!(e.connector.count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn revocation_applies_to_the_next_read_and_audit_contains_only_outcomes() {
    let org = Uuid::new_v4();
    let e = executor(org, false, false);
    let input = json!({"resource_ref":"record-a"});
    assert!(
        e.execute(org, "verified-principal", "record.read", "1", &input)
            .await
            .is_ok()
    );
    *e.authorization.active.lock().expect("test lock") = false;
    assert!(matches!(
        e.execute(org, "verified-principal", "record.read", "1", &input)
            .await,
        Err(ExecutionError::Forbidden)
    ));
    assert_eq!(e.connector.count.load(Ordering::SeqCst), 1);
    assert_eq!(
        *e.audit.0.lock().expect("test lock"),
        ["started", "succeeded", "started", "forbidden"]
    );
}

#[tokio::test]
async fn unavailable_policy_storage_fails_closed_and_wrong_output_is_rejected() {
    let org = Uuid::new_v4();
    let e = executor(org, true, false);
    let input = json!({"resource_ref":"record-a"});
    assert!(matches!(
        e.execute(org, "verified-principal", "record.read", "1", &input)
            .await,
        Err(ExecutionError::Unavailable)
    ));
    assert_eq!(e.connector.count.load(Ordering::SeqCst), 0);
    let e = executor(org, false, true);
    assert!(matches!(
        e.execute(org, "verified-principal", "record.read", "1", &input)
            .await,
        Err(ExecutionError::InvalidData)
    ));
}

#[test]
fn operator_must_confirm_identity_and_grant_and_bound_lifetime() {
    for action in [
        json!({"operation":"link","subject":"external-subject","ttl_seconds":60,"confirmed":false}),
        json!({"operation":"link","subject":"external-subject","ttl_seconds":0,"confirmed":true}),
        json!({"operation":"grant","capability":"record.read","version":"1","resource_ref":"record-a","ttl_seconds":2_592_001,"confirmed":true}),
    ] {
        let request: ProvisionRequest = serde_json::from_value(json!({"organization":Uuid::new_v4(),"principal_hash":"a".repeat(64),"adapter":"system-a","action":action})).expect("fixture");
        assert!(request.validate().is_err());
    }
}
