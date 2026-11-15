//! Connection revision gate composed inside the existing resource-authorized environment.
//! This wrapper accepts only an injected fixture Environment, never a provider transport.
use async_trait::async_trait;
use kusi_application::{
    adaptive::{Actor, Environment, Event},
    connection_settings::ConnectionKind,
    managed_integrations::FixtureConnectionResolver,
};
use kusi_domain::adaptive::{Batch, Catalog, Error, Query, Resource};
use std::sync::Mutex;

pub struct ManagedFixtureEnvironment<E, R> {
    inner: E,
    resolver: R,
    kind: ConnectionKind,
    cache: Mutex<Option<(i64, String, Catalog)>>,
}
impl<E, R> ManagedFixtureEnvironment<E, R> {
    pub fn new(inner: E, resolver: R, kind: ConnectionKind) -> Self {
        Self {
            inner,
            resolver,
            kind,
            cache: Mutex::new(None),
        }
    }
}
#[async_trait]
impl<E: Environment, R: FixtureConnectionResolver> Environment for ManagedFixtureEnvironment<E, R> {
    async fn discover(&self, actor: &Actor) -> Result<Catalog, Error> {
        let revision = self.resolver.fixture_revision(actor, self.kind).await?;
        let original = self.inner.discover(actor).await?;
        let mut catalog = original.clone();
        catalog.version = kusi_application::hash_api_key(&format!(
            "fixture:{}:{revision}:{}",
            self.kind.name(),
            original.version
        ));
        *self.cache.lock().map_err(|_| Error::Unavailable)? =
            Some((revision, catalog.version.clone(), original));
        Ok(catalog)
    }
    async fn read(
        &self,
        actor: &Actor,
        catalog: &Catalog,
        query: &Query,
        limit: usize,
    ) -> Result<Batch, Error> {
        let revision = self.resolver.fixture_revision(actor, self.kind).await?;
        let original = {
            let lock = self.cache.lock().map_err(|_| Error::Unavailable)?;
            let (saved, version, original) = lock.as_ref().ok_or(Error::Forbidden)?;
            if *saved != revision || *version != catalog.version {
                return Err(Error::Forbidden);
            }
            original.clone()
        };
        self.inner.read(actor, &original, query, limit).await
    }
    async fn watch(&self, _: &Actor, _: &Resource) -> Result<(), Error> {
        Err(Error::Forbidden)
    }
    async fn authorize_event(&self, _: &Actor, _: &Event) -> Result<(), Error> {
        Err(Error::Forbidden)
    }
}
