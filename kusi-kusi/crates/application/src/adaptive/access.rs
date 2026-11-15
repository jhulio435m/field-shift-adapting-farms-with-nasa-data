//! Authorization of whole analytical resources, independent of channel/provider.
use super::{Actor, Environment, Event};
use crate::{
    CapabilityRegistry, hash_api_key,
    integrations::{AuditRepository, AuthorizationService, ExecutionError},
};
use async_trait::async_trait;
use kusi_domain::adaptive::{Batch, Catalog, Error, Query, Resource};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBinding {
    pub adapter: String,
    pub capability: String,
    pub version: String,
    pub resource_ref: String,
}

impl ResourceBinding {
    pub fn is_valid(&self) -> bool {
        !self.adapter.is_empty()
            && self.adapter.len() <= 120
            && !self.capability.is_empty()
            && self.capability.len() <= 120
            && !self.version.is_empty()
            && self.version.len() <= 32
            && kusi_domain::external_access::valid_reference(&self.resource_ref)
    }
}

pub struct AuthorizedEnvironment<E, R> {
    inner: E,
    repository: R,
    actor: Actor,
    bindings: BTreeMap<String, ResourceBinding>,
    catalog: Mutex<Option<(String, Catalog)>>,
}

fn error(error: ExecutionError) -> Error {
    match error {
        ExecutionError::Forbidden => Error::Forbidden,
        ExecutionError::InvalidData => Error::Invalid,
        ExecutionError::Unavailable => Error::Unavailable,
    }
}

impl<E, R> AuthorizedEnvironment<E, R>
where
    E: Environment,
    R: CapabilityRegistry + AuthorizationService + AuditRepository,
{
    pub fn new(
        inner: E,
        repository: R,
        actor: Actor,
        bindings: BTreeMap<String, ResourceBinding>,
    ) -> Result<Self, Error> {
        if bindings.is_empty()
            || bindings.len() > 80
            || bindings
                .iter()
                .any(|(id, binding)| id.is_empty() || id.len() > 255 || !binding.is_valid())
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            inner,
            repository,
            actor,
            bindings,
            catalog: Mutex::new(None),
        })
    }

    fn actor(&self, actor: &Actor) -> Result<(), Error> {
        if !actor.read
            || !self.actor.read
            || actor.organization != self.actor.organization
            || actor.principal != self.actor.principal
        {
            return Err(Error::Forbidden);
        }
        Ok(())
    }

    async fn authorize(&self, actor: &Actor, resource: &str) -> Result<(), Error> {
        self.actor(actor)?;
        let binding = self.bindings.get(resource).ok_or(Error::Forbidden)?;
        let event = Uuid::new_v4();
        self.repository
            .record(
                event,
                actor.organization,
                "adaptive.resource.authorize",
                "1",
                "started",
            )
            .await
            .map_err(error)?;
        let decision = async {
            let capabilities = self
                .repository
                .list(actor.organization)
                .await
                .map_err(|_| Error::Unavailable)?;
            let capability = capabilities
                .into_iter()
                .find(|cap| {
                    cap.enabled
                        && cap.adapter == binding.adapter
                        && cap.identifier == binding.capability
                        && cap.version == binding.version
                })
                .ok_or(Error::Forbidden)?;
            if !self
                .repository
                .permits(
                    actor.organization,
                    &actor.principal,
                    &capability.required_permission,
                )
                .await
            {
                return Err(Error::Forbidden);
            }
            let scope = self
                .repository
                .authorize_resource(
                    actor.organization,
                    &actor.principal,
                    &capability,
                    &serde_json::json!({"resource_ref":binding.resource_ref}),
                )
                .await
                .map_err(error)?;
            // Unlike service execution, an absent resource policy is not authority.
            if scope.is_none_or(|scope| scope.resource_ref != binding.resource_ref) {
                return Err(Error::Forbidden);
            }
            Ok(())
        }
        .await;
        let outcome = match &decision {
            Ok(()) => "succeeded",
            Err(Error::Forbidden) => "forbidden",
            Err(Error::Invalid) => "invalid_data",
            _ => "unavailable",
        };
        self.repository
            .record(
                event,
                actor.organization,
                "adaptive.resource.authorize",
                "1",
                outcome,
            )
            .await
            .map_err(error)?;
        decision
    }
}

#[async_trait]
impl<E, R> Environment for AuthorizedEnvironment<E, R>
where
    E: Environment,
    R: CapabilityRegistry + AuthorizationService + AuditRepository,
{
    async fn discover(&self, actor: &Actor) -> Result<Catalog, Error> {
        self.actor(actor)?;
        let mut allowed = Vec::new();
        for id in self.bindings.keys() {
            match self.authorize(actor, id).await {
                Ok(()) => allowed.push(id.as_str()),
                Err(Error::Forbidden) => {}
                Err(error) => return Err(error),
            }
        }
        if allowed.is_empty() {
            return Err(Error::Forbidden);
        }
        let full = self.inner.discover(actor).await?;
        let resources: Vec<_> = full
            .resources
            .iter()
            .filter(|resource| allowed.contains(&resource.id.as_str()))
            .cloned()
            .collect();
        if resources.is_empty() {
            return Err(Error::Forbidden);
        }
        let version = hash_api_key(
            &serde_json::to_string(&(
                &full.version,
                &resources,
                &self.bindings,
                actor.organization,
                &actor.principal,
            ))
            .map_err(|_| Error::Invalid)?,
        );
        *self.catalog.lock().map_err(|_| Error::Unavailable)? = Some((version.clone(), full));
        Ok(Catalog {
            source: "Entorno autorizado".into(),
            version,
            resources,
        })
    }

    async fn read(
        &self,
        actor: &Actor,
        catalog: &Catalog,
        query: &Query,
        limit: usize,
    ) -> Result<Batch, Error> {
        self.authorize(actor, &query.resource).await?;
        catalog.resource(&query.resource)?;
        let full = {
            let cache = self.catalog.lock().map_err(|_| Error::Unavailable)?;
            let (version, full) = cache.as_ref().ok_or(Error::Invalid)?;
            if version != &catalog.version {
                return Err(Error::Invalid);
            }
            full.clone()
        };
        self.inner.read(actor, &full, query, limit).await
    }

    async fn watch(&self, actor: &Actor, resource: &Resource) -> Result<(), Error> {
        if !actor.subscribe || !self.actor.subscribe {
            return Err(Error::Forbidden);
        }
        self.authorize(actor, &resource.id).await?;
        self.inner.watch(actor, resource).await
    }

    async fn authorize_event(&self, actor: &Actor, event: &Event) -> Result<(), Error> {
        if !actor.subscribe || !self.actor.subscribe {
            return Err(Error::Forbidden);
        }
        self.authorize(actor, &event.resource).await?;
        self.inner.authorize_event(actor, event).await
    }
}
