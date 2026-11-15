//! Owner-managed configuration. Saving never grants runtime or provider authority.
use crate::local::LocalError;
use async_trait::async_trait;
use kusi_domain::local::LocalUser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Plane,
    Frappe,
    Telegram,
}
impl ConnectionKind {
    pub fn parse(value: &str) -> Result<Self, LocalError> {
        match value {
            "plane" => Ok(Self::Plane),
            "frappe" => Ok(Self::Frappe),
            "telegram" => Ok(Self::Telegram),
            _ => Err(LocalError::Invalid),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Plane => "plane",
            Self::Frappe => "frappe",
            Self::Telegram => "telegram",
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialPatch {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub bot_token: Option<String>,
    pub webhook_secret: Option<String>,
}
impl CredentialPatch {
    /// Values exist only in the write path. Never derive Debug or return these.
    pub fn values(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("api_key", self.api_key.as_deref()),
            ("api_secret", self.api_secret.as_deref()),
            ("bot_token", self.bot_token.as_deref()),
            ("webhook_secret", self.webhook_secret.as_deref()),
        ]
        .into_iter()
        .filter_map(|(key, value)| {
            value
                .filter(|value| !value.is_empty())
                .map(|value| (key, value))
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionInput {
    pub label: String,
    pub base_url: Option<String>,
    pub workspace: Option<String>,
    pub bot_username: Option<String>,
    pub webhook_url: Option<String>,
    pub expected_revision: Option<i64>,
    #[serde(default)]
    pub credentials: CredentialPatch,
}

#[derive(Clone, Serialize)]
pub struct ConnectionSummary {
    pub kind: ConnectionKind,
    pub label: String,
    pub base_url: Option<String>,
    pub workspace: Option<String>,
    pub bot_username: Option<String>,
    pub webhook_url: Option<String>,
    pub revision: i64,
    pub credential_fields: Vec<String>,
    pub configured: bool,
    pub runtime_active: bool,
    pub verification: &'static str,
}

fn https_url(value: &str, origin_only: bool) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.query().is_none()
        && url.fragment().is_none()
        && matches!(url.host(), Some(url::Host::Domain(host)) if host.contains('.') && !host.ends_with(".localhost") && !host.ends_with(".local"))
        && (!origin_only || url.path() == "/")
        && value.len() <= 2048
}

impl ConnectionInput {
    pub fn validate(&self, kind: ConnectionKind) -> Result<(), LocalError> {
        let optional_valid = |value: &Option<String>, check: fn(&str) -> bool| {
            value
                .as_deref()
                .is_none_or(|value| value.is_empty() || check(value))
        };
        let slug = |value: &str| {
            value.len() <= 120
                && value
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        };
        let username = |value: &str| {
            (5..=32).contains(&value.len())
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        };
        if self.label.trim() != self.label
            || !(2..=120).contains(&self.label.len())
            || self.label.chars().any(char::is_control)
            || self.expected_revision.is_some_and(|revision| revision < 1)
        {
            return Err(LocalError::Invalid);
        }
        let valid = match kind {
            ConnectionKind::Plane => {
                self.base_url
                    .as_deref()
                    .is_some_and(|url| https_url(url, true))
                    && optional_valid(&self.workspace, slug)
                    && self.bot_username.is_none()
                    && self.webhook_url.is_none()
            }
            ConnectionKind::Frappe => {
                self.base_url
                    .as_deref()
                    .is_some_and(|url| https_url(url, true))
                    && self.workspace.is_none()
                    && self.bot_username.is_none()
                    && self.webhook_url.is_none()
            }
            ConnectionKind::Telegram => {
                self.base_url.is_none()
                    && self.workspace.is_none()
                    && optional_valid(&self.bot_username, username)
                    && self
                        .webhook_url
                        .as_deref()
                        .is_none_or(|url| url.is_empty() || https_url(url, false))
            }
        };
        if !valid {
            return Err(LocalError::Invalid);
        }
        for (name, value) in self.credentials.values() {
            if value.len() > 4096
                || !value.is_ascii()
                || value
                    .bytes()
                    .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
            {
                return Err(LocalError::Invalid);
            }
            let permitted = match kind {
                ConnectionKind::Plane => name == "api_key",
                ConnectionKind::Frappe => matches!(name, "api_key" | "api_secret"),
                ConnectionKind::Telegram => matches!(name, "bot_token" | "webhook_secret"),
            };
            if !permitted {
                return Err(LocalError::Invalid);
            }
            if name == "bot_token"
                && !value.split_once(':').is_some_and(|(id, secret)| {
                    !id.is_empty()
                        && id.len() <= 20
                        && id.bytes().all(|c| c.is_ascii_digit())
                        && (16..=256).contains(&secret.len())
                        && secret
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                })
            {
                return Err(LocalError::Invalid);
            }
            if name == "webhook_secret"
                && (!(16..=256).contains(&value.len())
                    || !value
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c)))
            {
                return Err(LocalError::Invalid);
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait ConnectionSettingsRepository: Send + Sync {
    async fn list_connection_settings(
        &self,
        actor: &LocalUser,
    ) -> Result<Vec<ConnectionSummary>, LocalError>;
    async fn save_connection_settings(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        input: &ConnectionInput,
    ) -> Result<ConnectionSummary, LocalError>;
    async fn clear_connection_credentials(
        &self,
        actor: &LocalUser,
        kind: ConnectionKind,
        revision: i64,
    ) -> Result<ConnectionSummary, LocalError>;
}
fn owner(actor: &LocalUser) -> Result<(), LocalError> {
    if actor.disabled || !actor.role.is_owner() {
        return Err(LocalError::Forbidden);
    }
    Ok(())
}
pub async fn list<R: ConnectionSettingsRepository>(
    repository: &R,
    actor: &LocalUser,
) -> Result<Vec<ConnectionSummary>, LocalError> {
    owner(actor)?;
    repository.list_connection_settings(actor).await
}
pub async fn save<R: ConnectionSettingsRepository>(
    repository: &R,
    actor: &LocalUser,
    kind: ConnectionKind,
    input: &ConnectionInput,
) -> Result<ConnectionSummary, LocalError> {
    owner(actor)?;
    input.validate(kind)?;
    repository
        .save_connection_settings(actor, kind, input)
        .await
}
pub async fn clear<R: ConnectionSettingsRepository>(
    repository: &R,
    actor: &LocalUser,
    kind: ConnectionKind,
    revision: i64,
) -> Result<ConnectionSummary, LocalError> {
    owner(actor)?;
    if revision < 1 {
        return Err(LocalError::Invalid);
    }
    repository
        .clear_connection_credentials(actor, kind, revision)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_destinations_and_credentials_for_another_adapter() {
        let mut input:ConnectionInput=serde_json::from_value(serde_json::json!({"label":"Plane","base_url":"https://plane.example.invalid","credentials":{"api_key":"synthetic-key"}})).expect("fixture");
        assert!(input.validate(ConnectionKind::Plane).is_ok());
        for url in [
            "http://plane.example.invalid",
            "https://127.0.0.1",
            "https://user:pass@plane.example.invalid",
            "https://plane.example.invalid?token=secret",
            "https://plane.example.invalid/api",
            "https://plane.example.invalid:8443",
        ] {
            input.base_url = Some(url.into());
            assert!(input.validate(ConnectionKind::Plane).is_err());
        }
        input.base_url = Some("https://plane.example.invalid".into());
        input.credentials.bot_token = Some("123:synthetic_private_token".into());
        assert!(input.validate(ConnectionKind::Plane).is_err());
        input.credentials.bot_token = None;
        input.credentials.api_key = Some("bad\nheader".into());
        assert!(input.validate(ConnectionKind::Plane).is_err());
    }
    #[test]
    fn telegram_uses_fixed_destination_and_validates_bot_and_webhook_secrets() {
        let mut input:ConnectionInput=serde_json::from_value(serde_json::json!({"label":"Telegram","bot_username":"synthetic_bot","credentials":{"bot_token":"123456:synthetic_private_token_12345"}})).expect("fixture");
        assert!(input.validate(ConnectionKind::Telegram).is_ok());
        input.base_url = Some("https://attacker.example.invalid".into());
        assert!(input.validate(ConnectionKind::Telegram).is_err());
        input.base_url = None;
        input.credentials.webhook_secret = Some("short".into());
        assert!(input.validate(ConnectionKind::Telegram).is_err());
    }
    struct UnreachableRepository;
    #[async_trait]
    impl ConnectionSettingsRepository for UnreachableRepository {
        async fn list_connection_settings(
            &self,
            _: &LocalUser,
        ) -> Result<Vec<ConnectionSummary>, LocalError> {
            panic!("unauthorized request reached repository")
        }
        async fn save_connection_settings(
            &self,
            _: &LocalUser,
            _: ConnectionKind,
            _: &ConnectionInput,
        ) -> Result<ConnectionSummary, LocalError> {
            panic!("unauthorized request reached repository")
        }
        async fn clear_connection_credentials(
            &self,
            _: &LocalUser,
            _: ConnectionKind,
            _: i64,
        ) -> Result<ConnectionSummary, LocalError> {
            panic!("unauthorized request reached repository")
        }
    }
    #[tokio::test]
    async fn unauthorized_actor_is_rejected_before_any_repository_or_secret_access() {
        use kusi_domain::local::LocalRole;
        let input: ConnectionInput = serde_json::from_value(
            serde_json::json!({"label":"Plane","base_url":"https://plane.example.invalid"}),
        )
        .expect("fixture");
        for (role, disabled) in [
            (LocalRole::Viewer, false),
            (LocalRole::Editor, false),
            (LocalRole::Approver, false),
            (LocalRole::Owner, true),
        ] {
            let actor = LocalUser {
                id: uuid::Uuid::new_v4(),
                organization_id: uuid::Uuid::new_v4(),
                organization: "synthetic".into(),
                username: "synthetic".into(),
                display_name: "Synthetic".into(),
                role,
                platform_admin: false,
                disabled,
            };
            assert!(matches!(
                list(&UnreachableRepository, &actor).await,
                Err(LocalError::Forbidden)
            ));
            assert!(matches!(
                save(
                    &UnreachableRepository,
                    &actor,
                    ConnectionKind::Plane,
                    &input
                )
                .await,
                Err(LocalError::Forbidden)
            ));
            assert!(matches!(
                clear(&UnreachableRepository, &actor, ConnectionKind::Plane, 1).await,
                Err(LocalError::Forbidden)
            ));
        }
    }
}
