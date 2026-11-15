//! Local, typed fixture reads composed with the existing 007 identity/resource authority.
use crate::{PgOrganizationRepository, local::LocalStore};
use kusi_application::{
    CapabilityRegistry,
    adaptive::Actor,
    connector_manifest::validate_operation,
    evaluation::{ReadGate, execute_fixture_read},
    integrations::AuthorizationService,
    local::LocalError,
    managed_integrations::{FixtureConnectionResolver, ManagedQueryInput},
};
use serde_json::{Value, json};
struct Gate<'a> {
    store: &'a LocalStore,
    actor: &'a Actor,
    request: &'a ManagedQueryInput,
}
#[async_trait::async_trait]
impl ReadGate for Gate<'_> {
    async fn authorize(&self, resource: &str) -> Result<(), LocalError> {
        let repo = PgOrganizationRepository::new(self.store.pool.clone());
        let cap = repo
            .list(self.actor.organization)
            .await
            .map_err(|_| LocalError::Unavailable)?
            .into_iter()
            .find(|cap| {
                cap.enabled
                    && cap.identifier == self.request.operation
                    && cap.version == self.request.version
                    && cap.adapter == self.request.kind.name()
            })
            .ok_or(LocalError::Forbidden)?;
        if !repo
            .permits(
                self.actor.organization,
                &self.actor.principal,
                &cap.required_permission,
            )
            .await
        {
            return Err(LocalError::Forbidden);
        }
        let scope = repo
            .authorize_resource(
                self.actor.organization,
                &self.actor.principal,
                &cap,
                &json!({"resource_ref":resource}),
            )
            .await
            .map_err(|_| LocalError::Forbidden)?;
        if scope.is_none() {
            return Err(LocalError::Forbidden);
        } // Managed reads always require a verified external link and an exact grant.
        Ok(())
    }
    async fn revision(&self) -> Result<i64, LocalError> {
        self.store
            .fixture_revision(self.actor, self.request.kind)
            .await
            .map_err(|_| LocalError::Forbidden)
    }
    async fn read(&self, resource: &str) -> Result<Value, LocalError> {
        self.authorize(resource).await?;
        if self.revision().await? != self.request.expected_revision {
            return Err(LocalError::Conflict);
        }
        if resource != "fixture-a" {
            return Err(LocalError::NotFound);
        }
        Ok(
            json!({"resource_ref":resource,"evidence":[format!("synthetic-{}-record-010",self.request.kind.name())]}),
        )
    }
}
impl LocalStore {
    pub async fn execute_managed_fixture(
        &self,
        actor: &Actor,
        input: &ManagedQueryInput,
    ) -> Result<Value, LocalError> {
        if !actor.read {
            return Err(LocalError::Forbidden);
        }
        validate_operation(input.kind, &input.version, &input.operation, &input.input)?;
        let gate = Gate {
            store: self,
            actor,
            request: input,
        };
        let result = execute_fixture_read(
            &gate,
            input.kind,
            &input.version,
            &input.operation,
            &input.input,
            input.expected_revision,
        )
        .await?;
        // Recheck after data access before publishing evidence.
        gate.authorize(
            input
                .input
                .get("resource_ref")
                .and_then(Value::as_str)
                .ok_or(LocalError::Invalid)?,
        )
        .await?;
        Ok(json!({"mode":"fixture","provider_active":false,"result":result}))
    }
}
