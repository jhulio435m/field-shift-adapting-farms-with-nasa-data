//! Driven adapter for registered environment sources (`Environment` port).
use super::{SourceConfig, private_read};
use async_trait::async_trait;
use kusi_application::{
    adaptive::{Actor, Environment, Event},
    hash_api_key,
};
use kusi_domain::adaptive::{Batch, Catalog, Error, Query, Resource};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, process::Stdio};
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    pub catalog: Catalog,
    pub rows: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
    #[serde(default = "yes")]
    pub enabled: bool,
}
fn yes() -> bool {
    true
}

pub struct Sources {
    pub actor: Actor,
    pub sources: Vec<SourceConfig>,
    pub bridge: PathBuf,
    cache: std::sync::Mutex<Option<(Catalog, BTreeMap<String, Catalog>)>>,
}
impl Sources {
    pub fn new(actor: Actor, sources: Vec<SourceConfig>, bridge: PathBuf) -> Self {
        Self {
            actor,
            sources,
            bridge,
            cache: std::sync::Mutex::new(None),
        }
    }
    pub fn with_managed_fixture<
        R: kusi_application::managed_integrations::FixtureConnectionResolver,
    >(
        self,
        resolver: R,
        kind: kusi_application::connection_settings::ConnectionKind,
    ) -> Result<super::ManagedFixtureEnvironment<Self, R>, Error> {
        if self.sources.iter().any(|source| source.kind != "manifest") {
            return Err(Error::Forbidden);
        }
        Ok(super::ManagedFixtureEnvironment::new(self, resolver, kind))
    }
    fn authorized(&self, actor: &Actor) -> Result<(), Error> {
        if actor.organization != self.actor.organization
            || actor.principal != self.actor.principal
            || !actor.read
            || !self.actor.read
        {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
    async fn rpc(&self, source: &SourceConfig, request: Value) -> Result<Value, Error> {
        let mut child = tokio::process::Command::new("python3")
            .arg(&self.bridge)
            .arg(&source.config_file)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Error::Unavailable)?;
        let bytes = serde_json::to_vec(&request).map_err(|_| Error::Invalid)?;
        child
            .stdin
            .take()
            .ok_or(Error::Unavailable)?
            .write_all(&bytes)
            .await
            .map_err(|_| Error::Unavailable)?;
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(35), child.wait_with_output())
                .await
                .map_err(|_| Error::Unavailable)?
                .map_err(|_| Error::Unavailable)?;
        if !output.status.success() {
            return Err(Error::Unavailable);
        }
        if output.stdout.len() > 2 * 1024 * 1024 {
            return Err(Error::Incomplete);
        }
        let result: Value = serde_json::from_slice(&output.stdout).map_err(|_| Error::Invalid)?;
        match result.get("error").and_then(Value::as_str) {
            Some("forbidden") => Err(Error::Forbidden),
            Some("stale") => Err(Error::Invalid),
            Some(_) => Err(Error::Unavailable),
            None => Ok(result),
        }
    }
    async fn local_catalog(&self, source: &SourceConfig) -> Result<Catalog, Error> {
        if source.kind == "manifest" {
            let manifest: Manifest = private_read(&source.config_file)?;
            if !manifest.enabled {
                return Err(Error::Forbidden);
            }
            let mut catalog = manifest.catalog;
            catalog.version = hash_api_key(
                &serde_json::to_string(&catalog.resources).map_err(|_| Error::Invalid)?,
            );
            Ok(catalog)
        } else {
            serde_json::from_value(self.rpc(source, json!({"op":"discover"})).await?)
                .map_err(|_| Error::Invalid)
        }
    }
    fn resolve<'a>(&'a self, id: &str) -> Result<(&'a SourceConfig, String), Error> {
        let (source, resource) = id.split_once("::").ok_or(Error::Invalid)?;
        Ok((
            self.sources
                .iter()
                .find(|s| s.id == source)
                .ok_or(Error::Forbidden)?,
            resource.into(),
        ))
    }
}
#[async_trait]
impl Environment for Sources {
    async fn discover(&self, actor: &Actor) -> Result<Catalog, Error> {
        self.authorized(actor)?;
        let mut resources = Vec::new();
        let mut versions = Vec::new();
        let mut locals = BTreeMap::new();
        for source in &self.sources {
            let catalog = self.local_catalog(source).await?;
            locals.insert(source.id.clone(), catalog.clone());
            versions.push((source.id.clone(), catalog.version));
            for mut resource in catalog.resources {
                resource.id = format!("{}::{}", source.id, resource.id);
                resource.label = format!("{} / {}", source.label, resource.label);
                resources.push(resource);
            }
        }
        if resources.len() > 80 {
            return Err(Error::Incomplete);
        }
        let version = hash_api_key(
            &serde_json::to_string(&(versions, &resources)).map_err(|_| Error::Invalid)?,
        );
        let catalog = Catalog {
            source: self
                .sources
                .iter()
                .map(|s| s.label.as_str())
                .collect::<Vec<_>>()
                .join("; "),
            version,
            resources,
        };
        *self.cache.lock().map_err(|_| Error::Unavailable)? = Some((catalog.clone(), locals));
        Ok(catalog)
    }
    async fn read(
        &self,
        actor: &Actor,
        catalog: &Catalog,
        query: &Query,
        limit: usize,
    ) -> Result<Batch, Error> {
        self.authorized(actor)?;
        if limit == 0 {
            return Err(Error::Incomplete);
        }
        let (source, id) = self.resolve(&query.resource)?;
        let local = {
            let cache = self.cache.lock().map_err(|_| Error::Unavailable)?;
            let (current, locals) = cache.as_ref().ok_or(Error::Invalid)?;
            if current.version != catalog.version {
                return Err(Error::Invalid);
            }
            locals.get(&source.id).cloned().ok_or(Error::Invalid)?
        };
        let resource = local.resource(&id)?;
        let fields = query.fields(resource)?;
        if source.kind == "manifest" {
            let manifest: Manifest = private_read(&source.config_file)?;
            if !manifest.enabled {
                return Err(Error::Forbidden);
            }
            if hash_api_key(
                &serde_json::to_string(&manifest.catalog.resources).map_err(|_| Error::Invalid)?,
            ) != local.version
            {
                return Err(Error::Invalid);
            }
            let rows = manifest.rows.get(&id).ok_or(Error::Forbidden)?;
            let mut selected = Vec::new();
            for row in rows {
                if kusi_domain::adaptive::matches_typed(resource, &query.filters, row)? {
                    selected.push(
                        fields
                            .iter()
                            .map(|f| Ok((f.clone(), row.get(f).ok_or(Error::Invalid)?.clone())))
                            .collect::<Result<_, Error>>()?,
                    );
                    if selected.len() > limit {
                        break;
                    }
                }
            }
            let truncated = selected.len() > limit;
            if truncated {
                selected.truncate(limit)
            }
            Ok(Batch {
                rows: selected,
                truncated,
            })
        } else {
            serde_json::from_value(
                self.rpc(
                    source,
                    json!({
                        "op": "read",
                        "version": local.version,
                        "resource": id,
                        "fields": fields,
                        "filters": query.filters,
                        "limit": limit
                    }),
                )
                .await?,
            )
            .map_err(|_| Error::Invalid)
        }
    }
    async fn authorize_event(&self, actor: &Actor, event: &Event) -> Result<(), Error> {
        self.authorized(actor)?;
        let (source, id) = self.resolve(&event.resource)?;
        if source.kind == "manifest" {
            let manifest: Manifest = private_read(&source.config_file)?;
            if !manifest.enabled || !manifest.rows.contains_key(&id) {
                return Err(Error::Forbidden);
            }
            return Ok(());
        }
        let catalog = {
            let cache = self.cache.lock().map_err(|_| Error::Unavailable)?;
            let (_, locals) = cache.as_ref().ok_or(Error::Invalid)?;
            locals.get(&source.id).cloned().ok_or(Error::Invalid)?
        };
        let result = self
            .rpc(
                source,
                json!({
                    "op": "authorize_event",
                    "resource": id,
                    "version": catalog.version,
                    "event_id": event.event_id,
                    "test": event.test
                }),
            )
            .await?;
        if result["authorized"] != true {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
    async fn watch(&self, actor: &Actor, resource: &Resource) -> Result<(), Error> {
        self.authorized(actor)?;
        if !actor.subscribe {
            return Err(Error::Forbidden);
        }
        let (source, id) = self.resolve(&resource.id)?;
        let catalog = self.local_catalog(source).await?;
        let current = catalog.resource(&id)?;
        if current.events.is_empty() {
            return Err(Error::Forbidden);
        }
        if source.kind != "manifest" {
            self.rpc(
                source,
                json!({
                    "op": "watch",
                    "resource": id,
                    "version": catalog.version,
                    "namespace": hash_api_key(&format!("{}:{}", actor.organization, actor.principal))
                }),
            )
            .await?;
        }
        Ok(())
    }
}
