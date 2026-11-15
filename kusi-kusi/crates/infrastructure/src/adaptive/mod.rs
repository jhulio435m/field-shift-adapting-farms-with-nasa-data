//! Registered adapters and private durable state; no business/resource names in dispatch.
mod managed;
pub use managed::ManagedFixtureEnvironment;
mod planner;
mod sources;
mod state;

pub use planner::LanguagePlanner;
pub use sources::Sources;
pub use state::FileState;

use kusi_application::adaptive::Actor;
use kusi_domain::adaptive::Error;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_secret_file: Option<PathBuf>,
    pub id: String,
    pub label: String,
    pub kind: String,
    pub config_file: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessConfig {
    pub database_url_env: String,
    pub resources: std::collections::BTreeMap<String, kusi_application::adaptive::ResourceBinding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedFixtureConfig {
    pub kind: kusi_application::connection_settings::ConnectionKind,
    pub storage_directory: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub managed_fixture: Option<ManagedFixtureConfig>,
    #[serde(default)]
    pub access: Option<AccessConfig>,
    pub organization: Uuid,
    pub principal: String,
    pub read: bool,
    pub subscribe: bool,
    pub sources: Vec<SourceConfig>,
    pub state_directory: PathBuf,
    pub bridge_script: PathBuf,
    pub llm: crate::llm::Profile,
    pub binding_hash: String,
}
impl RuntimeConfig {
    /// A separate local composition; never relaxes the generic HTTP/paid-provider block.
    pub fn validate_loopback(&self) -> Result<(), Error> {
        let local_model = self
            .llm
            .endpoint
            .as_deref()
            .and_then(|s| url::Url::parse(s).ok())
            .is_some_and(|u| {
                u.scheme() == "http"
                    && u.host_str() == Some("127.0.0.1")
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.query().is_none()
                    && u.fragment().is_none()
            });
        if !local_model
            || self.llm.api_key_env.is_some()
            || !self.read
            || self.subscribe
            || !matches!(
                self.llm.provider,
                crate::llm::Provider::OpenaiCompatible | crate::llm::Provider::Ollama
            )
            || self
                .sources
                .iter()
                .any(|s| s.kind != "graphql_loopback" || s.event_secret_file.is_some())
            || self.access.is_some()
            || self.managed_fixture.is_some()
        {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
    pub fn actor(&self) -> Actor {
        Actor {
            organization: self.organization,
            principal: self.principal.clone(),
            read: self.read,
            subscribe: self.subscribe,
        }
    }
    pub fn load(path: &Path) -> Result<Self, Error> {
        let value: Self = private_read(path)?;
        if value.principal.is_empty()
            || value.principal.len() > 120
            || value.sources.is_empty()
            || value.sources.len() > 8
            || !value.state_directory.is_absolute()
            || !value.bridge_script.is_absolute()
        {
            return Err(Error::Invalid);
        }
        if let Some(fixture) = &value.managed_fixture {
            let local_planner = value
                .llm
                .endpoint
                .as_deref()
                .and_then(|endpoint| url::Url::parse(endpoint).ok())
                .is_some_and(|url| {
                    url.scheme() == "http"
                        && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.query().is_none()
                        && url.fragment().is_none()
                });
            if value.access.is_none()
                || !fixture.storage_directory.is_absolute()
                || !local_planner
                || value.llm.api_key_env.is_some()
                || !matches!(value.llm.provider, crate::llm::Provider::OpenaiCompatible)
                || value.sources.iter().any(|source| source.kind != "manifest")
            {
                return Err(Error::Forbidden);
            }
        }
        if let Some(access) = &value.access
            && (value.principal.len() != 64
                || !value
                    .principal
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || access.database_url_env.is_empty()
                || access.database_url_env.len() > 120
                || !access
                    .database_url_env
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || access.resources.is_empty()
                || access.resources.len() > 80
                || access
                    .resources
                    .iter()
                    .any(|(id, binding)| !id.contains("::") || !binding.is_valid()))
        {
            return Err(Error::Invalid);
        }
        let mut ids = std::collections::BTreeSet::new();
        for source in &value.sources {
            if source.id.is_empty()
                || source.id.len() > 80
                || !source
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !source.config_file.is_absolute()
                || source
                    .event_secret_file
                    .as_ref()
                    .is_some_and(|path| !path.is_absolute())
                || !ids.insert(&source.id)
                || !matches!(
                    source.kind.as_str(),
                    "manifest" | "frappe_ssh" | "graphql_loopback"
                )
            {
                return Err(Error::Invalid);
            }
        }
        Ok(value)
    }
}
pub fn private_read<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Error::Unavailable)?;
    if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
        return Err(Error::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::Forbidden);
        }
    }
    read_json(path)
}
fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| Error::Unavailable)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(Error::Incomplete);
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)
}
pub(crate) fn directory(path: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(path).map_err(|_| Error::Unavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| Error::Unavailable)?
    }
    Ok(())
}
pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Error> {
    let parent = path.parent().ok_or(Error::Invalid)?;
    directory(parent)?;
    let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|_| Error::Unavailable)?;
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Invalid)?;
    file.write_all(&bytes).map_err(|_| Error::Unavailable)?;
    file.sync_all().map_err(|_| Error::Unavailable)?;
    std::fs::rename(&temporary, path).map_err(|_| Error::Unavailable)?;
    std::fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unavailable)?;
    Ok(())
}
pub(crate) fn exclusive(path: &Path) -> Result<bool, Error> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(b"{\"format\":1,\"status\":\"claimed\"}")
                .and_then(|_| file.sync_all())
                .map_err(|_| Error::Unavailable)?;
            std::fs::File::open(path.parent().ok_or(Error::Invalid)?)
                .and_then(|f| f.sync_all())
                .map_err(|_| Error::Unavailable)?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(_) => Err(Error::Unavailable),
    }
}

#[cfg(test)]
mod loopback_composition_tests {
    use super::*;
    #[test]
    fn local_adaptation_refuses_external_models_credentials_and_other_protocols() {
        let value = serde_json::json!({
            "organization": "11111111-1111-4111-8111-111111111111", "principal": "seller", "read": true, "subscribe": false,
            "sources": [{"id":"source","label":"Source","kind":"graphql_loopback","config_file":"/tmp/source.json"}],
            "state_directory":"/tmp/state","bridge_script":"/tmp/bridge.py","binding_hash":"binding",
            "llm":{"provider":"openai_compatible","model":"local","endpoint":"http://127.0.0.1:11434/v1/chat/completions","api_key_env":null,"max_output_tokens":1000,"timeout_ms":20000,"tools_supported":false,"anthropic_version":null,"routing_providers":null}
        });
        let config: RuntimeConfig = serde_json::from_value(value.clone()).expect("fixture");
        assert!(config.validate_loopback().is_ok());
        for (path, replacement) in [
            (
                "/llm/endpoint",
                serde_json::json!("https://external.example/v1/chat/completions"),
            ),
            ("/llm/api_key_env", serde_json::json!("PAID_KEY")),
            ("/sources/0/kind", serde_json::json!("frappe_ssh")),
            ("/subscribe", serde_json::json!(true)),
        ] {
            let mut changed = value.clone();
            *changed.pointer_mut(path).expect("fixture path") = replacement;
            let changed: RuntimeConfig = serde_json::from_value(changed).expect("fixture");
            assert!(changed.validate_loopback().is_err());
        }
    }
}
