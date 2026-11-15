//! Driven adapter for private durable state (`State` port).
use super::{directory, exclusive, private_read, write_json};
use async_trait::async_trait;
use kusi_application::{
    adaptive::{Actor, AuditEntry, Receipt, State, Subscription},
    hash_api_key,
};
use kusi_domain::adaptive::{Context, Error};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub struct FileState {
    pub root: PathBuf,
}
impl FileState {
    fn actor_dir(&self, actor: &Actor) -> Result<PathBuf, Error> {
        let path = self.root.join(hash_api_key(&format!(
            "{}:{}",
            actor.organization, actor.principal
        )));
        directory(&path)?;
        Ok(path)
    }
    fn path(&self, a: &Actor, kind: &str, id: &str) -> Result<PathBuf, Error> {
        Ok(self
            .actor_dir(a)?
            .join(format!("{kind}-{}.json", hash_api_key(id))))
    }
    fn envelope<T: DeserializeOwned>(&self, path: &Path) -> Result<T, Error> {
        let value: Value = private_read(path)?;
        if value["format"] != 1 {
            return Err(Error::Invalid);
        }
        serde_json::from_value(value["data"].clone()).map_err(|_| Error::Invalid)
    }
}
#[async_trait]
impl State for FileState {
    async fn context(&self, a: &Actor) -> Result<Context, Error> {
        let p = self.path(a, "context", "current")?;
        if !p.exists() {
            return Ok(Context::default());
        }
        if std::fs::metadata(&p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|v| v.elapsed().ok())
            .is_some_and(|d| d.as_secs() > 86400)
        {
            return Ok(Context::default());
        }
        self.envelope(&p)
    }
    async fn save_context(&self, a: &Actor, c: &Context) -> Result<(), Error> {
        write_json(
            &self.path(a, "context", "current")?,
            &json!({"format":1,"data":c}),
        )
    }
    async fn claim(&self, a: &Actor, id: &str) -> Result<bool, Error> {
        exclusive(&self.path(a, "claim", id)?)
    }
    async fn receipt(&self, a: &Actor, id: &str) -> Result<Option<Receipt>, Error> {
        let p = self.path(a, "receipt", id)?;
        if !p.exists() {
            return Ok(None);
        }
        Ok(Some(self.envelope(&p)?))
    }
    async fn finish(&self, a: &Actor, id: &str, r: &Receipt) -> Result<(), Error> {
        write_json(&self.path(a, "receipt", id)?, &json!({"format":1,"data":r}))
    }
    async fn subscriptions(&self, a: &Actor) -> Result<Vec<Subscription>, Error> {
        let mut subscriptions = Vec::new();
        for entry in std::fs::read_dir(self.actor_dir(a)?).map_err(|_| Error::Unavailable)? {
            let path = entry.map_err(|_| Error::Unavailable)?.path();
            if path
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|n| n.starts_with("subscription-"))
            {
                subscriptions.push(self.envelope(&path)?);
            }
        }
        Ok(subscriptions)
    }
    async fn save_subscription(&self, a: &Actor, s: &Subscription) -> Result<(), Error> {
        write_json(
            &self.path(a, "subscription", &s.id)?,
            &json!({"format":1,"data":s}),
        )
    }
    async fn claim_event(&self, a: &Actor, s: &str, e: &str) -> Result<bool, Error> {
        exclusive(&self.path(a, "delivery", &format!("{s}:{e}"))?)
    }
    async fn record_audit(&self, a: &Actor, entry: &AuditEntry) -> Result<(), Error> {
        write_json(
            &self.path(a, "audit", &format!("{}:{}", entry.kind, entry.id))?,
            &json!({"format":1,"data":entry}),
        )
    }
    async fn audits(&self, a: &Actor) -> Result<Vec<AuditEntry>, Error> {
        let mut items = Vec::new();
        for entry in std::fs::read_dir(self.actor_dir(a)?).map_err(|_| Error::Unavailable)? {
            let path = entry.map_err(|_| Error::Unavailable)?.path();
            if path
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|n| n.starts_with("audit-"))
            {
                items.push(self.envelope(&path)?);
            }
        }
        items.sort_by(|left: &AuditEntry, right: &AuditEntry| {
            (&left.kind, &left.id).cmp(&(&right.kind, &right.id))
        });
        Ok(items)
    }
}
